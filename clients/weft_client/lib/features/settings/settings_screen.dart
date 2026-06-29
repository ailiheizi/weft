import 'dart:io' show Directory, Platform, Process;
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';
import 'package:url_launcher/url_launcher.dart';
import '../../core/providers/connection_provider.dart';
import '../../core/providers/catalog_service.dart';
import '../../core/providers/data_providers.dart';
import '../../core/providers/preferences_provider.dart';
import '../../shared/widgets/app_shell.dart' show PulseDot;

class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final conn = ref.watch(connectionProvider);
    final apps = ref.watch(appsProvider);
    final packages = ref.watch(packagesProvider);
    final providers = ref.watch(providersProvider);
    final prefs = ref.watch(preferencesProvider);
    final prefsNotifier = ref.read(preferencesProvider.notifier);

    final (statusColor, statusText) = switch (conn.status) {
      CoreConnectionStatus.connected => (
          const Color(0xFF4CB782),
          'Connected'
        ),
      CoreConnectionStatus.connecting => (
          const Color(0xFFF2C94C),
          'Connecting…'
        ),
      CoreConnectionStatus.offline => (
          const Color(0xFFEB5757),
          'Offline'
        ),
    };

    int? countOf(AsyncValue v) => v.asData?.value.length as int?;

    return Scaffold(
      backgroundColor: Colors.transparent,
      body: ListView(
        padding: const EdgeInsets.all(24),
        children: [
          Text('Settings',
              style: theme.textTheme.headlineSmall
                  ?.copyWith(fontWeight: FontWeight.w600)),
          const SizedBox(height: 24),
          _Section(
            title: 'Connection',
            children: [
              // 实时状态行。
              Padding(
                padding: const EdgeInsets.symmetric(
                    horizontal: 16, vertical: 11),
                child: Row(
                  children: [
                    PulseDot(
                        color: statusColor,
                        pulsing:
                            conn.status == CoreConnectionStatus.connecting),
                    const SizedBox(width: 10),
                    Text('weft-core',
                        style: theme.textTheme.bodyMedium),
                    const Spacer(),
                    Text(statusText,
                        style: theme.textTheme.bodyMedium
                            ?.copyWith(color: statusColor)),
                  ],
                ),
              ),
              const Divider(height: 1),
              _CoreUrlTile(
                url: prefs.coreBaseUrl,
                onSave: (value) => prefsNotifier.setCoreBaseUrl(value),
              ),
              const Divider(height: 1),
              // 计数。
              Padding(
                padding: const EdgeInsets.symmetric(
                    horizontal: 16, vertical: 11),
                child: Row(
                  children: [
                    _CountChip(label: 'Apps', count: countOf(apps)),
                    _CountChip(
                        label: 'Packages', count: countOf(packages)),
                    _CountChip(
                        label: 'Providers', count: countOf(providers)),
                  ],
                ),
              ),
              const Divider(height: 1),
              _SettingTile(
                icon: Icons.refresh,
                title: 'Run health check',
                subtitle: '重新检测 weft-core 连接',
                onTap: () {
                  ref.read(connectionProvider.notifier).retry();
                  ref.invalidate(appsProvider);
                  ref.invalidate(packagesProvider);
                  ref.invalidate(providersProvider);
                },
              ),
            ],
          ),
          const SizedBox(height: 16),
          _Section(
            title: 'Preferences',
            children: [
              ListTile(
                leading: const Icon(Icons.brightness_6_outlined, size: 18),
                title: const Text('主题', style: TextStyle(fontSize: 14)),
                subtitle: const Text('外观:暗色 / 亮色 / 跟随系统',
                    style: TextStyle(fontSize: 12)),
                dense: true,
                trailing: SegmentedButton<ThemeMode>(
                  style: const ButtonStyle(
                    visualDensity: VisualDensity.compact,
                  ),
                  segments: const [
                    ButtonSegment(
                        value: ThemeMode.dark,
                        icon: Icon(Icons.dark_mode_outlined, size: 16),
                        tooltip: '暗色'),
                    ButtonSegment(
                        value: ThemeMode.light,
                        icon: Icon(Icons.light_mode_outlined, size: 16),
                        tooltip: '亮色'),
                    ButtonSegment(
                        value: ThemeMode.system,
                        icon: Icon(Icons.brightness_auto_outlined, size: 16),
                        tooltip: '跟随系统'),
                  ],
                  selected: {prefs.themeMode},
                  showSelectedIcon: false,
                  onSelectionChanged: (s) =>
                      prefsNotifier.setThemeMode(s.first),
                ),
              ),
              const Divider(height: 1),
              SwitchListTile(
                secondary: const Icon(Icons.show_chart, size: 18),
                title: const Text('显示趋势图',
                    style: TextStyle(fontSize: 14)),
                subtitle: const Text('Dashboard 统计卡上的 sparkline',
                    style: TextStyle(fontSize: 12)),
                value: prefs.showSparkline,
                onChanged: prefsNotifier.setShowSparkline,
                dense: true,
              ),
              const Divider(height: 1),
              SwitchListTile(
                secondary: const Icon(Icons.animation, size: 18),
                title: const Text('启用动画',
                    style: TextStyle(fontSize: 14)),
                subtitle: const Text('页面过渡与列表进场动画',
                    style: TextStyle(fontSize: 12)),
                value: prefs.enableAnimations,
                onChanged: prefsNotifier.setEnableAnimations,
                dense: true,
              ),
              const Divider(height: 1),
              _WorkspaceDirTile(
                currentDir: prefs.workspaceDir,
                onSave: (value) => prefsNotifier.setWorkspaceDir(value),
              ),
            ],
          ),
          const SizedBox(height: 16),
          const _Section(
            title: 'Package Catalog',
            children: [_CatalogUrlTile()],
          ),
          const SizedBox(height: 16),
          _Section(
            title: 'About',
            children: [
              _SettingTile(
                icon: Icons.info_outline,
                title: 'WEFT Client',
                subtitle: 'v0.1.0',
                onTap: null,
              ),
              const Divider(height: 1),
              _SettingTile(
                icon: Icons.computer_outlined,
                title: 'Platform',
                subtitle: '${_platformName()} · ${Platform.localeName}',
                onTap: null,
              ),
              const Divider(height: 1),
              _SettingTile(
                icon: Icons.folder_outlined,
                title: '打开数据目录',
                subtitle: '在文件管理器中查看应用数据',
                onTap: () => _openDataDir(context),
              ),
            ],
          ),
        ],
      ),
    );
  }

  static String _platformName() {
    if (Platform.isWindows) return 'Windows';
    if (Platform.isMacOS) return 'macOS';
    if (Platform.isLinux) return 'Linux';
    return Platform.operatingSystem;
  }

  Future<void> _openDataDir(BuildContext context) async {
    final messenger = ScaffoldMessenger.of(context);
    try {
      final dir = await getApplicationSupportDirectory();
      Directory(dir.path).createSync(recursive: true);
      if (Platform.isWindows) {
        await Process.run('explorer.exe', [dir.path]);
      } else {
        final ok = await launchUrl(Uri.directory(dir.path))
            .timeout(const Duration(seconds: 5), onTimeout: () => false);
        if (!ok) {
          messenger.showSnackBar(
              SnackBar(content: Text('无法打开：${dir.path}')));
        }
      }
    } catch (e) {
      messenger.showSnackBar(SnackBar(content: Text('失败：$e')));
    }
  }
}

/// 计数小块：标签 + 大数字（连接面板用）。
class _CountChip extends StatelessWidget {
  const _CountChip({required this.label, required this.count});
  final String label;
  final int? count;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(right: 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(count?.toString() ?? '—',
              style: theme.textTheme.titleMedium
                  ?.copyWith(fontWeight: FontWeight.w600)),
          Text(label.toUpperCase(), style: theme.textTheme.labelLarge),
        ],
      ),
    );
  }
}

class _Section extends StatelessWidget {
  const _Section({required this.title, required this.children});
  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(title,
            style: Theme.of(context).textTheme.titleSmall?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant)),
        const SizedBox(height: 8),
        Card(child: Column(children: children)),
      ],
    );
  }
}

class _SettingTile extends StatelessWidget {
  const _SettingTile({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.onTap,
    this.trailing,
  });
  final IconData icon;
  final String title;
  final String subtitle;
  final VoidCallback? onTap;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: Icon(icon, size: 18),
      title: Text(title, style: const TextStyle(fontSize: 14)),
      subtitle: Text(subtitle, style: const TextStyle(fontSize: 12)),
      trailing: trailing,
      onTap: onTap,
      dense: true,
    );
  }
}

/// 包清单(catalog)地址设置项:显示当前 URL,点击弹框编辑并持久化。
class _CatalogUrlTile extends ConsumerStatefulWidget {
  const _CatalogUrlTile();

  @override
  ConsumerState<_CatalogUrlTile> createState() => _CatalogUrlTileState();
}

class _CatalogUrlTileState extends ConsumerState<_CatalogUrlTile> {
  String? _url;

  @override
  void initState() {
    super.initState();
    _loadUrl();
  }

  Future<void> _loadUrl() async {
    final url = await ref.read(catalogServiceProvider).catalogUrl();
    if (mounted) setState(() => _url = url);
  }

  Future<void> _edit() async {
    final controller = TextEditingController(text: _url ?? '');
    final result = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('包清单地址'),
        content: TextField(
          controller: controller,
          autofocus: true,
          decoration: const InputDecoration(
            hintText: 'https://raw.githubusercontent.com/owner/repo/main/catalog.json',
          ),
        ),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(ctx), child: const Text('取消')),
          FilledButton(
              onPressed: () => Navigator.pop(ctx, controller.text.trim()),
              child: const Text('保存')),
        ],
      ),
    );
    if (result != null && result.isNotEmpty) {
      await ref.read(catalogServiceProvider).setCatalogUrl(result);
      if (mounted) setState(() => _url = result);
    }
  }

  @override
  Widget build(BuildContext context) {
    return _SettingTile(
      icon: Icons.inventory_2_outlined,
      title: '清单地址',
      subtitle: _url ?? '加载中…',
      trailing: const Icon(Icons.edit_outlined, size: 15),
      onTap: _edit,
    );
  }
}

/// 可编辑的 Core URL 设置项。改动写入偏好，apiClient 会随之重建并连到新地址。
class _CoreUrlTile extends StatelessWidget {
  const _CoreUrlTile({required this.url, required this.onSave});

  final String url;
  final Future<void> Function(String) onSave;

  Future<void> _edit(BuildContext context) async {
    final controller = TextEditingController(text: url);
    final messenger = ScaffoldMessenger.of(context);
    final result = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Core 连接地址'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            TextField(
              controller: controller,
              autofocus: true,
              decoration: const InputDecoration(
                hintText: 'http://127.0.0.1:17830',
              ),
            ),
            const SizedBox(height: 8),
            const Text(
              '默认连接本机自带的 weft-core。仅在连接远程 core 时才需要修改。',
              style: TextStyle(fontSize: 12, color: Colors.grey),
            ),
          ],
        ),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(ctx), child: const Text('取消')),
          TextButton(
              onPressed: () =>
                  Navigator.pop(ctx, 'http://127.0.0.1:17830'),
              child: const Text('恢复默认')),
          FilledButton(
              onPressed: () => Navigator.pop(ctx, controller.text.trim()),
              child: const Text('保存')),
        ],
      ),
    );
    if (result != null) {
      await onSave(result);
      messenger.showSnackBar(
          const SnackBar(content: Text('Core 地址已更新')));
    }
  }

  @override
  Widget build(BuildContext context) {
    return _SettingTile(
      icon: Icons.dns_outlined,
      title: 'Core URL',
      subtitle: url,
      trailing: const Icon(Icons.edit_outlined, size: 15),
      onTap: () => _edit(context),
    );
  }
}

/// AI 工作目录设置项：输入框 + 浏览按钮。
class _WorkspaceDirTile extends StatelessWidget {
  const _WorkspaceDirTile({required this.currentDir, required this.onSave});

  final String currentDir;
  final Future<void> Function(String) onSave;

  Future<void> _edit(BuildContext context) async {
    final controller = TextEditingController(text: currentDir);
    final messenger = ScaffoldMessenger.of(context);
    final result = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('工作目录'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: controller,
                    autofocus: true,
                    decoration: const InputDecoration(
                      hintText: '留空使用默认(data/workspaces/)',
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                IconButton(
                  icon: const Icon(Icons.folder_open),
                  tooltip: '浏览',
                  onPressed: () async {
                    final picked = await FilePicker.getDirectoryPath(
                      dialogTitle: '选择工作目录',
                      initialDirectory: controller.text.isNotEmpty
                          ? controller.text
                          : null,
                    );
                    if (picked != null) {
                      controller.text = picked;
                    }
                  },
                ),
              ],
            ),
            const SizedBox(height: 8),
            const Text(
              'AI 文件操作(读写/执行)的沙盒目录。设置后新对话中的相对路径都解析到此目录。',
              style: TextStyle(fontSize: 12, color: Colors.grey),
            ),
          ],
        ),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(ctx), child: const Text('取消')),
          TextButton(
              onPressed: () => Navigator.pop(ctx, ''),
              child: const Text('恢复默认')),
          FilledButton(
              onPressed: () => Navigator.pop(ctx, controller.text.trim()),
              child: const Text('保存')),
        ],
      ),
    );
    if (result != null) {
      await onSave(result);
      messenger.showSnackBar(
          SnackBar(content: Text(result.isEmpty ? '已恢复默认工作目录' : '工作目录已设为 $result')));
    }
  }

  @override
  Widget build(BuildContext context) {
    return _SettingTile(
      icon: Icons.folder_outlined,
      title: '工作目录',
      subtitle: currentDir.isEmpty ? '默认 (data/workspaces/)' : currentDir,
      trailing: const Icon(Icons.edit_outlined, size: 15),
      onTap: () => _edit(context),
    );
  }
}
