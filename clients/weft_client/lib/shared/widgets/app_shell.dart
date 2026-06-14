import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/models/app.dart';
import '../../core/providers/connection_provider.dart';
import '../../core/providers/data_providers.dart';
import '../../core/providers/pinned_apps_provider.dart';
import '../theme/app_theme.dart';
import 'command_palette.dart';

class _OpenPaletteIntent extends Intent {
  const _OpenPaletteIntent();
}

class AppShell extends ConsumerWidget {
  const AppShell({super.key, required this.child});
  final Widget child;

  // Fixed system destinations (always present).
  static const _fixed = [
    (path: '/dashboard', icon: Icons.dashboard_outlined, label: 'Dashboard'),
    (path: '/chat', icon: Icons.chat_outlined, label: 'Chat'),
    (path: '/orchestration', icon: Icons.groups_outlined, label: '团队'),
    (path: '/packages', icon: Icons.extension_outlined, label: 'Packages'),
    (path: '/providers', icon: Icons.bolt_outlined, label: 'Providers'),
    (path: '/services', icon: Icons.dns_outlined, label: 'Services'),
    (path: '/settings', icon: Icons.settings_outlined, label: 'Settings'),
  ];

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final pinned = ref.watch(pinnedAppsProvider);
    final appsAsync = ref.watch(appsProvider);
    final apps = appsAsync.asData?.value ?? const <ResolvedApp>[];

    // Resolve pinned app names to (path,label,icon). Drop pins whose app
    // is no longer present.
    final pinnedDest = <({String path, IconData icon, String label})>[];
    for (final name in pinned) {
      final app = apps.where((a) => a.name == name).firstOrNull;
      if (app == null) continue;
      pinnedDest.add((
        path: '/apps/${app.name}',
        icon: _iconFor(null),
        label: app.displayName,
      ));
    }

    // Unified destination list: fixed first, then pinned.
    final allPaths = [
      ..._fixed.map((d) => d.path),
      ...pinnedDest.map((d) => d.path),
    ];
    final location = GoRouterState.of(context).uri.path;
    var selected = allPaths.indexWhere((p) => location.startsWith(p));
    if (selected < 0) selected = 0;

    final destinations = <NavigationRailDestination>[
      ..._fixed.map((d) => NavigationRailDestination(
            icon: Icon(d.icon, size: 20),
            label: Text(d.label),
            padding: const EdgeInsets.symmetric(vertical: 4),
          )),
      ...pinnedDest.map((d) => NavigationRailDestination(
            icon: Icon(d.icon, size: 20),
            label: Text(d.label),
            padding: const EdgeInsets.symmetric(vertical: 4),
          )),
    ];

    return Shortcuts(
      shortcuts: const {
        SingleActivator(LogicalKeyboardKey.keyK, meta: true):
            _OpenPaletteIntent(),
        SingleActivator(LogicalKeyboardKey.keyK, control: true):
            _OpenPaletteIntent(),
      },
      child: Actions(
        actions: {
          _OpenPaletteIntent: CallbackAction<_OpenPaletteIntent>(
            onInvoke: (_) {
              CommandPalette.show(context);
              return null;
            },
          ),
        },
        child: _GotoKeyHandler(
          child: Scaffold(
            body: Column(
              children: [
                Expanded(
                  child: Row(
                    children: [
                      // 不透明侧栏 chrome：比画布略深的冷色实色。
                      ColoredBox(
                        color: const Color(0xFF0B0C10),
                        child: NavigationRail(
                          backgroundColor: Colors.transparent,
                          selectedIndex: selected,
                          onDestinationSelected: (i) =>
                              context.go(allPaths[i]),
                          leading: Padding(
                            padding:
                                const EdgeInsets.symmetric(vertical: 16),
                            child: Icon(Icons.hexagon_outlined,
                                color: theme.colorScheme.primary, size: 24),
                          ),
                          destinations: destinations,
                        ),
                      ),
                      const VerticalDivider(
                          width: 1, color: Color(0x0FFFFFFF)),
                      Expanded(child: child),
                    ],
                  ),
                ),
                const _StatusBar(),
              ],
            ),
          ),
        ),
      ),
    );
  }

  /// Maps Core-provided icon name to a Material icon. Mirrors dashboard.
  static IconData _iconFor(String? name) {
    switch (name) {
      case 'robot':
        return Icons.smart_toy_outlined;
      case 'movie':
      case 'video':
        return Icons.movie_outlined;
      case 'code':
        return Icons.code;
      case 'chat':
        return Icons.chat_outlined;
      default:
        return Icons.apps_outlined;
    }
  }
}

/// Linear 式连续键导航：按 `g` 后 1.2s 内再按目标键跳转。
/// g d=dashboard / c=chat / p=packages / s=store / o=orchestration /
/// r=providers / v=services / , =settings。输入框聚焦时不拦截。
class _GotoKeyHandler extends StatefulWidget {
  const _GotoKeyHandler({required this.child});
  final Widget child;

  @override
  State<_GotoKeyHandler> createState() => _GotoKeyHandlerState();
}

class _GotoKeyHandlerState extends State<_GotoKeyHandler> {
  bool _armed = false;
  DateTime? _armedAt;

  static final _map = {
    LogicalKeyboardKey.keyD: '/dashboard',
    LogicalKeyboardKey.keyC: '/chat',
    LogicalKeyboardKey.keyO: '/orchestration',
    LogicalKeyboardKey.keyP: '/packages',
    LogicalKeyboardKey.keyS: '/settings',
    LogicalKeyboardKey.keyR: '/providers',
    LogicalKeyboardKey.keyV: '/services',
  };

  KeyEventResult _onKey(FocusNode node, KeyEvent event) {
    if (event is! KeyDownEvent) return KeyEventResult.ignored;

    // 文本输入聚焦时不拦截（让用户能正常打字）。
    final primary = FocusManager.instance.primaryFocus;
    if (primary?.context?.widget is EditableText) {
      return KeyEventResult.ignored;
    }

    final now = DateTime.now();
    if (_armed && _armedAt != null &&
        now.difference(_armedAt!) < const Duration(milliseconds: 1200)) {
      final dest = _map[event.logicalKey];
      _armed = false;
      _armedAt = null;
      if (dest != null) {
        context.go(dest);
        return KeyEventResult.handled;
      }
      return KeyEventResult.ignored;
    }

    if (event.logicalKey == LogicalKeyboardKey.keyG) {
      _armed = true;
      _armedAt = now;
      return KeyEventResult.handled;
    }
    return KeyEventResult.ignored;
  }

  @override
  Widget build(BuildContext context) {
    return Focus(
      autofocus: true,
      onKeyEvent: _onKey,
      child: widget.child,
    );
  }
}

/// 全局底部状态栏（终端质感）：连接态脉冲点 + ⌘K 提示 + 版本号。
class _StatusBar extends ConsumerWidget {
  const _StatusBar();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final warm = theme.extension<AppSurfaces>()!;
    final conn = ref.watch(connectionProvider);
    final mono = warm.mono.copyWith(fontSize: 11);

    final (statusColor, statusText) = switch (conn.status) {
      CoreConnectionStatus.connected => (warm.statusOk, 'connected'),
      CoreConnectionStatus.connecting => (warm.statusWarn, 'connecting'),
      CoreConnectionStatus.offline => (warm.statusError, 'offline'),
    };

    return Container(
      height: 26,
      decoration: const BoxDecoration(
        color: Color(0xFF0B0C10),
        border: Border(top: BorderSide(color: Color(0x0FFFFFFF))),
      ),
      padding: const EdgeInsets.symmetric(horizontal: 12),
      child: Row(
        children: [
          PulseDot(
            color: statusColor,
            pulsing: conn.status == CoreConnectionStatus.connecting,
          ),
          const SizedBox(width: 6),
          Text('weft-core $statusText',
              style: mono.copyWith(color: theme.colorScheme.onSurfaceVariant)),
          const Spacer(),
          Icon(Icons.search,
              size: 11, color: theme.colorScheme.onSurfaceVariant),
          const SizedBox(width: 4),
          Text('Ctrl K  to search',
              style: mono.copyWith(color: theme.colorScheme.onSurfaceVariant)),
          const Spacer(),
          Text('v0.1.0',
              style: mono.copyWith(color: theme.colorScheme.onSurfaceVariant)),
        ],
      ),
    );
  }
}

/// 状态点：connecting 时呼吸脉冲，其余静态。
class PulseDot extends StatefulWidget {
  const PulseDot({super.key, required this.color, this.pulsing = false});
  final Color color;
  final bool pulsing;

  @override
  State<PulseDot> createState() => _PulseDotState();
}

class _PulseDotState extends State<PulseDot>
    with SingleTickerProviderStateMixin {
  late final AnimationController _c = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 1100),
  );

  @override
  void initState() {
    super.initState();
    if (widget.pulsing) _c.repeat(reverse: true);
  }

  @override
  void didUpdateWidget(PulseDot old) {
    super.didUpdateWidget(old);
    if (widget.pulsing && !_c.isAnimating) {
      _c.repeat(reverse: true);
    } else if (!widget.pulsing && _c.isAnimating) {
      _c.stop();
      _c.value = 0;
    }
  }

  @override
  void dispose() {
    _c.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: _c,
      builder: (context, _) {
        final glow = widget.pulsing ? (0.3 + 0.7 * _c.value) : 1.0;
        return Container(
          width: 7,
          height: 7,
          decoration: BoxDecoration(
            color: widget.color,
            shape: BoxShape.circle,
            boxShadow: [
              BoxShadow(
                color: widget.color.withValues(alpha: 0.5 * glow),
                blurRadius: 5 * glow,
                spreadRadius: 0.5 * glow,
              ),
            ],
          ),
        );
      },
    );
  }
}
