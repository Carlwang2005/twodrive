import 'dart:io';
import 'package:flutter/material.dart';
import 'package:file_selector/file_selector.dart';
import 'client.dart';

class SharingPage extends StatefulWidget {
  const SharingPage({super.key, required this.client});
  final StorageClient client;
  @override
  State<SharingPage> createState() => _SharingPageState();
}

class _SharingPageState extends State<SharingPage> {
  StorageClient get c => widget.client;
  final form = GlobalKey<FormState>();
  final name = TextEditingController();
  final root = TextEditingController();
  bool writable = false;
  bool discoverable = true;
  String network = 'auto';
  bool loaded = false;
  String? exported;
  void load() {
    final settings = c.sharing['settings'];
    if (loaded || settings is! Map) return;
    loaded = true;
    name.text = (settings['name'] as String?)?.isNotEmpty == true
        ? settings['name'] as String
        : Platform.localHostname;
    root.text = settings['root'] as String? ?? '';
    writable = settings['writable'] == true;
    if (root.text.isNotEmpty) discoverable = settings['discoverable'] == true;
    final saved = settings['network'];
    if (['auto', 'direct', 'system', 'lan'].contains(saved)) {
      network = saved as String;
    }
  }

  @override
  void initState() {
    super.initState();
    load();
  }

  @override
  void didUpdateWidget(covariant SharingPage oldWidget) {
    super.didUpdateWidget(oldWidget);
    load();
  }

  @override
  void dispose() {
    name.dispose();
    root.dispose();
    super.dispose();
  }

  Future<void> export() async {
    final destination = await getSaveLocation(
      suggestedName: 'TwoDrive-连接.json',
      acceptedTypeGroups: const [
        XTypeGroup(label: 'TwoDrive 连接文件', extensions: ['json']),
      ],
    );
    if (!mounted || destination == null) return;
    final exists = await File(destination.path).exists();
    if (!mounted) return;
    if (exists) {
      final replace = await showDialog<bool>(
        context: context,
        builder: (context) => AlertDialog(
          title: const Text('替换现有连接文件？'),
          content: const Text('目标文件会被新的配对邀请替换。'),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: const Text('取消'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(context, true),
              child: const Text('替换'),
            ),
          ],
        ),
      );
      if (replace != true) return;
    }
    final result = await c.shareAction('share_export', {
      'output': destination.path,
      'replace': exists,
    });
    if (mounted && result != null) setState(() => exported = destination.path);
  }

  @override
  Widget build(BuildContext context) {
    final running = c.hosting;
    final available = c.available && c.sharing['supported'] == true && !c.busy;
    final editable = available && !running;
    final peers = ((c.sharing['peers'] as List?) ?? []).cast<String>();
    return Form(
      key: form,
      child: ListView(
        children: [
          Card(
            color: Colors.white,
            child: Padding(
              padding: const EdgeInsets.all(20),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(
                        running ? Icons.wifi_tethering : Icons.share_outlined,
                        color: running ? Colors.teal : Colors.blueGrey,
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: Text(
                          running ? '正在共享此电脑的文件夹' : '共享此电脑的文件夹',
                          style: const TextStyle(
                            fontSize: 20,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 10),
                  const Text('选择一个文件夹，启动共享后导出连接文件，发给另一台电脑即可连接。'),
                  const SizedBox(height: 20),
                  TextFormField(
                    controller: name,
                    enabled: editable,
                    decoration: const InputDecoration(labelText: '共享名称'),
                    validator: (value) => value == null || value.trim().isEmpty
                        ? '请填写共享名称'
                        : null,
                  ),
                  const SizedBox(height: 16),
                  TextFormField(
                    controller: root,
                    readOnly: true,
                    decoration: InputDecoration(
                      labelText: '共享文件夹',
                      suffixIcon: IconButton(
                        tooltip: '选择共享文件夹',
                        onPressed: editable
                            ? () async {
                                final folder = await getDirectoryPath();
                                if (mounted && folder != null) {
                                  setState(() => root.text = folder);
                                }
                              }
                            : null,
                        icon: const Icon(Icons.folder_open),
                      ),
                    ),
                    validator: (value) =>
                        value == null || value.isEmpty ? '请选择共享文件夹' : null,
                  ),
                  SwitchListTile(
                    contentPadding: EdgeInsets.zero,
                    title: const Text('允许对方修改文件'),
                    subtitle: const Text('默认只读。开启后允许上传、移动和删除共享文件夹中的文件。'),
                    value: writable,
                    onChanged: editable
                        ? (value) => setState(() => writable = value)
                        : null,
                  ),
                  ExpansionTile(
                    tilePadding: EdgeInsets.zero,
                    title: const Text('共享选项'),
                    children: [
                      SwitchListTile(
                        contentPadding: EdgeInsets.zero,
                        title: const Text('允许局域网发现'),
                        subtitle: const Text('对方仍需导入连接文件才能访问。'),
                        value: discoverable,
                        onChanged: editable
                            ? (value) => setState(() => discoverable = value)
                            : null,
                      ),
                      DropdownButtonFormField<String>(
                        initialValue: network,
                        decoration: const InputDecoration(labelText: '网络连接方式'),
                        items: const [
                          DropdownMenuItem(
                            value: 'auto',
                            child: Text('自动选择，优先直连'),
                          ),
                          DropdownMenuItem(
                            value: 'direct',
                            child: Text('仅使用物理网络'),
                          ),
                          DropdownMenuItem(
                            value: 'system',
                            child: Text('使用系统路由和代理'),
                          ),
                          DropdownMenuItem(
                            value: 'lan',
                            child: Text('仅局域网（不使用公共中继）'),
                          ),
                        ],
                        onChanged: editable
                            ? (value) => setState(() => network = value!)
                            : null,
                      ),
                      const SizedBox(height: 12),
                    ],
                  ),
                  const SizedBox(height: 16),
                  Wrap(
                    spacing: 12,
                    runSpacing: 12,
                    children: [
                      if (!running)
                        FilledButton.icon(
                          onPressed: available
                              ? () async {
                                  if (!form.currentState!.validate()) return;
                                  setState(() => exported = null);
                                  await c.shareAction('share_start', {
                                    'name': name.text.trim(),
                                    'root': root.text,
                                    'writable': writable,
                                    'discoverable': discoverable,
                                    'network': network,
                                  });
                                }
                              : null,
                          icon: const Icon(Icons.play_arrow),
                          label: const Text('启动共享'),
                        ),
                      if (running)
                        FilledButton.icon(
                          onPressed: available ? export : null,
                          icon: const Icon(Icons.ios_share),
                          label: const Text('导出连接文件'),
                        ),
                      if (running)
                        OutlinedButton.icon(
                          onPressed: available
                              ? () => c.shareAction('share_stop', {})
                              : null,
                          icon: const Icon(Icons.stop),
                          label: const Text('停止共享'),
                        ),
                    ],
                  ),
                  const SizedBox(height: 16),
                  const Text(
                    '连接文件包含访问邀请，十分钟内有效且仅能首次使用一次。对方连接成功后会记住设备；重新导出会替换尚未使用的旧邀请。',
                  ),
                  const SizedBox(height: 8),
                  const Text(
                    '关闭窗口后共享继续在托盘运行；退出应用将停止共享。修改文件夹或读写权限后，会建立新的共享身份，需重新导出连接文件。',
                  ),
                  if (!c.available || c.sharing['supported'] != true)
                    const Padding(
                      padding: EdgeInsets.only(top: 12),
                      child: Text('请等待支持共享功能的本地引擎启动。'),
                    ),
                  if (c.sharing['error'] is String)
                    Padding(
                      padding: const EdgeInsets.only(top: 12),
                      child: Text(c.sharing['error'] as String),
                    ),
                  if (exported != null)
                    Padding(
                      padding: const EdgeInsets.only(top: 12),
                      child: SelectableText('已导出：$exported'),
                    ),
                ],
              ),
            ),
          ),
          if (peers.isNotEmpty)
            Card(
              color: Colors.white,
              child: Column(
                children: [
                  const ListTile(
                    title: Text('已授权设备'),
                    subtitle: Text('撤销后，该设备的新文件请求会被拒绝。'),
                  ),
                  for (final peer in peers)
                    ListTile(
                      leading: const Icon(Icons.devices),
                      title: Text('${peer.substring(0, 16)}…'),
                      trailing: TextButton(
                        onPressed: available
                            ? () async {
                                final confirmed = await showDialog<bool>(
                                  context: context,
                                  builder: (context) => AlertDialog(
                                    title: const Text('撤销这台设备的访问？'),
                                    content: const Text('需要再次导入新的连接文件，才能重新授权。'),
                                    actions: [
                                      TextButton(
                                        onPressed: () =>
                                            Navigator.pop(context, false),
                                        child: const Text('取消'),
                                      ),
                                      FilledButton(
                                        onPressed: () =>
                                            Navigator.pop(context, true),
                                        child: const Text('撤销'),
                                      ),
                                    ],
                                  ),
                                );
                                if (confirmed == true) {
                                  await c.shareAction('share_revoke', {
                                    'device': peer,
                                  });
                                }
                              }
                            : null,
                        child: const Text('撤销访问'),
                      ),
                    ),
                ],
              ),
            ),
        ],
      ),
    );
  }
}
