import 'package:flutter/material.dart';
import 'client.dart';

/// Engine/OS evidence only. Flutter never opens the files or infers completion
/// from an app icon. This panel exists only for the opt-in generated fixture.
class NativeLocalFilesCard extends StatefulWidget {
  const NativeLocalFilesCard({super.key, required this.client});
  final StorageClient client;
  @override
  State<NativeLocalFilesCard> createState() => _NativeLocalFilesCardState();
}

class _NativeLocalFilesCardState extends State<NativeLocalFilesCard> {
  Json? report;
  bool busy = false;
  String? error;
  int generation = 0;
  @override
  void didUpdateWidget(covariant NativeLocalFilesCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.client, widget.client)) {
      generation++;
      report = null;
      error = null;
      busy = false;
    }
  }

  Future<void> run(String op, [String? id]) async {
    final requestGeneration = ++generation;
    final client = widget.client;
    bool current() =>
        mounted &&
        generation == requestGeneration &&
        identical(client, widget.client);
    setState(() {
      busy = true;
      error = null;
    });
    try {
      final next = await client.nativeItems(op, id);
      if (current()) setState(() => report = next);
    } catch (_) {
      if (current()) {
        setState(() {
          report = null;
          error = '操作未完成。文件保持保留，请重新读取状态；脏数据或打开的文件会阻止释放。';
        });
      }
    } finally {
      if (current()) setState(() => busy = false);
    }
  }

  @override
  Widget build(BuildContext context) => Card(
    child: Padding(
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'Windows 本机生成文件验证',
            style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
          ),
          const Text('仅内置生成文件。真实账户未接入；Explorer 图标与完整同步尚未验收。'),
          TextButton(
            onPressed: busy ? null : () => run('local_items'),
            child: const Text('读取原生文件状态'),
          ),
          if (busy) const LinearProgressIndicator(),
          if (error != null)
            Text(error!, style: const TextStyle(color: Colors.red)),
          if (report != null) ...[
            Text(
              'OS 读取回调：${report!['fetch_callbacks']}；失败：${report!['fetch_failures']}',
            ),
            for (final raw in report!['items'] as List)
              item(Map<String, dynamic>.from(raw as Map)),
          ],
        ],
      ),
    ),
  );

  Widget item(Json value) {
    const residence = {
      'online_only': '仅联机',
      'locally_available': '本地可用',
      'partially_available': '部分已下载',
    };
    const sync = {
      'in_sync': '当前世代已同步',
      'pending_sync': '待同步／需要恢复',
      'remote_version_known': '已知远端版本，尚无本地确认',
      'conflict': '冲突',
      'error': '错误',
    };
    final caps = widget.client.nativeItemCapabilities;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(value['name'] as String),
          Text(
            '${residence[value['residency']] ?? '未知驻留状态'} · ${value['pinned'] == true ? '固定保留' : '未固定'} · ${sync[value['sync']] ?? '未知同步状态'}',
          ),
          Text(
            '已下载：${value['downloaded'] == true ? '是' : '否'}；引擎缓存：${value['cache_resident'] == true ? '保留' : '无'}；世代 ${value['generation'] ?? '—'} / 已确认 ${value['acknowledged_generation'] ?? '—'}',
          ),
          Wrap(
            spacing: 12,
            children: [
              TextButton(
                onPressed: busy || caps['read'] != true
                    ? null
                    : () => run('local_read', value['id'] as String),
                child: const Text('通过系统读取验证'),
              ),
              TextButton(
                onPressed:
                    busy ||
                        caps['native_copy_dehydrate'] != true ||
                        value['downloaded'] != true ||
                        value['pinned'] == true ||
                        value['sync'] != 'in_sync'
                    ? null
                    : () => run('local_dehydrate', value['id'] as String),
                child: const Text('释放原生副本（保留引擎缓存）'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
