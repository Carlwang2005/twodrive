import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:twodrive_dav/client.dart';
import 'package:twodrive_dav/native_local_files.dart';

class NativeClient extends StorageClient {
  Completer<Json>? pending;
  final calls = <String>[];
  Json reply = {
    'schema_version': 1,
    'scope': 'generated_fixture_only',
    'fetch_callbacks': 0,
    'fetch_failures': 0,
    'items': [
      {
        'id': 'generated-0',
        'name': 'generated-中文.bin',
        'residency': 'online_only',
        'pinned': false,
        'sync': 'remote_version_known',
        'downloaded': false,
        'cache_resident': false,
      },
    ],
  };
  NativeClient() {
    status = {
      'local_files': {
        'item_status': {
          'schema_version': 1,
          'supported': true,
          'scope': 'generated_fixture_only',
          'read': true,
          'native_copy_dehydrate': true,
          'write': false,
        },
      },
    };
  }
  @override
  Future<Json> request(
    String op, [
    Json args = const {},
    int? requestId,
  ]) async {
    calls.add(op);
    expect(args['schema_version'], 1);
    return pending?.future ?? reply;
  }
}

void main() {
  Future<void> show(WidgetTester tester, NativeClient c) => tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: SingleChildScrollView(child: NativeLocalFilesCard(client: c)),
      ),
    ),
  );
  testWidgets(
    'native residency, cache and acknowledgement are rendered separately',
    (tester) async {
      final c = NativeClient();
      await show(tester, c);
      await tester.tap(find.text('读取原生文件状态'));
      await tester.pumpAndSettle();
      expect(find.textContaining('仅联机'), findsOneWidget);
      expect(find.textContaining('尚无本地确认'), findsOneWidget);
      final release = find.widgetWithText(TextButton, '释放原生副本（保留引擎缓存）');
      expect(tester.widget<TextButton>(release).onPressed, isNull);
      c.reply['items'] = [
        {
          'id': 'generated-0',
          'name': 'generated-中文.bin',
          'residency': 'locally_available',
          'pinned': false,
          'sync': 'in_sync',
          'downloaded': true,
          'cache_resident': true,
          'generation': 0,
          'acknowledged_generation': 0,
        },
      ];
      await tester.tap(find.text('通过系统读取验证'));
      await tester.pumpAndSettle();
      expect(find.textContaining('当前世代已同步'), findsOneWidget);
      expect(tester.widget<TextButton>(release).onPressed, isNotNull);
      c.reply['items'][0]['sync'] = 'pending_sync';
      await tester.tap(find.text('读取原生文件状态'));
      await tester.pumpAndSettle();
      expect(tester.widget<TextButton>(release).onPressed, isNull);
      await tester.pumpWidget(const SizedBox());
      c.dispose();
    },
  );
  testWidgets('old service result cannot cross a client change', (
    tester,
  ) async {
    final old = NativeClient()..pending = Completer<Json>();
    final next = NativeClient();
    await show(tester, old);
    await tester.tap(find.text('读取原生文件状态'));
    await tester.pump();
    await show(tester, next);
    old.pending!.complete(old.reply);
    await tester.pumpAndSettle();
    expect(find.text('generated-中文.bin'), findsNothing);
    await tester.pumpWidget(const SizedBox());
    old.dispose();
    next.dispose();
  });
  test('unknown schema or scope refuses operations', () async {
    final c = NativeClient();
    c.status['local_files']['item_status']['schema_version'] = 2;
    await expectLater(
      c.nativeItems('local_read', 'generated-0'),
      throwsStateError,
    );
    expect(c.calls, isEmpty);
    c.status['local_files']['item_status']['schema_version'] = 1;
    c.status['local_files']['item_status']['scope'] = 'real_account';
    expect(c.canInspectNativeItems, isFalse);
    c.dispose();
  });
}
