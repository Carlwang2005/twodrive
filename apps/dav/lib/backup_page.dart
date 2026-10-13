import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'client.dart';

class BackupPage extends StatefulWidget {
  const BackupPage({super.key, required this.client});
  final StorageClient client;
  @override
  State<BackupPage> createState() => _BackupPageState();
}

class _BackupPageState extends State<BackupPage> {
  List<Json> folders = [];
  bool loading = true;
  String? failure;
  StorageClient get c => widget.client;
  @override
  void initState() {
    super.initState();
    load();
  }

  Future<void> load() async {
    try {
      if (c.selected != null) {
        final result = await c.request('backups', {'id': c.selected});
        folders = (result['folders'] as List).cast<Json>();
      }
    } catch (e) {
      failure = '$e';
    }
    if (mounted) {
      setState(() => loading = false);
    }
  }

  Future<void> save(List<Json> next) async {
    final id = c.selected;
    await c.action('保存备份文件夹', () async {
      await c.request('save_backups', {'id': id, 'folders': next});
      if (mounted) {
        setState(() => folders = next);
      }
      await c.refreshStatus();
    });
  }

  Future<void> add() async {
    final id = c.selected;
    final local = await getDirectoryPath(confirmButtonText: '选择备份文件夹');
    if (local == null || !mounted || id != c.selected) {
      return;
    }
    final name = local.split('/').last;
    final controller = TextEditingController(text: '/Backups/$name');
    final remote = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('选择备份目标'),
        content: SizedBox(
          width: 440,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(local),
              const SizedBox(height: 16),
              TextField(
                controller: controller,
                decoration: const InputDecoration(labelText: '服务器中的备份目录'),
              ),
              const SizedBox(height: 12),
              const Text(
                '自动上传新建和修改的文件。停止备份或删除本地文件，不会删除服务器副本。',
                style: TextStyle(fontSize: 12, color: Colors.blueGrey),
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, controller.text.trim()),
            child: const Text('开始备份'),
          ),
        ],
      ),
    );
    Future<void>.delayed(const Duration(seconds: 1), controller.dispose);
    if (remote != null && id == c.selected) {
      await save([
        ...folders,
        {'local': local, 'remote': remote, 'enabled': true},
      ]);
    }
  }

  @override
  Widget build(BuildContext context) => ListView(
    children: [
      const Text(
        '备份文件夹',
        style: TextStyle(fontSize: 22, fontWeight: FontWeight.w600),
      ),
      const SizedBox(height: 10),
      const Text('选择需要自动备份的本地文件夹。原文件保留在本机；服务器上的已有版本受到冲突保护。'),
      const SizedBox(height: 18),
      if (failure != null)
        Text(failure!, style: const TextStyle(color: Colors.deepOrange)),
      if (loading) const LinearProgressIndicator(),
      for (var i = 0; i < folders.length; i++)
        Card(
          child: ListTile(
            leading: const Icon(Icons.folder_outlined),
            title: Text((folders[i]['local'] as String).split('/').last),
            subtitle: Text('${folders[i]['local']} → ${folders[i]['remote']}'),
            trailing: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Switch(
                  value: folders[i]['enabled'] == true,
                  onChanged: c.busy
                      ? null
                      : (value) => save([
                          for (var j = 0; j < folders.length; j++)
                            if (i == j)
                              {...folders[j], 'enabled': value}
                            else
                              folders[j],
                        ]),
                ),
                IconButton(
                  tooltip: '停止备份，保留两端文件',
                  onPressed: c.busy
                      ? null
                      : () => save([
                          for (var j = 0; j < folders.length; j++)
                            if (i != j) folders[j],
                        ]),
                  icon: const Icon(Icons.remove_circle_outline),
                ),
              ],
            ),
          ),
        ),
      const SizedBox(height: 12),
      Align(
        alignment: Alignment.centerLeft,
        child: FilledButton.icon(
          onPressed: c.selected != null && c.canBackup && !c.busy ? add : null,
          icon: const Icon(Icons.add),
          label: const Text('选择备份文件夹'),
        ),
      ),
      if (!c.canBackup)
        const Padding(
          padding: EdgeInsets.only(top: 12),
          child: Text('请选择支持安全上传和备份的连接。'),
        ),
      const SizedBox(height: 24),
      const Text(
        '符号链接不会上传。暂停同步会暂停文件夹备份与挂载的新网络请求。',
        style: TextStyle(color: Colors.blueGrey, fontSize: 12),
      ),
    ],
  );
}
