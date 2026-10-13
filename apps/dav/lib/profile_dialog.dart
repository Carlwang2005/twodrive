import 'dart:io';
import 'package:flutter/material.dart';
import 'client.dart';
import 'platform_paths.dart';

class ProfileDialog extends StatefulWidget {
  const ProfileDialog({super.key, this.original, required this.providers});
  final Json? original;
  final List<Json> providers;
  @override
  State<ProfileDialog> createState() => _ProfileDialogState();
}

class _ProfileDialogState extends State<ProfileDialog> {
  final form = GlobalKey<FormState>();
  late final Map<String, TextEditingController> fields;
  late String? kind;
  late bool writable;
  late String route;
  Json? get provider =>
      widget.providers.where((p) => p['id'] == kind).firstOrNull;
  List<Json> get providerFields =>
      ((provider?['fields'] as List?) ?? []).cast<Json>();
  @override
  void initState() {
    super.initState();
    final p = widget.original ?? {};
    route = p['network_route'] as String? ?? 'auto';
    final originalKind = p['kind'] as String?;
    kind = widget.providers.any((d) => d['id'] == originalKind)
        ? originalKind
        : widget.providers.firstOrNull?['id'] as String?;
    writable = p['writable'] == true;
    final keys = <String>{
      'name',
      'mount_path',
      for (final d in widget.providers)
        for (final f in d['fields'] as List) f['key'] as String,
    };
    fields = {
      for (final key in keys)
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
                decoration: const InputDecoration(labelText: '存储提供者'),
                items: [
                  for (final d in widget.providers)
                    DropdownMenuItem(
                      value: d['id'] as String,
                      child: Text(d['label'] as String),
                    ),
                ],
                onChanged: (value) => setState(() {
                  for (final f in providerFields.where(
                    (f) => f['secret'] == true,
                  )) {
                    fields[f['key']]!.clear();
                  }
                  kind = value;
                }),
              ),
              const SizedBox(height: 14),
              if (provider == null) const Text('引擎没有可用的存储提供者，请检查引擎版本。'),
              for (final f in providerFields)
                field(
                  f['key'] as String,
                  f['secret'] == true &&
                          widget.original?['kind'] == kind &&
                          widget.original?['${f['key']}_saved'] == true
                      ? '${f['label']} · 留空保留已保存值'
                      : f['label'] as String,
                  secret: f['secret'] == true,
                  required: f['required'] == true,
                ),
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
                subtitle: const Text('可用操作取决于提供者支持的能力。'),
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
        onPressed: provider == null
            ? null
            : () {
                if (form.currentState!.validate()) {
                  Navigator.pop(context, {
                    'id': widget.original?['id'] ?? '',
                    'kind': kind,
                    'network_route': route,
                    'writable': writable,
                    'name': fields['name']!.text.trim(),
                    'mount_path': fields['mount_path']!.text.trim(),
                    for (final f in providerFields)
                      f['key'] as String: f['secret'] == true
                          ? fields[f['key']]!.text
                          : fields[f['key']]!.text.trim(),
                  });
                }
              },
        child: const Text('保存'),
      ),
    ],
  );
}
