import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import '../../core/models/app.dart';
import '../../core/models/error.dart';
import '../../core/providers/connection_provider.dart';
import '../../core/providers/data_providers.dart';
import '../../core/providers/pinned_apps_provider.dart';
import '../../core/providers/preferences_provider.dart';
import '../../shared/widgets/skeleton.dart';
import '../../shared/widgets/hover_card.dart';
import '../../shared/widgets/glass_card.dart';
import '../../shared/widgets/sparkline.dart';
import '../../shared/widgets/empty_state.dart';
import '../../shared/theme/app_theme.dart';
import '../../shared/theme/spacing.dart';
import '../apps/surface/native_registry.dart';

class DashboardScreen extends ConsumerWidget {
  const DashboardScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final conn = ref.watch(connectionProvider);
    final apps = ref.watch(appsProvider);
    final providers = ref.watch(providersProvider);
    final packages = ref.watch(packagesProvider);
    final animate =
        ref.watch(preferencesProvider.select((p) => p.enableAnimations));
    final theme = Theme.of(context);

    return Scaffold(
      backgroundColor: Colors.transparent,
      body: Column(
        children: [
          // ── Connection status bar ──────────────────────────────────────
          _ConnectionBar(conn: conn, onRetry: () => ref.read(connectionProvider.notifier).retry()),

          // ── Offline banner ────────────────────────────────────────────
          if (conn.isOffline)
            _OfflineBanner(
              onRetry: () {
                ref.read(connectionProvider.notifier).retry();
                ref.invalidate(appsProvider);
                ref.invalidate(providersProvider);
                ref.invalidate(packagesProvider);
              },
            ),

          // ── Fixed header (non-scrolling) ──────────────────────────────
          // Header row + stats 固定在顶部，使统计卡的真实模糊成本恒定、
          // 与下方滚动列表无关。
          Padding(
            padding: const EdgeInsets.fromLTRB(
                Spacing.lg, Spacing.sm, Spacing.lg, 0),
            child: Column(
              children: [
                Row(
                  children: [
                    Text(
                      'Dashboard',
                      style: theme.textTheme.headlineSmall
                          ?.copyWith(fontWeight: FontWeight.w600),
                    ),
                    const Spacer(),
                    IconButton(
                      icon: const Icon(Icons.refresh, size: 16),
                      onPressed: () {
                        ref.invalidate(appsProvider);
                        ref.invalidate(providersProvider);
                        ref.invalidate(packagesProvider);
                      },
                      tooltip: 'Refresh',
                    ),
                  ],
                ),
                const SizedBox(height: Spacing.md + 4),
                GridView.count(
                  crossAxisCount: 3,
                  shrinkWrap: true,
                  physics: const NeverScrollableScrollPhysics(),
                  crossAxisSpacing: Spacing.md - 4,
                  mainAxisSpacing: Spacing.md - 4,
                  childAspectRatio: 1.6,
                  children: [
                    _StatCard(
                      icon: Icons.apps_outlined,
                      label: 'Apps',
                      asyncValue: apps,
                      count: (list) => list.length,
                    ),
                    _StatCard(
                      icon: Icons.extension_outlined,
                      label: 'Packages',
                      asyncValue: packages,
                      count: (list) => list.length,
                      subCount: (list) =>
                          list.where((p) => p.enabled).length,
                      subLabel: 'enabled',
                    ),
                    _StatCard(
                      icon: Icons.bolt_outlined,
                      label: 'Providers',
                      asyncValue: providers,
                      count: (list) => list.length,
                    ),
                  ],
                ),
              ],
            ),
          ),

          // ── Apps section (scrolling) ──────────────────────────────────
          Expanded(
            child: ListView(
              padding: const EdgeInsets.fromLTRB(
                  Spacing.lg, Spacing.lg, Spacing.lg, Spacing.lg),
              children: [
                Text(
                  'Apps',
                  style: theme.textTheme.titleSmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant),
                ),
                const SizedBox(height: 12),
                apps.when(
                  data: (list) => list.isEmpty
                      ? const EmptyState(
                          icon: Icons.apps_outlined,
                          title: 'No apps registered',
                          subtitle:
                              'Apps will appear here once weft-core loads them.',
                        )
                      : Column(
                          children: [
                            for (var i = 0; i < list.length; i++)
                              _StaggerIn(
                                index: i,
                                animate: animate,
                                child: _AppCard(app: list[i]),
                              ),
                          ],
                        ),
                  loading: () => const SkeletonList(count: 3),
                  error: (e, _) => _ErrorState(
                    error: e,
                    onRetry: () => ref.invalidate(appsProvider),
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

// ── Connection status bar ────────────────────────────────────────────────────

class _ConnectionBar extends StatelessWidget {
  const _ConnectionBar({required this.conn, required this.onRetry});
  final CoreConnectionState conn;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final warm = theme.extension<AppSurfaces>()!;
    final (label, color, icon) = switch (conn.status) {
      CoreConnectionStatus.connected => (
          'weft-core connected',
          warm.statusOk,
          Icons.circle,
        ),
      CoreConnectionStatus.connecting => (
          'Connecting…',
          warm.statusWarn,
          Icons.circle,
        ),
      CoreConnectionStatus.offline => (
          'weft-core offline',
          warm.statusError,
          Icons.circle,
        ),
    };

    // 不透明圆角连接条：固定尺寸非滚动 chrome，零 BackdropFilter。
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 4),
      child: Container(
        width: double.infinity,
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
        decoration: BoxDecoration(
          color: theme.colorScheme.surface,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: GlassTokens.borderIdle),
        ),
        child: Row(
          children: [
            Icon(icon, size: 8, color: color),
            const SizedBox(width: 8),
            Text(
              label,
              style: theme.textTheme.labelSmall?.copyWith(color: color),
            ),
            if (conn.lastChecked != null) ...[
              const SizedBox(width: 8),
              Text(
                '· last checked ${_formatTime(conn.lastChecked!)}',
                style: theme.textTheme.labelSmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant),
              ),
            ],
            const Spacer(),
            if (conn.status == CoreConnectionStatus.connecting)
              const SizedBox(
                width: 10,
                height: 10,
                child: CircularProgressIndicator(strokeWidth: 1.5),
              ),
          ],
        ),
      ),
    );
  }

  String _formatTime(DateTime dt) {
    final now = DateTime.now();
    final diff = now.difference(dt);
    if (diff.inSeconds < 5) return 'just now';
    if (diff.inSeconds < 60) return '${diff.inSeconds}s ago';
    return '${diff.inMinutes}m ago';
  }
}

// ── Offline banner ───────────────────────────────────────────────────────────

class _OfflineBanner extends StatelessWidget {
  const _OfflineBanner({required this.onRetry});
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final err = theme.colorScheme.error;
    return Container(
      margin: const EdgeInsets.fromLTRB(16, 4, 16, 0),
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
      decoration: BoxDecoration(
        color: err.withValues(alpha: 0.10),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: err.withValues(alpha: 0.35)),
      ),
      child: Row(
        children: [
          Icon(Icons.cloud_off_outlined, color: err, size: 16),
          const SizedBox(width: 10),
          Expanded(
            child: Text(
              'weft-core is not running',
              style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurface),
            ),
          ),
          TextButton(
            onPressed: onRetry,
            style: TextButton.styleFrom(
              foregroundColor: err,
              padding: const EdgeInsets.symmetric(horizontal: 10),
              minimumSize: const Size(0, 32),
            ),
            child: const Text('Retry'),
          ),
        ],
      ),
    );
  }
}

// ── Stats card ───────────────────────────────────────────────────────────────

class _StatCard<T> extends StatelessWidget {
  const _StatCard({
    required this.icon,
    required this.label,
    required this.asyncValue,
    required this.count,
    this.subCount,
    this.subLabel,
  });

  final IconData icon;
  final String label;
  final AsyncValue<List<T>> asyncValue;
  final int Function(List<T>) count;
  final int Function(List<T>)? subCount;
  final String? subLabel;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final warm = theme.extension<AppSurfaces>()!;
    // 英雄立面卡：竖向紧凑布局，抬升一档亮度，零 BackdropFilter。
    return GlassCard(
      elevated: true,
      padding: const EdgeInsets.all(Spacing.md),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Row(
            children: [
              Icon(icon, size: 18, color: theme.colorScheme.primary),
              const Spacer(),
              Consumer(
                builder: (context, ref, _) {
                  final show = ref.watch(
                      preferencesProvider.select((p) => p.showSparkline));
                  if (!show) return const SizedBox.shrink();
                  return SizedBox(
                    width: 64,
                    child: Sparkline.seeded(
                      seed: label.hashCode,
                      color: theme.colorScheme.primary,
                      height: 22,
                    ),
                  );
                },
              ),
            ],
          ),
          Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              asyncValue.when(
                data: (list) {
                  final n = count(list);
                  final sub = subCount != null ? subCount!(list) : null;
                  return Row(
                    crossAxisAlignment: CrossAxisAlignment.baseline,
                    textBaseline: TextBaseline.alphabetic,
                    children: [
                      TweenAnimationBuilder<double>(
                        tween: Tween(begin: 0, end: n.toDouble()),
                        duration: const Duration(milliseconds: 650),
                        curve: Curves.easeOutCubic,
                        builder: (context, value, _) => Text(
                          '${value.round()}',
                          style: warm.mono.copyWith(
                              fontSize: 32,
                              fontWeight: FontWeight.w600,
                              letterSpacing: -0.8),
                        ),
                      ),
                      if (sub != null && subLabel != null) ...[
                        const SizedBox(width: 6),
                        Padding(
                          padding: const EdgeInsets.only(bottom: 4),
                          child: Text('$sub $subLabel',
                              style: theme.textTheme.labelSmall?.copyWith(
                                  color: theme.colorScheme.onSurfaceVariant)),
                        ),
                      ],
                    ],
                  );
                },
                loading: () => const SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator(strokeWidth: 2)),
                error: (_, _) => Text('—',
                    style: warm.mono.copyWith(
                        fontSize: 32,
                        color: theme.colorScheme.onSurfaceVariant)),
              ),
              const SizedBox(height: 2),
              Text(label.toUpperCase(), style: theme.textTheme.labelLarge),
            ],
          ),
        ],
      ),
    );
  }
}

// ── App card (launcher tile) ─────────────────────────────────────────────────

class _AppCard extends ConsumerWidget {
  const _AppCard({required this.app});
  final ResolvedApp app;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final warm = theme.extension<AppSurfaces>()!;
    final hasUi = nativeSurfaceRegistry.containsKey(app.name);
    final pinned = ref.watch(pinnedAppsProvider).contains(app.name);
    final statusColor = switch (app.status) {
      ResolvedAppStatus.ok => warm.statusOk,
      ResolvedAppStatus.resolved => warm.statusOk,
      ResolvedAppStatus.partial => warm.statusWarn,
      ResolvedAppStatus.degraded => warm.statusWarn,
      ResolvedAppStatus.error => warm.statusError,
      ResolvedAppStatus.failed => warm.statusError,
      ResolvedAppStatus.unknown => const Color(0xFF6E7178),
    };
    final muted = theme.colorScheme.onSurfaceVariant;

    return Opacity(
      opacity: hasUi ? 1.0 : 0.55,
      child: HoverCard(
        // 有 UI 才可"打开"进功能界面；无 UI 仍可进诊断（降级）
        onTap: () => context.go('/apps/${app.name}'),
        child: Padding(
          padding: const EdgeInsets.all(Spacing.md),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              // App icon
              Container(
                width: 40,
                height: 40,
                decoration: BoxDecoration(
                  color: GlassTokens.innerTileFill,
                  borderRadius: BorderRadius.circular(GlassTokens.radiusInner),
                  border: Border.all(color: GlassTokens.borderIdle),
                ),
                child: Icon(
                  _iconFor(null),
                  size: 20,
                  color: hasUi ? theme.colorScheme.primary : muted,
                ),
              ),
              const SizedBox(width: Spacing.md),
              // Title + status / description
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(children: [
                      Container(
                        width: 8,
                        height: 8,
                        decoration: BoxDecoration(
                            color: statusColor, shape: BoxShape.circle),
                      ),
                      const SizedBox(width: Spacing.sm),
                      Flexible(
                        child: Text(app.displayName,
                            style: theme.textTheme.bodyMedium
                                ?.copyWith(fontWeight: FontWeight.w500)),
                      ),
                      const SizedBox(width: Spacing.sm),
                      Text('v${app.version}',
                          style: theme.textTheme.bodySmall?.copyWith(color: muted)),
                    ]),
                    if (app.description.isNotEmpty)
                      Padding(
                        padding: const EdgeInsets.only(top: 2),
                        child: Text(app.description,
                            maxLines: 2,
                            overflow: TextOverflow.ellipsis,
                            style: theme.textTheme.bodySmall?.copyWith(color: muted)),
                      ),
                  ],
                ),
              ),
              const SizedBox(width: Spacing.md),
              // Open affordance / no-UI marker
              if (hasUi) ...[
                Row(mainAxisSize: MainAxisSize.min, children: [
                  Text('打开',
                      style: theme.textTheme.labelMedium
                          ?.copyWith(color: theme.colorScheme.primary)),
                  Icon(Icons.chevron_right,
                      size: 18, color: theme.colorScheme.primary),
                ]),
                const SizedBox(width: Spacing.sm),
                IconButton(
                  icon: Icon(pinned ? Icons.push_pin : Icons.push_pin_outlined,
                      size: 16),
                  color: pinned ? theme.colorScheme.primary : null,
                  tooltip: pinned ? '取消固定' : '固定到侧边栏',
                  visualDensity: VisualDensity.compact,
                  onPressed: () =>
                      ref.read(pinnedAppsProvider.notifier).toggle(app.name),
                ),
              ] else
                _Tag(label: '无界面', color: muted),
            ],
          ),
        ),
      ),
    );
  }

  /// Maps Core-provided icon name (e.g. "robot") to a Material icon.
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

// ── Small tag chip ───────────────────────────────────────────────────────────

class _Tag extends StatelessWidget {
  const _Tag({required this.label, required this.color});
  final String label;
  final Color color;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.12),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(label,
          style: theme.textTheme.labelSmall?.copyWith(color: color)),
    );
  }
}

// ── Shared error / empty states ──────────────────────────────────────────────

/// Unified error widget that handles [AppException] subtypes.
class _ErrorState extends StatelessWidget {
  const _ErrorState({required this.error, this.onRetry});
  final Object error;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    final (icon, title, subtitle) = switch (error) {
      CoreOfflineException e => (
          Icons.cloud_off_outlined,
          'Core offline',
          e.message,
        ),
      ApiException e => (
          Icons.error_outline,
          'Error ${e.statusCode}',
          e.message,
        ),
      _ => (
          Icons.warning_amber_outlined,
          'Something went wrong',
          error.toString(),
        ),
    };

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(children: [
          Icon(icon, color: theme.colorScheme.error, size: 20),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title,
                    style: theme.textTheme.bodyMedium?.copyWith(
                        color: theme.colorScheme.error,
                        fontWeight: FontWeight.w500)),
                Text(subtitle,
                    style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant)),
              ],
            ),
          ),
          if (onRetry != null)
            TextButton.icon(
              icon: const Icon(Icons.refresh, size: 14),
              label: const Text('Retry'),
              onPressed: onRetry,
            ),
        ]),
      ),
    );
  }
}

/// 列表项 stagger 进场：按 index 错峰淡入 + 轻微上滑，仅播放一次。
class _StaggerIn extends StatefulWidget {
  const _StaggerIn({
    required this.index,
    required this.child,
    this.animate = true,
  });
  final int index;
  final Widget child;
  final bool animate;

  @override
  State<_StaggerIn> createState() => _StaggerInState();
}

class _StaggerInState extends State<_StaggerIn>
    with SingleTickerProviderStateMixin {
  late final AnimationController _c = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 320),
  );
  late final Animation<double> _curve =
      CurvedAnimation(parent: _c, curve: Curves.easeOutCubic);

  @override
  void initState() {
    super.initState();
    if (!widget.animate) {
      _c.value = 1.0; // 关闭动画：直接显示末态。
      return;
    }
    // 错峰：每项延迟 index*45ms，上限 360ms，避免长列表拖尾。
    final delayMs = (widget.index * 45).clamp(0, 360);
    Future.delayed(Duration(milliseconds: delayMs), () {
      if (mounted) _c.forward();
    });
  }

  @override
  void dispose() {
    _c.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: _curve,
      builder: (context, child) => Opacity(
        opacity: _curve.value,
        child: Transform.translate(
          offset: Offset(0, 8 * (1 - _curve.value)),
          child: child,
        ),
      ),
      child: widget.child,
    );
  }
}
