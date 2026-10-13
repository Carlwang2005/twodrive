import 'dart:io';
import 'package:flutter/material.dart';
import 'client.dart';
import 'platform_paths.dart';

class ProfileDialog extends StatefulWidget {
  const ProfileDialog({super.key, this.original});
  final Json? original;
  @override
  State<ProfileDialog> createState() => _ProfileDialogState();
}

class _ProfileDialogState extends State<ProfileDialog> {
  final form = GlobalKey<FormState>();
  late final Map<String, TextEditingController> fields;
  late String kind;
  late bool writable;
  late String route;
  @override
  void initState() {
    super.initState();
    final p = widget.original ?? {};
    route = p['network_route'] as String? ?? 'auto';
    kind = p['kind'] as String? ?? 'dav';
    writable = p['writable'] == true;
    fields = {
      for (final key in [
        'name',
        'url',
        'username',
        'password',
        'peer_state',
        'mount_path',
      ])
        key: TextEditingController(text: p[key] as String? ?? ''),
    };
    if (fields['mount_path']!.text.isEmpty) {
      fields['mount_path']!.text = '$userHome/TwoDrive Development';
    }
  }

  @override
  void dispose() {
    for (final field in fields.values) {
      field.dispose();
    }
    super.dispose();
  }

  Widget field(
    String key,
    String label, {
    bool secret = false,
    bool required = false,
  }) => Padding(
    padding: const EdgeInsets.only(bottom: 14),
    child: TextFormField(
      controller: fields[key],
      obscureText: secret,
      decoration: InputDecoration(labelText: label),
      validator: (text) =>
          required && (text == null || text.trim().isEmpty) ? '请填写此项' : null,
    ),
  );
  @override
  Widget build(BuildContext context) => AlertDialog(
    title: Text(widget.original == null ? '添加连接' : '编辑连接'),
    content: SizedBox(
      width: 490,
      child: SingleChildScrollView(
        child: Form(
          key: form,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              field('name', '连接名称', required: true),
              DropdownButtonFormField<String>(
                initialValue: kind,
                decoration: const InputDecoration(labelText: '连接方式'),
                items: const [
                  DropdownMenuItem(value: 'dav', child: Text('HTTPS WebDAV')),
                  DropdownMenuItem(value: 'peer', child: Text('TwoDrive 已有配对')),
                ],
                onChanged: (value) => setState(() => kind = value!),
              ),
              const SizedBox(height: 14),
              if (kind == 'dav') ...[
                field('url', 'WebDAV 根地址 · https://…', required: true),
                field('username', '用户名'),
                field(
                  'password',
                  widget.original?['password_saved'] == true
                      ? '密码 · 留空保留已保存密码'
                      : '密码',
                  secret: true,
                ),
              ] else
                field('peer_state', '本机配对状态目录 · 绝对路径', required: true),
              DropdownButtonFormField<String>(
                initialValue: route,
                decoration: const InputDecoration(labelText: '网络连接方式'),
                items: const [
                  DropdownMenuItem(
                    value: 'auto',
                    child: Text('优先直连，失败时提示并允许代理兜底'),
                  ),
                  DropdownMenuItem(value: 'direct', child: Text('仅直连（绕过代理）')),
                  DropdownMenuItem(value: 'system', child: Text('使用系统路由和代理')),
                ],
                onChanged: (value) => setState(() => route = value!),
              ),
              const SizedBox(height: 14),
              if (Platform.isLinux)
                field('mount_path', '挂载目录 · 使用空目录', required: true),
              SwitchListTile(
                contentPadding: EdgeInsets.zero,
                title: const Text('允许修改服务器文件'),
                subtitle: const Text('包含上传、备份、重命名和删除。'),
                value: writable,
                onChanged: (value) => setState(() => writable = value),
              ),
            ],
          ),
        ),
      ),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.pop(context),
        child: const Text('取消'),
      ),
      FilledButton(
        onPressed: () {
          if (form.currentState!.validate()) {
            Navigator.pop(context, {
              'id': widget.original?['id'] ?? '',
              'kind': kind,
              'network_route': route,
              'writable': writable,
              for (final entry in fields.entries)
                entry.key: entry.key == 'password'
                    ? entry.value.text
                    : entry.value.text.trim(),
            });
          }
        },
        child: const Text('保存'),
      ),
    ],
  );
}
