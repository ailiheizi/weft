import 'dart:io' show Platform;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart' show Clipboard, ClipboardData;
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
              _SettingTile(
                icon: Icons.dns_outlined,
                title: 'Core URL',
                subtitle: 'http://127.0.0.1:3004',
                trailing: Icon(Icons.copy_outlined,
                    size: 15, color: theme.colorScheme.onSurfaceVariant),
                onTap: () async {
                  final messenger = ScaffoldMessenger.of(context);
                  await Clipboard.setData(
                      const ClipboardData(text: 'http://127.0.0.1:3004'));
                  messenger.showSnackBar(
                      const SnackBar(content: Text('已复制 Core URL')));
                },
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
            ],
          ),
          const SizedBox(height: 16),
          const _Section(
            title: 'Package Catalog',
            children: [_CatalogUrlTile()],
          ),
          const SizedBox(height: 16),
          const _Section(
            title: 'Keyboard Shortcuts',
            children: [
              _ShortcutTile(
                  keys: ['Ctrl', 'K'], description: '打开命令面板（搜索 / 跳转 / 执行）'),
              _ShortcutTile(keys: ['G', 'D'], description: '跳转到 Dashboard'),
              _ShortcutTile(keys: ['G', 'C'], description: '跳转到 Chat'),
              _ShortcutTile(keys: ['G', 'O'], description: '跳转到 Orchestration'),
              _ShortcutTile(keys: ['G', 'P'], description: '跳转到 Packages'),
              _ShortcutTile(keys: ['G', 'R'], description: '跳转到 Providers'),
              _ShortcutTile(keys: ['G', 'V'], description: '跳转到 Services'),
              _ShortcutTile(keys: ['G', 'S'], description: '跳转到 Settings'),
              _ShortcutTile(
                  keys: ['↑', '↓', '↵'],
                  description: '命令面板内：上下选择 / 回车执行'),
              _ShortcutTile(keys: ['Esc'], description: '关闭命令面板'),
            ],
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
      final ok = await launchUrl(Uri.file(dir.path));
      if (!ok) {
        messenger.showSnackBar(
            SnackBar(content: Text('无法打开：${dir.path}')));
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

/// 快捷键说明行：左侧描述，右侧键帽序列。
class _ShortcutTile extends StatelessWidget {
  const _ShortcutTile({required this.keys, required this.description});
  final List<String> keys;
  final String description;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 9),
      child: Row(
        children: [
          Expanded(
            child: Text(description,
                style: theme.textTheme.bodyMedium),
          ),
          for (var i = 0; i < keys.length; i++) ...[
            if (i > 0)
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 4),
                child: Text('then',
                    style: theme.textTheme.labelSmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant)),
              ),
            _KeyCap(label: keys[i]),
          ],
        ],
      ),
    );
  }
}

/// 键帽样式（内凹深色 + hairline 边 + 等宽字）。
class _KeyCap extends StatelessWidget {
  const _KeyCap({required this.label});
  final String label;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      margin: const EdgeInsets.only(left: 4),
      constraints: const BoxConstraints(minWidth: 22),
      padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 3),
      decoration: BoxDecoration(
        color: const Color(0xFF0A0B0E),
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: const Color(0x17FFFFFF)),
      ),
      child: Text(
        label,
        textAlign: TextAlign.center,
        style: const TextStyle(
          fontFamily: 'monospace',
          fontSize: 11,
          fontWeight: FontWeight.w600,
          fontFeatures: [FontFeature.tabularFigures()],
        ).copyWith(color: theme.colorScheme.onSurface),
      ),
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
