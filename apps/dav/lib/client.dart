import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/foundation.dart';

typedef Json = Map<String, dynamic>;

class Transfer {
  Transfer(this.id, this.operation, this.args);
  final int id;
  final String operation;
  final Json args;
  int done = 0;
  int total = 0;
  String state = '传输中';
  String? error;
  String get name => (args['path'] as String).split('/').last;
}

class StorageClient extends ChangeNotifier {
  Process? _process;
  final Map<int, Completer<Json>> _pending = {};
  final List<Transfer> transfers = [];
  List<Json> profiles = [];
  List<Json> providers = [];
  List<Json> entries = [];
  Json status = {};
  String? selected;
  String path = '/';
  String? error;
  String activity = '';
  bool busy = false;
  bool available = false;
  bool quitting = false;
  int _id = 0;
  int _browseGeneration = 0;
  int? activeId;
  Timer? _poll;
  bool get connected => status['connected'] == true;
  bool get syncError =>
      status['backup_failed'] == true || status['backup']?['state'] == 'error';
  bool get paused => status['paused'] == true;
  List<Json> get syncActivity => [
    ...((status['backup']?['active'] as List?) ?? []),
    ...((status['mount_activity']?['active'] as List?) ?? []),
  ].map((v) => Map<String, dynamic>.from(v as Map)).toList();
  bool get syncing =>
      busy ||
      syncActivity.isNotEmpty ||
      (status['backup']?['state'] == 'scanning' ||
          status['backup']?['state'] == 'starting') ||
      (status['cache']?['pending'] as num? ?? 0) > 0;
  Future<void> pauseToggle() => action(paused ? '恢复同步' : '暂停同步', () async {
    status = await request('pause', {'paused': !paused});
  });
  Future<void> signOut() => action('退出登录', () async {
    await request('sign_out', {'id': selected});
    entries = [];
    status = {};
    await loadProfiles();
    await refreshStatus();
  });
  bool get canMount => status['capabilities']?['mount'] == true;
  bool get mounted => status['mounted'] == true;
  Json? get profile => profiles.where((p) => p['id'] == selected).firstOrNull;
  bool capability(String key) =>
      connected && status['capabilities']?[key] == true;
  bool get canUpload =>
      capability('upload') && capability('conditional_upload');
  bool get canMkdir => capability('mkdir');
  bool get canRename => capability('rename');
  bool get canDelete => capability('delete');
  bool get canBackup => connected
      ? capability('backup')
      : profile?['capabilities']?['backup'] == true;
  bool get writable => canUpload || canMkdir || canRename || canDelete;
  String get providerLabel =>
      profile?['provider_label'] as String? ?? 'TwoDrive';
  String get accountLabel => profile?['account_label'] as String? ?? '账号';
  String get account =>
      profile?['account'] as String? ?? profile?['name'] as String? ?? '未登录';
  String get trayState => error != null || syncError
      ? 'error'
      : connected && proxyFallback
      ? 'warning'
      : paused
      ? 'paused'
      : syncing
      ? 'syncing'
      : connected
      ? 'connected'
      : 'offline';
  Json get target => {
    'profile': selected,
    'revision': profile?['revision'] ?? '',
  };
  bool get proxyFallback => connected && status['route_fallback'] == true;
  void requireProfile(Json args) {
    if (!connected ||
        (args['profile'] != null && args['profile'] != selected)) {
      throw StateError('文件操作所属连接已改变，请刷新后重试');
    }
    if (args['revision'] != null && args['revision'] != target['revision']) {
      throw StateError('连接配置已更新，请刷新后重新选择文件');
    }
  }

  Future<void> start(
    String engine,
    String state,
    String peer,
    String launcher,
  ) async {
    try {
      final p = await Process.start(engine, [
        '--state',
        state,
        '--peer-bin',
        peer,
        '--launcher',
        launcher,
      ]);
      _process = p;
      p.stdout.transform(utf8.decoder).transform(const LineSplitter()).listen((
        line,
      ) {
        try {
          accept(jsonDecode(line) as Json);
        } catch (_) {
          fail('引擎返回无效数据');
        }
      }, onError: (_) => fail('引擎通信中断'));
      // Keep stderr drained without exposing private configuration in the UI.
      p.stderr.drain<void>();
      unawaited(
        p.exitCode.then((code) {
          if (!quitting) {
            fail('本地引擎已退出（$code）。请重新启动客户端。');
          }
        }),
      );
      available = true;
      await loadProfiles();
      await refreshStatus();
      _poll = Timer.periodic(const Duration(seconds: 3), (_) {
        if (!busy && available && !quitting) {
          unawaited(
            refreshStatus().catchError((Object e) {
              error = '$e';
              notifyListeners();
            }),
          );
        }
      });
    } catch (e) {
      fail('无法启动本地引擎：$e');
    }
  }

  @visibleForTesting
  void accept(Json message) {
    final id = message['id'] as int;
    if (message['event'] == 'progress') {
      final task = transfers.where((t) => t.id == id).firstOrNull;
      if (task != null) {
        task.done = message['done'] as int;
        task.total = message['total'] as int;
        notifyListeners();
      }
      return;
    }
    final reply = _pending.remove(id);
    if (reply == null) {
      return;
    }
    if (message['ok'] == true) {
      reply.complete(Map<String, dynamic>.from(message['result'] as Map));
    } else {
      reply.completeError(StateError(message['error'] as String));
    }
  }

  @visibleForTesting
  void fail(String message) {
    available = false;
    status = {};
    entries = [];
    _browseGeneration++;
    error = message;
    for (final pending in _pending.values) {
      pending.completeError(StateError(message));
    }
    _pending.clear();
    notifyListeners();
  }

  Future<Json> request(String op, [Json args = const {}, int? requestId]) {
    if (!available || _process == null) {
      return Future.error(StateError('本地引擎未运行'));
    }
    final id = requestId ?? ++_id;
    final completer = Completer<Json>();
    _pending[id] = completer;
    _process!.stdin.writeln(jsonEncode({'id': id, 'op': op, 'args': args}));
    return completer.future;
  }

  Future<void> loadProfiles() async {
    final result = await request('profiles');
    profiles = (result['profiles'] as List).cast<Json>();
    providers = ((result['providers'] as List?) ?? []).cast<Json>();
    selected = connected
        ? status['profile'] as String?
        : result['selected'] as String?;
    notifyListeners();
  }

  Future<void> refreshStatus() async {
    status = await request('status');
    if (!connected) {
      entries = [];
    }
    if (status['gateway_failed'] == true) {
      error = '配对连接已断开，请重连。';
    }
    if (status['mount_failed'] == true) {
      error = '挂载进程已退出；请卸载后重试。缓存已保留。';
    }
    notifyListeners();
  }

  Future<void> action(String label, Future<void> Function() run) async {
    if (busy) {
      return;
    }
    busy = true;
    activity = label;
    error = null;
    notifyListeners();
    try {
      await run();
    } catch (e) {
      error = e.toString().replaceFirst('Bad state: ', '');
    } finally {
      busy = false;
      activity = '';
      activeId = null;
      notifyListeners();
    }
  }

  Future<void> connect() => action('正在连接服务器', () async {
    if (selected == null) {
      throw StateError('请先添加连接');
    }
    _browseGeneration++;
    entries = [];
    status = {};
    notifyListeners();
    activeId = ++_id;
    final result = await request('connect', {'id': selected}, activeId);
    path = '/';
    entries = (result['entries'] as List)
        .map((e) => {...e as Json, ...target})
        .toList();
    await refreshStatus();
  });
  Future<void> disconnect() => action('正在断开', () async {
    await request('disconnect');
    _browseGeneration++;
    entries = [];
    path = '/';
    await refreshStatus();
  });
  Future<void> browse(String target) =>
      action('读取目录', () => readDirectory(target));
  Future<void> readDirectory(String target) async {
    final generation = ++_browseGeneration;
    final identity = this.target;
    final result = await request('list', {'path': target, ...identity});
    if (generation != _browseGeneration || !available) {
      return;
    }
    path = target;
    entries = (result['entries'] as List)
        .map((e) => {...e as Json, ...identity})
        .toList();
    notifyListeners();
  }

  Future<void> mutate(String op, Json args) => action('正在更新文件', () async {
    requireProfile(args);
    await request(op, {...target, ...args});
    await readDirectory(path);
  });
  Future<void> transfer(String op, Json args) =>
      action(op == 'upload' ? '正在上传' : '正在下载', () async {
        requireProfile(args);
        args = {...target, ...args};
        final id = ++_id;
        activeId = id;
        final task = Transfer(id, op, args);
        transfers.insert(0, task);
        if (transfers.length > 100) {
          transfers.removeLast();
        }
        notifyListeners();
        try {
          await request(op, args, id);
          task.state = '已完成';
          task.done = task.total;
          if (op == 'upload') {
            await readDirectory(path);
          }
        } catch (e) {
          task.state = '未完成';
          task.error = '$e';
          rethrow;
        }
      });
  Future<void> cancel() async {
    final id = activeId;
    if (id != null) {
      activity = '正在取消，等待当前网络读写返回';
      notifyListeners();
      await request('cancel', {'id': id});
    }
  }

  Future<void> mountToggle() => action(mounted ? '正在卸载' : '正在挂载', () async {
    activeId = ++_id;
    await request(mounted ? 'unmount' : 'mount', {}, activeId);
    await refreshStatus();
  });
  Future<void> save(Json profile) => action('保存连接', () async {
    await request('save_profile', profile);
    await loadProfiles();
  });
  Future<void> remove(String id) => action('删除连接配置', () async {
    await request('remove_profile', {'id': id});
    await loadProfiles();
  });
  Future<void> autostart(bool enabled) => action('更新自启动设置', () async {
    status = await request('autostart', {'enabled': enabled});
  });
  Future<void> clearCache() => action('释放已同步缓存', () async {
    await request('clear_cache', {'id': selected});
    await refreshStatus();
  });
  Future<bool> shutdown() async {
    if (busy) {
      error = '请先完成或取消当前操作，再退出。';
      notifyListeners();
      return false;
    }
    try {
      if (available) {
        await request('quit');
      }
      quitting = true;
      _poll?.cancel();
      await _process?.stdin.close();
      return true;
    } catch (e) {
      error = '$e';
      notifyListeners();
      return false;
    }
  }

  @override
  void dispose() {
    _poll?.cancel();
    super.dispose();
  }
}
