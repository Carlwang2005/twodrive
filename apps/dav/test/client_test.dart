import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:twodrive_dav/client.dart';
import 'package:twodrive_dav/main.dart';
import 'package:twodrive_dav/tray_panel.dart';
import 'package:twodrive_dav/profile_dialog.dart';

class FakeClient extends DavClient {
  Completer<Json>? listing;
  final calls = <String>[];
  @override
  Future<Json> request(
    String op, [
    Json args = const {},
    int? requestId,
  ]) async {
    calls.add(op);
    if (op == 'list') {
      return listing!.future;
    }
    if (op == 'backups') {
      return {'folders': []};
    }
    if (op == 'pause') {
      return {...status, 'paused': args['paused']};
    }
    if (op == 'autostart') {
      return {'autostart': args['enabled']};
    }
    return {};
  }
}

void main() {
  testWidgets('tray shows actual sync state, progress and pause control', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(390, 570);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final c = FakeClient()..available = true;
    c.selected = 'p';
    c.profiles = [
      {'id': 'p', 'kind': 'dav', 'name': 'Example', 'username': 'demo'},
    ];
    c.status = {
      'connected': true,
      'backup': {'folders': 1, 'completed_scan': false, 'active': []},
    };
    Future<void> show() => tester.pumpWidget(
      MaterialApp(
        home: TrayPanel(
          client: c,
          onManage: ([int page = 0]) async {},
          onOpenFolder: () async {},
          onQuit: () async {},
        ),
      ),
    );
    await show();
    expect(find.text('WebDAV'), findsOneWidget);
    expect(find.text('账号 · demo'), findsOneWidget);
    expect(find.text('已备份和同步'), findsNothing);
    c.status['backup'] = {'folders': 1, 'completed_scan': true, 'active': []};
    await show();
    await tester.pump();
    expect(find.text('已备份和同步'), findsOneWidget);
    c.transfers.add(
      Transfer(1, 'download', {'path': '/example.txt'})
        ..done = 25
        ..total = 100,
    );
    await show();
    await tester.pump();
    expect(find.text('25%'), findsOneWidget);
    await tester.tap(find.byType(PopupMenuButton<String>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('暂停同步'));
    await tester.pumpAndSettle();
    expect(c.calls, contains('pause'));
    expect(c.paused, isTrue);
    await show();
    await tester.pump();
    expect(find.text('同步已暂停'), findsOneWidget);
    await tester.pumpWidget(const SizedBox());
    c.dispose();
  });
  testWidgets('network policies are user-selectable and saved', (tester) async {
    tester.view.physicalSize = const Size(1000, 1100);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    Json? saved;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => TextButton(
            onPressed: () async {
              saved = await showDialog<Json>(
                context: context,
                builder: (_) => const ProfileDialog(
                  original: {
                    'id': 'p',
                    'name': 'Sample',
                    'kind': 'dav',
                    'url': 'https://example.invalid/',
                    'mount_path': '/tmp/mount',
                    'network_route': 'auto',
                  },
                ),
              );
            },
            child: const Text('open'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('优先直连，失败时提示并允许代理兜底'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('仅直连（绕过代理）').last);
    await tester.pumpAndSettle();
    await tester.tap(find.text('保存'));
    await tester.pumpAndSettle();
    expect(saved?['network_route'], 'direct');
    await tester.pump(const Duration(seconds: 1));
  });

  test(
    'a failed transfer cannot be retried against a different connection',
    () async {
      final c = FakeClient()..available = true;
      c.selected = 'server-b';
      c.status = {'connected': true, 'profile': 'server-b'};
      await c.transfer('upload', {
        'path': '/old.txt',
        'source': '/tmp/old.txt',
        'profile': 'server-a',
      });
      expect(c.calls, isNot(contains('upload')));
      expect(c.error, contains('连接'));
      c.dispose();
    },
  );
  test('engine loss invalidates a directory request still in flight', () async {
    final c = FakeClient()..available = true;
    c.status = {'connected': true};
    c.listing = Completer<Json>();
    final result = c.browse('/old');
    c.fail('lost');
    c.listing!.complete({
      'entries': [
        {'name': 'stale'},
      ],
    });
    await result;
    expect(c.entries, isEmpty);
    expect(c.connected, isFalse);
    expect(c.available, isFalse);
    c.dispose();
  });
  test('progress is associated with the matching transfer', () {
    final c = DavClient();
    c.transfers.addAll([
      Transfer(1, 'upload', {'path': '/a'}),
      Transfer(2, 'download', {'path': '/b'}),
    ]);
    c.accept({'id': 2, 'event': 'progress', 'done': 128, 'total': 256});
    expect(c.transfers[0].done, 0);
    expect(c.transfers[1].done, 128);
    c.dispose();
  });
  testWidgets('offline state exposes connections but no upload control', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1200, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final c = FakeClient()..available = true;
    await tester.pumpWidget(DavApp(client: c));
    expect(find.text('连接你的文件'), findsOneWidget);
    expect(find.text('上传'), findsNothing);
    await tester.tap(find.text('设置'));
    await tester.pumpAndSettle();
    final toggle = tester.widget<SwitchListTile>(find.byType(SwitchListTile));
    expect(toggle.value, isFalse);
    await tester.tap(find.byType(SwitchListTile));
    await tester.pumpAndSettle();
    expect(c.calls, contains('autostart'));
    expect(c.status['autostart'], isTrue);
    await tester.pumpWidget(const SizedBox());
    c.dispose();
  });
  testWidgets('readonly connection cannot expose mutations', (tester) async {
    tester.view.physicalSize = const Size(1200, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final c = FakeClient()..available = true;
    c.selected = 'p';
    c.profiles = [
      {'id': 'p', 'name': 'Read only', 'writable': false},
    ];
    c.status = {'connected': true};
    c.entries = [
      {
        'name': 'sample.txt',
        'path': '/sample.txt',
        'directory': false,
        'size': 123,
        'etag': 'version',
      },
    ];
    await tester.pumpWidget(DavApp(client: c));
    expect(find.text('只读'), findsOneWidget);
    expect(find.text('上传'), findsNothing);
    await tester.tap(find.byType(PopupMenuButton<String>));
    await tester.pumpAndSettle();
    expect(find.text('下载'), findsOneWidget);
    expect(find.text('删除'), findsNothing);
    expect(find.text('替换内容'), findsNothing);
    await tester.pumpWidget(const SizedBox());
    c.dispose();
  });
}
