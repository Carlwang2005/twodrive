import 'dart:io';

Future<void> openLocalDirectory(
  String path, {
  String? platform,
  Future<void> Function(String, List<String>)? launch,
}) async {
  final target = platform ?? Platform.operatingSystem;
  final executable = switch (target) {
    'windows' => 'explorer.exe',
    'linux' => 'xdg-open',
    'macos' => 'open',
    _ => throw UnsupportedError('Unsupported file manager platform'),
  };
  await (launch ??
      (exe, args) async {
        await Process.start(exe, args, mode: ProcessStartMode.detached);
      })(executable, [path]);
}
