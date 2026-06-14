import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/models/app.dart';
import '../../core/models/package.dart';
import '../../core/providers/core_repository.dart';
import '../../core/providers/data_providers.dart';
import '../../main.dart' show navigatorKey;
import 'glass_card.dart';

/// 单条命令。
class _Command {
  const _Command({
    required this.id,
    required this.label,
    required this.icon,
    required this.group,
    required this.onRun,
    this.keywords = '',
  });

  final String id;
  final String label;
  final IconData icon;
  final String group;
  final String keywords;
  final void Function(ScaffoldMessengerState, WidgetRef) onRun;
}

/// 全局 ⌘K / Ctrl+K 命令面板（Linear / Raycast 招牌）。
///
/// 模糊搜索所有路由 + 已注册 apps，键盘 ↑↓ 选择、Enter 执行、Esc 关闭。
class CommandPalette extends ConsumerStatefulWidget {
  const CommandPalette({super.key});

  /// 唤起命令面板（顶部下滑淡入）。
  static Future<void> show(BuildContext context) {
    return showGeneralDialog(
      context: context,
      barrierDismissible: true,
      barrierLabel: 'Command palette',
      barrierColor: const Color(0x99000000),
      transitionDuration: const Duration(milliseconds: 160),
      pageBuilder: (context, a, b) => const CommandPalette(),
      transitionBuilder: (context, anim, sec, child) {
        final curved =
            CurvedAnimation(parent: anim, curve: Curves.easeOutCubic);
        return FadeTransition(
          opacity: curved,
          child: SlideTransition(
            position: Tween(begin: const Offset(0, -0.04), end: Offset.zero)
                .animate(curved),
            child: child,
          ),
        );
      },
    );
  }

  @override
  ConsumerState<CommandPalette> createState() => _CommandPaletteState();
}

class _CommandPaletteState extends ConsumerState<CommandPalette> {
  final _searchCtrl = TextEditingController();
  final _focus = FocusNode();
  final _scrollCtrl = ScrollController();
  String _query = '';
  int _selected = 0;

  static const _nav = [
    (path: '/dashboard', label: 'Dashboard', icon: Icons.dashboard_outlined),
    (path: '/chat', label: 'Chat', icon: Icons.chat_outlined),
    (path: '/orchestration', label: 'Orchestration', icon: Icons.account_tree_outlined),
    (path: '/packages', label: 'Packages', icon: Icons.extension_outlined),
    (path: '/providers', label: 'Providers', icon: Icons.bolt_outlined),
    (path: '/services', label: 'Services', icon: Icons.dns_outlined),
    (path: '/settings', label: 'Settings', icon: Icons.settings_outlined),
  ];

  @override
  void initState() {
    super.initState();
    _focus.requestFocus();
  }

  @override
  void dispose() {
    _searchCtrl.dispose();
    _focus.dispose();
    _scrollCtrl.dispose();
    super.dispose();
  }

  List<_Command> _allCommands() {
    void goTo(String path) => navigatorKey.currentContext?.go(path);

    final cmds = <_Command>[
      for (final n in _nav)
        _Command(
          id: 'nav:${n.path}',
          label: n.label,
          icon: n.icon,
          group: 'Navigation',
          onRun: (_, _) => goTo(n.path),
        ),
    ];
    final apps = ref.read(appsProvider).asData?.value ?? const <ResolvedApp>[];
    for (final a in apps) {
      cmds.add(_Command(
        id: 'app:${a.name}',
        label: a.displayName,
        icon: Icons.apps_outlined,
        group: 'Apps',
        keywords: a.name,
        onRun: (_, _) => goTo('/apps/${a.name}'),
      ));
    }
    // Package 动作（安全、可逆）：Toggle / Reload。
    final pkgs =
        ref.read(packagesProvider).asData?.value ?? const <PackageInfo>[];
    for (final p in pkgs) {
      cmds.add(_Command(
        id: 'pkg-toggle:${p.name}',
        label: '${p.enabled ? "Disable" : "Enable"} package: ${p.name}',
        icon: p.enabled
            ? Icons.toggle_on_outlined
            : Icons.toggle_off_outlined,
        group: 'Package Actions',
        keywords: 'toggle ${p.name}',
        onRun: (messenger, ref) async {
          try {
            await ref.read(coreRepositoryProvider).togglePackage(p.name);
            ref.invalidate(packagesProvider);
            messenger.showSnackBar(SnackBar(
                content: Text(
                    '${p.enabled ? "Disabled" : "Enabled"} ${p.name}')));
          } catch (e) {
            messenger.showSnackBar(SnackBar(content: Text('Failed: $e')));
          }
        },
      ));
      cmds.add(_Command(
        id: 'pkg-reload:${p.name}',
        label: 'Reload package: ${p.name}',
        icon: Icons.refresh,
        group: 'Package Actions',
        keywords: 'reload ${p.name}',
        onRun: (messenger, ref) async {
          try {
            await ref.read(coreRepositoryProvider).reloadPackage(p.name);
            ref.invalidate(packagesProvider);
            messenger.showSnackBar(
                SnackBar(content: Text('Reloaded ${p.name}')));
          } catch (e) {
            messenger.showSnackBar(SnackBar(content: Text('Failed: $e')));
          }
        },
      ));
    }
    return cmds;
  }

  List<_Command> _filtered() {
    final q = _query.trim().toLowerCase();
    final all = _allCommands();
    if (q.isEmpty) return all;
    return all.where((c) {
      final hay = '${c.label} ${c.keywords} ${c.group}'.toLowerCase();
      // 简单子序列模糊匹配。
      var i = 0;
      for (final ch in hay.split('')) {
        if (i < q.length && ch == q[i]) i++;
      }
      return i == q.length || hay.contains(q);
    }).toList();
  }

  void _run(List<_Command> results) {
    if (results.isEmpty) return;
    final cmd = results[_selected.clamp(0, results.length - 1)];
    // pop 前捕获根 messenger，避免面板关闭后 context 失效。
    final messenger = ScaffoldMessenger.of(context);
    Navigator.of(context).pop();
    cmd.onRun(messenger, ref);
  }

  void _move(int delta, int len) {
    if (len == 0) return;
    setState(() => _selected = (_selected + delta) % len);
    if (_selected < 0) _selected += len;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final results = _filtered();
    if (_selected >= results.length) _selected = 0;

    return Align(
      alignment: const Alignment(0, -0.55),
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560, maxHeight: 440),
          child: Shortcuts(
            shortcuts: const {
              SingleActivator(LogicalKeyboardKey.arrowDown): _DownIntent(),
              SingleActivator(LogicalKeyboardKey.arrowUp): _UpIntent(),
              SingleActivator(LogicalKeyboardKey.enter): _RunIntent(),
              SingleActivator(LogicalKeyboardKey.escape): _CloseIntent(),
            },
            child: Actions(
              actions: {
                _DownIntent: CallbackAction<_DownIntent>(
                    onInvoke: (_) => _move(1, results.length)),
                _UpIntent: CallbackAction<_UpIntent>(
                    onInvoke: (_) => _move(-1, results.length)),
                _RunIntent: CallbackAction<_RunIntent>(
                    onInvoke: (_) => _run(results)),
                _CloseIntent: CallbackAction<_CloseIntent>(
                    onInvoke: (_) => Navigator.of(context).pop()),
              },
              child: GlassCard(
                radius: 14,
                child: Material(
                  type: MaterialType.transparency,
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                    // 搜索框。
                    Padding(
                      padding: const EdgeInsets.fromLTRB(14, 12, 14, 8),
                      child: Row(
                        children: [
                          Icon(Icons.search,
                              size: 18,
                              color: theme.colorScheme.onSurfaceVariant),
                          const SizedBox(width: 10),
                          Expanded(
                            child: TextField(
                              controller: _searchCtrl,
                              focusNode: _focus,
                              autofocus: true,
                              style: theme.textTheme.bodyMedium,
                              decoration: InputDecoration(
                                isDense: true,
                                filled: false,
                                border: InputBorder.none,
                                enabledBorder: InputBorder.none,
                                focusedBorder: InputBorder.none,
                                hintText: 'Search or jump to…',
                                hintStyle: theme.textTheme.bodyMedium?.copyWith(
                                    color: theme.colorScheme.onSurfaceVariant),
                              ),
                              onChanged: (v) =>
                                  setState(() { _query = v; _selected = 0; }),
                              onSubmitted: (_) => _run(results),
                            ),
                          ),
                          _KeyHint(label: 'ESC'),
                        ],
                      ),
                    ),
                    const Divider(height: 1),
                    // 结果列表。
                    Flexible(
                      child: results.isEmpty
                          ? Padding(
                              padding: const EdgeInsets.all(28),
                              child: Text('No results',
                                  style: theme.textTheme.bodySmall),
                            )
                          : ListView.builder(
                              controller: _scrollCtrl,
                              shrinkWrap: true,
                              padding: const EdgeInsets.all(6),
                              itemCount: results.length,
                              itemBuilder: (ctx, i) {
                                final c = results[i];
                                final showGroup = i == 0 ||
                                    results[i - 1].group != c.group;
                                return Column(
                                  crossAxisAlignment:
                                      CrossAxisAlignment.start,
                                  children: [
                                    if (showGroup)
                                      Padding(
                                        padding: const EdgeInsets.fromLTRB(
                                            10, 8, 10, 4),
                                        child: Text(c.group.toUpperCase(),
                                            style:
                                                theme.textTheme.labelLarge),
                                      ),
                                    _Row(
                                      cmd: c,
                                      selected: i == _selected,
                                      onHover: () =>
                                          setState(() => _selected = i),
                                      onTap: () => _run(results),
                                    ),
                                  ],
                                );
                              },
                            ),
                    ),
                  ],
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _Row extends StatelessWidget {
  const _Row({
    required this.cmd,
    required this.selected,
    required this.onHover,
    required this.onTap,
  });

  final _Command cmd;
  final bool selected;
  final VoidCallback onHover;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return MouseRegion(
      onEnter: (_) => onHover(),
      cursor: SystemMouseCursors.click,
      child: GestureDetector(
        onTap: onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 90),
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 9),
          decoration: BoxDecoration(
            color: selected
                ? theme.colorScheme.primary.withValues(alpha: 0.12)
                : Colors.transparent,
            borderRadius: BorderRadius.circular(8),
          ),
          child: Row(
            children: [
              Icon(cmd.icon,
                  size: 16,
                  color: selected
                      ? theme.colorScheme.primary
                      : theme.colorScheme.onSurfaceVariant),
              const SizedBox(width: 10),
              Expanded(
                child: Text(cmd.label,
                    style: theme.textTheme.bodyMedium,
                    overflow: TextOverflow.ellipsis),
              ),
              if (selected) _KeyHint(label: '↵'),
            ],
          ),
        ),
      ),
    );
  }
}

class _KeyHint extends StatelessWidget {
  const _KeyHint({required this.label});
  final String label;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: GlassTokens.innerTileFill,
        borderRadius: BorderRadius.circular(5),
        border: Border.all(color: GlassTokens.borderIdle),
      ),
      child: Text(label,
          style: theme.textTheme.labelSmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
              fontSize: 10,
              fontWeight: FontWeight.w600)),
    );
  }
}

class _DownIntent extends Intent {
  const _DownIntent();
}

class _UpIntent extends Intent {
  const _UpIntent();
}

class _RunIntent extends Intent {
  const _RunIntent();
}

class _CloseIntent extends Intent {
  const _CloseIntent();
}
