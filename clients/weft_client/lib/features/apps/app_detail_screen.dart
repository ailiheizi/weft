import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/models/app.dart';
import '../../core/models/provider.dart';
import '../../core/providers/core_repository.dart';
import '../../core/providers/data_providers.dart';
import '../../shared/theme/spacing.dart';

class AppDetailScreen extends ConsumerWidget {
  const AppDetailScreen({super.key, required this.appName});
  final String appName;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final appAsync = ref.watch(appDetailProvider(appName));
    final providersAsync = ref.watch(providersProvider);

    return Scaffold(
      backgroundColor: Colors.transparent,
      body: appAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => _DetailError(
          error: e,
          onRetry: () => ref.invalidate(appDetailProvider(appName)),
        ),
        data: (app) => _DetailBody(
          app: app,
          providersAsync: providersAsync,
          onRefresh: () {
            ref.invalidate(appDetailProvider(appName));
            ref.invalidate(providersProvider);
          },
          onUpdateBinding: (capability, provider) async {
            await ref
                .read(coreRepositoryProvider)
                .updateBinding(appName, capability, provider);
            ref.invalidate(appDetailProvider(appName));
          },
        ),
      ),
    );
  }
}

// ── Main body ────────────────────────────────────────────────────────────────

class _DetailBody extends StatelessWidget {
  const _DetailBody({
    required this.app,
    required this.providersAsync,
    required this.onRefresh,
    required this.onUpdateBinding,
  });

  final ResolvedApp app;
  final AsyncValue<List<ProviderConfig>> providersAsync;
  final VoidCallback onRefresh;
  final Future<void> Function(String capability, String provider) onUpdateBinding;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final statusColor = switch (app.status) {
      ResolvedAppStatus.ok => const Color(0xFF10B981),
      ResolvedAppStatus.resolved => const Color(0xFF10B981),
      ResolvedAppStatus.partial => Colors.orange,
      ResolvedAppStatus.degraded => Colors.orange,
      ResolvedAppStatus.error => Colors.red,
      ResolvedAppStatus.failed => Colors.red,
      ResolvedAppStatus.unknown => Colors.grey,
    };
    final statusLabel = switch (app.status) {
      ResolvedAppStatus.ok => 'ok',
      ResolvedAppStatus.resolved => 'resolved',
      ResolvedAppStatus.partial => 'partial',
      ResolvedAppStatus.degraded => 'degraded',
      ResolvedAppStatus.error => 'error',
      ResolvedAppStatus.failed => 'failed',
      ResolvedAppStatus.unknown => 'unknown',
    };

    return ListView(
      padding: const EdgeInsets.all(Spacing.lg),
      children: [
        // ── Header row ──────────────────────────────────────────────────
        Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    app.displayName,
                    style: theme.textTheme.headlineSmall
                        ?.copyWith(fontWeight: FontWeight.w600),
                  ),
                  const SizedBox(height: 4),
                  Row(
                    children: [
                      Container(
                        width: 8,
                        height: 8,
                        decoration: BoxDecoration(
                            color: statusColor, shape: BoxShape.circle),
                      ),
                      const SizedBox(width: 6),
                      Text(
                        statusLabel,
                        style: theme.textTheme.labelSmall
                            ?.copyWith(color: statusColor),
                      ),
                      const SizedBox(width: 12),
                      Text(
                        'v${app.version}',
                        style: theme.textTheme.labelSmall?.copyWith(
                            color: theme.colorScheme.onSurfaceVariant),
                      ),
                    ],
                  ),
                  if (app.description.isNotEmpty) ...[
                    const SizedBox(height: 6),
                    Text(
                      app.description,
                      style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant),
                    ),
                  ],
                ],
              ),
            ),
            IconButton(
              icon: const Icon(Icons.refresh, size: 18),
              onPressed: onRefresh,
              tooltip: 'Refresh',
            ),
          ],
        ),
        const SizedBox(height: Spacing.lg),

        // ── Errors ──────────────────────────────────────────────────────
        if (app.errors.isNotEmpty) ...[
          _SectionHeader(
            icon: Icons.error_outline,
            label: 'Errors',
            color: theme.colorScheme.error,
          ),
          const SizedBox(height: Spacing.sm),
          ...app.errors.map((e) => _ErrorItem(message: e)),
          const SizedBox(height: Spacing.lg),
        ],

        // ── Capabilities ────────────────────────────────────────────────
        if (app.capabilities.isNotEmpty) ...[
          _SectionHeader(
            icon: Icons.bolt_outlined,
            label: 'Capabilities',
          ),
          const SizedBox(height: Spacing.sm),
          _SectionCard(
            child: Column(
              children: app.capabilities
                  .map((c) => _CapabilityRow(name: c))
                  .toList(),
            ),
          ),
          const SizedBox(height: Spacing.lg),
        ],

        // ── Bindings ────────────────────────────────────────────────────
        if (app.bindings.isNotEmpty) ...[
          _SectionHeader(
            icon: Icons.link_outlined,
            label: 'Bindings',
          ),
          const SizedBox(height: Spacing.sm),
          _SectionCard(
            child: Column(
              children: app.bindings.map((b) {
                return _BindingDetailRow(
                  binding: b,
                  providersAsync: providersAsync,
                  onUpdate: (provider) =>
                      onUpdateBinding(b.capability, provider),
                );
              }).toList(),
            ),
          ),
          const SizedBox(height: Spacing.lg),
        ],

        // ── Validation ──────────────────────────────────────────────────
        if (app.validationChecks.isNotEmpty) ...[
          _SectionHeader(
            icon: Icons.check_circle_outline,
            label: 'Validation',
          ),
          const SizedBox(height: Spacing.sm),
          _SectionCard(
            child: Column(
              children: app.validationChecks
                  .map((v) => _ValidationRow(check: v))
                  .toList(),
            ),
          ),
          const SizedBox(height: Spacing.lg),
        ],
      ],
    );
  }
}

// ── Section header ───────────────────────────────────────────────────────────

class _SectionHeader extends StatelessWidget {
  const _SectionHeader({
    required this.icon,
    required this.label,
    this.color,
  });

  final IconData icon;
  final String label;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final c = color ?? theme.colorScheme.onSurfaceVariant;
    return Row(
      children: [
        Icon(icon, size: 14, color: c),
        const SizedBox(width: 6),
        Text(
          label,
          style: theme.textTheme.titleSmall?.copyWith(color: c),
        ),
      ],
    );
  }
}

// ── Section card wrapper ─────────────────────────────────────────────────────

class _SectionCard extends StatelessWidget {
  const _SectionCard({required this.child});
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(
          color: theme.colorScheme.outline.withValues(alpha: 0.2),
        ),
      ),
      child: child,
    );
  }
}

// ── Capability row ───────────────────────────────────────────────────────────

class _CapabilityRow extends StatelessWidget {
  const _CapabilityRow({required this.name});
  final String name;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(
          horizontal: Spacing.md, vertical: Spacing.sm),
      child: Row(
        children: [
          Icon(Icons.bolt, size: 14, color: theme.colorScheme.primary),
          const SizedBox(width: 8),
          Text(
            name,
            style: theme.textTheme.bodySmall?.copyWith(
              fontFamily: 'monospace',
              color: theme.colorScheme.onSurface,
            ),
          ),
        ],
      ),
    );
  }
}

// ── Binding detail row ───────────────────────────────────────────────────────

class _BindingDetailRow extends StatefulWidget {
  const _BindingDetailRow({
    required this.binding,
    required this.providersAsync,
    required this.onUpdate,
  });

  final AppBindingResolution binding;
  final AsyncValue<List<ProviderConfig>> providersAsync;
  final Future<void> Function(String provider) onUpdate;

  @override
  State<_BindingDetailRow> createState() => _BindingDetailRowState();
}

class _BindingDetailRowState extends State<_BindingDetailRow> {
  bool _saving = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final b = widget.binding;

    return Padding(
      padding: const EdgeInsets.symmetric(
          horizontal: Spacing.md, vertical: Spacing.sm),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Capability name + source tag
          Row(
            children: [
              Text(
                b.capability,
                style: theme.textTheme.bodySmall?.copyWith(
                  fontFamily: 'monospace',
                  color: theme.colorScheme.primary,
                  fontWeight: FontWeight.w500,
                ),
              ),
              const SizedBox(width: 8),
              if (b.source.isNotEmpty)
                _Tag(label: b.source),
              if (b.mutable) ...[
                const SizedBox(width: 4),
                _Tag(
                  label: 'mutable',
                  color: theme.colorScheme.tertiary,
                ),
              ],
            ],
          ),
          const SizedBox(height: 6),
          // Provider selector or static display
          if (b.mutable)
            widget.providersAsync.when(
              loading: () => const SizedBox(
                height: 16,
                width: 16,
                child: CircularProgressIndicator(strokeWidth: 1.5),
              ),
              error: (_, _e) => Text(
                b.provider,
                style: theme.textTheme.bodySmall,
              ),
              data: (providerList) => _ProviderDropdown(
                current: b.provider,
                providers: providerList,
                saving: _saving,
                onChanged: (newProvider) async {
                  if (newProvider == null || newProvider == b.provider) return;
                  setState(() => _saving = true);
                  try {
                    await widget.onUpdate(newProvider);
                  } finally {
                    if (mounted) setState(() => _saving = false);
                  }
                },
              ),
            )
          else
            Row(
              children: [
                Icon(Icons.arrow_forward,
                    size: 12,
                    color: theme.colorScheme.onSurfaceVariant),
                const SizedBox(width: 6),
                Text(
                  b.provider,
                  style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurface),
                ),
              ],
            ),
        ],
      ),
    );
  }
}

// ── Provider dropdown ────────────────────────────────────────────────────────

class _ProviderDropdown extends StatelessWidget {
  const _ProviderDropdown({
    required this.current,
    required this.providers,
    required this.saving,
    required this.onChanged,
  });

  final String current;
  final List<ProviderConfig> providers;
  final bool saving;
  final ValueChanged<String?> onChanged;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    // Ensure current value is in the list; if not, add a placeholder entry
    final names = providers.map((p) => p.name).toList();
    if (!names.contains(current) && current.isNotEmpty) {
      names.insert(0, current);
    }

    return Row(
      children: [
        Expanded(
          child: DropdownButtonFormField<String>(
            value: names.contains(current) ? current : null,
            isDense: true,
            decoration: InputDecoration(
              contentPadding: const EdgeInsets.symmetric(
                  horizontal: 10, vertical: 6),
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(6),
                borderSide: BorderSide(
                    color: theme.colorScheme.outline.withValues(alpha: 0.4)),
              ),
              enabledBorder: OutlineInputBorder(
                borderRadius: BorderRadius.circular(6),
                borderSide: BorderSide(
                    color: theme.colorScheme.outline.withValues(alpha: 0.4)),
              ),
            ),
            items: names
                .map((n) => DropdownMenuItem(value: n, child: Text(n)))
                .toList(),
            onChanged: saving ? null : onChanged,
          ),
        ),
        if (saving) ...[
          const SizedBox(width: 8),
          const SizedBox(
            width: 14,
            height: 14,
            child: CircularProgressIndicator(strokeWidth: 1.5),
          ),
        ],
      ],
    );
  }
}

// ── Validation row ───────────────────────────────────────────────────────────

class _ValidationRow extends StatelessWidget {
  const _ValidationRow({required this.check});
  final String check;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(
          horizontal: Spacing.md, vertical: Spacing.sm),
      child: Row(
        children: [
          Icon(Icons.check_circle_outline,
              size: 14, color: const Color(0xFF10B981)),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              check,
              style: theme.textTheme.bodySmall,
            ),
          ),
        ],
      ),
    );
  }
}

// ── Error item ───────────────────────────────────────────────────────────────

class _ErrorItem extends StatelessWidget {
  const _ErrorItem({required this.message});
  final String message;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      margin: const EdgeInsets.only(bottom: 6),
      padding: const EdgeInsets.symmetric(
          horizontal: Spacing.md, vertical: Spacing.sm),
      decoration: BoxDecoration(
        color: theme.colorScheme.errorContainer.withValues(alpha: 0.4),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
            color: theme.colorScheme.error.withValues(alpha: 0.3)),
      ),
      child: Row(
        children: [
          Icon(Icons.warning_amber_outlined,
              size: 14, color: theme.colorScheme.error),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              message,
              style: theme.textTheme.bodySmall
                  ?.copyWith(color: theme.colorScheme.onErrorContainer),
            ),
          ),
        ],
      ),
    );
  }
}

// ── Tag chip ─────────────────────────────────────────────────────────────────

class _Tag extends StatelessWidget {
  const _Tag({required this.label, this.color});
  final String label;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final c = color ?? theme.colorScheme.onSurfaceVariant;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: c.withValues(alpha: 0.12),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        label,
        style: theme.textTheme.labelSmall?.copyWith(color: c),
      ),
    );
  }
}

// ── Full-page error ──────────────────────────────────────────────────────────

class _DetailError extends StatelessWidget {
  const _DetailError({required this.error, required this.onRetry});
  final Object error;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(Spacing.lg),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.error_outline,
                size: 40, color: theme.colorScheme.error),
            const SizedBox(height: 12),
            Text(
              'Failed to load app details',
              style: theme.textTheme.titleMedium,
            ),
            const SizedBox(height: 6),
            Text(
              error.toString(),
              style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant),
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 16),
            FilledButton.icon(
              icon: const Icon(Icons.refresh, size: 16),
              label: const Text('Retry'),
              onPressed: onRetry,
            ),
          ],
        ),
      ),
    );
  }
}
