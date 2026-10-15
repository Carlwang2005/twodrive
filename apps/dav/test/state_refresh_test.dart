import 'dart:async';
import 'package:flutter_test/flutter_test.dart';
import 'package:twodrive_dav/client.dart';

class DelayedClient extends StorageClient {
  final replies = <Completer<Json>>[];
  final calls = <String>[];
  @override
  Future<Json> request(String op, [Json args = const {}, int? requestId]) {
    calls.add(op);
    final reply = Completer<Json>();
    replies.add(reply);
    return reply.future;
  }
}

void main() {
  test('long operation coalesces a hundred status refreshes', () async {
    final c = DelayedClient();
    final refreshes = List.generate(100, (_) => c.refreshStatus());
    final sent = c.calls.length;
    for (final reply in c.replies) {
      reply.complete({'connected': false});
    }
    await Future.wait(refreshes);
    expect(sent, 1);
    await Future<void>.delayed(Duration.zero);
    final next = c.refreshStatus();
    c.replies.last.complete({'connected': false});
    await next;
    expect(c.calls.length, 2);
    c.dispose();
  });

  test('status notices a changed active connection and real account', () async {
    final c = DelayedClient()..selected = 'old';
    c.profiles = [
      {'id': 'old', 'name': 'Old label'},
      {'id': 'new', 'name': 'Custom label'},
    ];
    final refresh = c.refreshStatus();
    c.replies.single.complete({
      'connected': true,
      'profile': 'new',
      'account_name': 'Signed in person',
    });
    await refresh;
    expect(c.selected, 'new');
    expect(c.account, 'Signed in person');
    c.dispose();
  });

  test(
    'successful poll clears its own failure but preserves action errors',
    () async {
      final c = DelayedClient();
      final failed = c.refreshStatus();
      final check = expectLater(failed, throwsStateError);
      c.replies.last.completeError(StateError('request queue is full'));
      await check;
      expect(c.error, contains('request queue is full'));
      final recovered = c.refreshStatus();
      c.replies.last.complete({'connected': false});
      await recovered;
      expect(c.error, isNull);
      c.error = '上传失败，保留重试入口';
      final again = c.refreshStatus();
      c.replies.last.complete({'connected': false});
      await again;
      expect(c.error, '上传失败，保留重试入口');
      c.dispose();
    },
  );

  test(
    'external engine operation notifies busy and updates identity on completion',
    () async {
      final c = StorageClient();
      c.accept({
        'id': 900,
        'event': 'operation',
        'op': 'mount',
        'running': true,
      });
      expect(c.busy, isTrue);
      expect(c.activeId, 900);
      await c.pollNow();
      c.accept({
        'id': 900,
        'event': 'operation',
        'op': 'mount',
        'running': false,
        'configuration': {
          'profiles': [
            {'id': 'p', 'name': 'Custom name', 'provider_label': 'Example'},
          ],
          'selected': 'p',
        },
        'status': {
          'connected': true,
          'profile': 'p',
          'mounted': true,
          'account_name': 'Actual account',
        },
      });
      expect(c.busy, isFalse);
      expect(c.mounted, isTrue);
      expect(c.selected, 'p');
      expect(c.account, 'Actual account');
      expect(c.trayTitle, contains('Actual account'));
      await c.pollNow();
      c.dispose();
    },
  );

  test('poll and directory refresh have one in-flight cycle', () async {
    final c = DelayedClient()..available = true;
    c.selected = 'p';
    c.status = {'connected': true, 'profile': 'p'};
    final polls = List.generate(100, (_) => c.pollNow());
    await Future<void>.delayed(Duration.zero);
    expect(c.calls, ['status']);
    c.replies.single.complete({'connected': true, 'profile': 'p'});
    await Future<void>.delayed(Duration.zero);
    expect(c.calls, ['status', 'list']);
    final more = List.generate(100, (_) => c.pollNow());
    c.replies.last.complete({
      'entries': [
        {'name': 'remote change'},
      ],
    });
    await Future.wait([...polls, ...more]);
    expect(c.calls, ['status', 'list']);
    expect(c.entries.single['name'], 'remote change');
    final next = c.pollNow();
    await Future<void>.delayed(Duration.zero);
    c.replies.last.complete({'connected': true, 'profile': 'p'});
    await next;
    expect(c.calls, ['status', 'list', 'status']);
    c.dispose();
  });
}
