import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/providers/data_providers.dart';
import '../../core/providers/core_repository.dart';
import '../../core/models/service.dart';
import '../../shared/widgets/app_error_widget.dart';
import '../../shared/widgets/skeleton.dart';
import '../../shared/widgets/hover_card.dart';
import '../../shared/widgets/empty_state.dart';
import '../../shared/theme/app_theme.dart';
import '../../shared/theme/spacing.dart';

class ServicesScreen extends ConsumerWidget {
  const ServicesScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);

    return Scaffold(
      backgroundColor: Colors.transparent,
      body: ListView(
        padding: const EdgeInsets.all(Spacing.lg),
        children: [
          Row(children: [
            Text('Services',
                style: theme.textTheme.headlineSmall
                    ?.copyWith(fontWeight: FontWeight.w600)),
            const Spacer(),
            IconButton(
              icon: const Icon(Icons.refresh, size: 16),
              onPressed: () => ref.invalidate(servicesProvider),
            ),
          ]),
          const SizedBox(height: Spacing.lg),
          const ServicesBody(),
        ],
      ),
    );
  }
}

/// 服务列表主体（无 Scaffold/标题），供独立页或合并页(扩展 Tab)复用。
class ServicesBody extends ConsumerWidget {
  const ServicesBody({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final services = ref.watch(servicesProvider);
    return services.when(
            data: (list) => list.isEmpty
                ? const EmptyState(
                    icon: Icons.dns_outlined,
                    title: 'No services found',
                    subtitle: 'Services managed by weft-core will appear here.',
                  )
                : Column(
                    children: list
                        .map((s) => _ServiceTile(
                            service: s,
                            onStart: () async {
                              await ref
                                  .read(coreRepositoryProvider)
                                  .startService(s.name);
                              ref.invalidate(servicesProvider);
                            },
                            onStop: () async {
                              await ref
                                  .read(coreRepositoryProvider)
                                  .stopService(s.name);
                              ref.invalidate(servicesProvider);
                            },
                            onRestart: () async {
                              await ref
                                  .read(coreRepositoryProvider)
                                  .restartService(s.name);
                              ref.invalidate(servicesProvider);
                            }))
                        .toList()),
            loading: () => const SkeletonList(count: 4),
            error: (e, _) => AppErrorWidget(
              error: e,
              onRetry: () => ref.invalidate(servicesProvider),
            ),
          );
  }
}

class _ServiceTile extends StatelessWidget {
  const _ServiceTile({
    required this.service,
    required this.onStart,
    required this.onStop,
    required this.onRestart,
  });
  final ServiceInfo service;
  final VoidCallback onStart;
  final VoidCallback onStop;
  final VoidCallback onRestart;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isRunning = service.status == ServiceStatus.running;
    final surfaces = theme.extension<AppSurfaces>()!;
    final statusColor = switch (service.status) {
      ServiceStatus.running => surfaces.statusOk,
      ServiceStatus.stopped => const Color(0xFF6E7178),
      ServiceStatus.error => surfaces.statusError,
      ServiceStatus.unknown => surfaces.statusWarn,
    };

    return HoverCard(
      child: Padding(
        padding: const EdgeInsets.symmetric(
            horizontal: Spacing.md, vertical: Spacing.md - 4),
        child: Row(children: [
          Container(
            width: 8,
            height: 8,
            decoration:
                BoxDecoration(color: statusColor, shape: BoxShape.circle),
          ),
          const SizedBox(width: Spacing.md - 4),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(service.name,
                    style: theme.textTheme.bodyMedium
                        ?.copyWith(fontWeight: FontWeight.w500)),
                if (service.description != null)
                  Text(service.description!,
                      style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant)),
              ],
            ),
          ),
          if (isRunning) ...[
            IconButton(
              icon: const Icon(Icons.refresh, size: 15),
              onPressed: onRestart,
              tooltip: 'Restart',
              visualDensity: VisualDensity.compact,
            ),
            IconButton(
              icon: const Icon(Icons.stop_circle_outlined, size: 15),
              onPressed: onStop,
              tooltip: 'Stop',
              visualDensity: VisualDensity.compact,
            ),
          ] else
            IconButton(
              icon: const Icon(Icons.play_circle_outline, size: 15),
              onPressed: onStart,
              tooltip: 'Start',
              visualDensity: VisualDensity.compact,
            ),
        ]),
      ),
    );
  }
}
