import 'package:flutter_test/flutter_test.dart';
import 'package:twodrive_dav/file_manager.dart';

void main() {
  test(
    'native file manager receives a single unsplit Unicode directory argument',
    () async {
      for (final target in {
        'windows': 'explorer.exe',
        'linux': 'xdg-open',
      }.entries) {
        await openLocalDirectory(
          r'C:\generated folder\中文',
          platform: target.key,
          launch: (exe, args) async {
            expect(exe, target.value);
            expect(args, [r'C:\generated folder\中文']);
          },
        );
      }
      await expectLater(
        openLocalDirectory(
          'generated',
          platform: 'windows',
          launch: (_, _) async => throw StateError('launch failed'),
        ),
        throwsStateError,
      );
    },
  );
}
