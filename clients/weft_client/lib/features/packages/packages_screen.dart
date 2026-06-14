import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/providers/data_providers.dart';
import '../../core/providers/core_repository.dart';
import '../../core/models/package.dart';
import '../../shared/widgets/app_error_widget.dart';
import '../../shared/widgets/skeleton.dart';
import '../../shared/widgets/hover_card.dart';
import '../../shared/widgets/empty_state.dart';
import '../../shared/theme/spacing.dart';
import 'import_package_dialog.dart';
import 'online_install_dialog.dart';
import 'package_config_dialog.dart';

class PackagesScreen extends ConsumerWidget {
  const PackagesScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final packages = ref.watch(packagesProvider);
    final theme = Theme.of(context);

    return Scaffold(
      backgroundColor: Colors.transparent,
      body: ListView(
        padding: const EdgeInsets.all(Spacing.lg),
        children: [
          Row(children: [
            Text('Packages',
                style: theme.textTheme.headlineSmall
                    ?.copyWith(fontWeight: FontWeight.w600)),
            const Spacer(),
            FilledButton.icon(
              onPressed: () => OnlineInstallDialog.show(context),
              icon: const Icon(Icons.cloud_download_outlined, size: 15),
              label: const Text('在线安装'),
            ),
            const SizedBox(width: 8),
            OutlinedButton.icon(
              onPressed: () => ImportPackageDialog.show(context),
              icon: const Icon(Icons.download_outlined, size: 15),
              label: const Text('导入本地包'),
            ),
            const SizedBox(width: 8),
            IconButton(
              icon: const Icon(Icons.refresh, size: 16),
              onPressed: () => ref.invalidate(packagesProvider),
              tooltip: 'Refresh',
            ),
          ]),
          const SizedBox(height: Spacing.lg),
          packages.when(
            data: (list) => list.isEmpty
                ? const EmptyState(
                    icon: Icons.extension_outlined,
                    title: 'No packages installed',
                    subtitle:
                        'Install packages via weft-core to see them here.',
                  )
                : Column(
                    children: list
                        .map((p) => _PackageTile(
                            package: p,
                            onToggle: () async {
                              await ref
                                  .read(coreRepositoryProvider)
                                  .togglePackage(p.name);
                              ref.invalidate(packagesProvider);
                            },
                            onReload: () async {
                              await ref
                                  .read(coreRepositoryProvider)
                                  .reloadPackage(p.name);
                              ref.invalidate(packagesProvider);
                            },
                            onConfigure: () async {
                              await showDialog<void>(
                                context: context,
                                builder: (_) =>
                                    PackageConfigDialog(packageName: p.name),
                              );
                            }))
                        .toList()),
            loading: () => const SkeletonList(count: 4),
            error: (e, _) => AppErrorWidget(
              error: e,
              onRetry: () => ref.invalidate(packagesProvider),
            ),
          ),
        ],
      ),
    );
  }
}

class _PackageTile extends StatelessWidget {
  const _PackageTile({
    required this.package,
    required this.onToggle,
    required this.onReload,
    required this.onConfigure,
  });
  final PackageInfo package;
  final VoidCallback onToggle;
  final VoidCallback onReload;
  final VoidCallback onConfigure;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return HoverCard(
      child: Padding(
        padding: const EdgeInsets.symmetric(
            horizontal: Spacing.md, vertical: Spacing.md - 4),
        child: Row(children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(children: [
                  Text(package.name,
                      style: theme.textTheme.bodyMedium
                          ?.copyWith(fontWeight: FontWeight.w500)),
                  if (package.version != null) ...[
                    const SizedBox(width: Spacing.sm),
                    Text('v${package.version}',
                        style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant)),
                  ],
                  const SizedBox(width: Spacing.sm),
                  _RuntimeChip(runtime: package.runtime),
                ]),
                if (package.description != null)
                  Padding(
                    padding: const EdgeInsets.only(top: 2),
                    child: Text(package.description!,
                        style: theme.textTheme.bodySmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant)),
                  ),
              ],
            ),
          ),
          IconButton(
            icon: const Icon(Icons.tune, size: 15),
            onPressed: onConfigure,
            tooltip: 'Configure',
            visualDensity: VisualDensity.compact,
          ),
          IconButton(
            icon: const Icon(Icons.refresh, size: 15),
            onPressed: onReload,
            tooltip: 'Reload',
            visualDensity: VisualDensity.compact,
          ),
          Switch(
            value: package.enabled,
            onChanged: (_) => onToggle(),
            materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
          ),
        ]),
      ),
    );
  }
}

class _RuntimeChip extends StatelessWidget {
  const _RuntimeChip({required this.runtime});
  final PackageRuntime runtime;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(
          horizontal: Spacing.sm - 2, vertical: 2),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(3),
        border: Border.all(color: Theme.of(context).colorScheme.outline),
      ),
      child: Text(runtime.name.toUpperCase(),
          style: TextStyle(
              fontSize: 10,
              color: Theme.of(context).colorScheme.onSurfaceVariant,
              fontWeight: FontWeight.w500)),
    );
  }
}
