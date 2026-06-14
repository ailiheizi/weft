import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/providers/data_providers.dart';
import '../../core/providers/core_repository.dart';
import '../../core/models/provider.dart';
import '../../shared/widgets/skeleton.dart';
import '../../shared/widgets/hover_card.dart';
import '../../shared/widgets/empty_state.dart';
import '../../shared/theme/app_theme.dart';
import '../../shared/theme/spacing.dart';
import '../../shared/widgets/app_error_widget.dart';

class ProvidersScreen extends ConsumerWidget {
  const ProvidersScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final providers = ref.watch(providersProvider);
    final theme = Theme.of(context);

    return Scaffold(
      backgroundColor: Colors.transparent,
      body: ListView(
        padding: const EdgeInsets.all(Spacing.lg),
        children: [
          Row(children: [
            Text('Providers',
                style: theme.textTheme.headlineSmall
                    ?.copyWith(fontWeight: FontWeight.w600)),
            const Spacer(),
            FilledButton.icon(
              icon: const Icon(Icons.add, size: 16),
              label: const Text('Add Provider'),
              onPressed: () => _showProviderDialog(context, ref, null),
            ),
          ]),
          const SizedBox(height: Spacing.lg),
          providers.when(
            data: (list) => list.isEmpty
                ? EmptyState(
                    icon: Icons.bolt_outlined,
                    title: 'No providers configured',
                    subtitle: 'Add an AI provider to get started.',
                    action: FilledButton.icon(
                      icon: const Icon(Icons.add, size: 16),
                      label: const Text('Add Provider'),
                      onPressed: () => _showProviderDialog(context, ref, null),
                    ),
                  )
                : Column(
                    children: list
                        .map((p) => _ProviderCard(
                            provider: p,
                            onEdit: () =>
                                _showProviderDialog(context, ref, p),
                            onDelete: () async {
                              await ref
                                  .read(coreRepositoryProvider)
                                  .deleteProvider(p.name);
                              ref.invalidate(providersProvider);
                            }))
                        .toList()),
            loading: () => const SkeletonList(count: 3),
            error: (e, _) => AppErrorWidget(
              error: e,
              onRetry: () => ref.invalidate(providersProvider),
            ),
          ),
        ],
      ),
    );
  }

  void _showProviderDialog(
      BuildContext context, WidgetRef ref, ProviderConfig? existing) {
    showDialog(
      context: context,
      builder: (_) => _ProviderDialog(
        existing: existing,
        onSave: (config) async {
          final repo = ref.read(coreRepositoryProvider);
          if (existing != null) {
            await repo.updateProvider(existing.name, config);
          } else {
            await repo.createProvider(config);
          }
          ref.invalidate(providersProvider);
        },
      ),
    );
  }
}

class _ProviderCard extends StatelessWidget {
  const _ProviderCard({
    required this.provider,
    required this.onEdit,
    required this.onDelete,
  });
  final ProviderConfig provider;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final surfaces = theme.extension<AppSurfaces>()!;
    return HoverCard(
      child: Padding(
        padding: const EdgeInsets.all(Spacing.md),
        child: Row(children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(children: [
                  Text(provider.name,
                      style: theme.textTheme.bodyMedium
                          ?.copyWith(fontWeight: FontWeight.w500)),
                  const SizedBox(width: Spacing.sm),
                  _FormatChip(format: provider.format),
                ]),
                const SizedBox(height: 2),
                Text(provider.baseUrl,
                    style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant)),
                const SizedBox(height: 2),
                Text(
                  '${provider.keys.length} key${provider.keys.length == 1 ? '' : 's'} configured',
                  style: theme.textTheme.bodySmall?.copyWith(
                      color: provider.keys.isEmpty
                          ? theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.5)
                          : theme.colorScheme.primary),
                ),
                if (provider.models.isNotEmpty) ...[
                  const SizedBox(height: Spacing.sm - 2),
                  Wrap(
                    spacing: Spacing.xs,
                    runSpacing: Spacing.xs,
                    children: provider.models
                        .take(3)
                        .map((m) => Chip(
                              label: Text(m),
                              visualDensity: VisualDensity.compact,
                              padding: EdgeInsets.zero,
                            ))
                        .toList(),
                  ),
                ],
              ],
            ),
          ),
          IconButton(
            icon: const Icon(Icons.edit_outlined, size: 15),
            onPressed: onEdit,
            tooltip: 'Edit',
            visualDensity: VisualDensity.compact,
          ),
          IconButton(
            icon: const Icon(Icons.delete_outline, size: 15),
            onPressed: onDelete,
            tooltip: 'Delete',
            visualDensity: VisualDensity.compact,
            color: surfaces.statusError,
          ),
        ]),
      ),
    );
  }
}

class _FormatChip extends StatelessWidget {
  const _FormatChip({required this.format});
  final String format;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: Theme.of(context).colorScheme.outline),
      ),
      child: Text(format.toUpperCase(),
          style: TextStyle(
              fontSize: 10,
              color: Theme.of(context).colorScheme.onSurfaceVariant,
              fontWeight: FontWeight.w500)),
    );
  }
}

class _ProviderDialog extends StatefulWidget {
  const _ProviderDialog({this.existing, required this.onSave});
  final ProviderConfig? existing;
  final Future<void> Function(ProviderConfig) onSave;

  @override
  State<_ProviderDialog> createState() => _ProviderDialogState();
}

// Holds mutable state for one API key row in the dialog
class _KeyEntry {
  final TextEditingController labelCtrl;
  final TextEditingController keyCtrl;
  bool enabled;
  bool obscure;

  _KeyEntry({
    String label = '',
    String key = '',
    this.enabled = true,
  })  : obscure = true,
        labelCtrl = TextEditingController(text: label),
        keyCtrl = TextEditingController(text: key);

  void dispose() {
    labelCtrl.dispose();
    keyCtrl.dispose();
  }
}

class _ProviderDialogState extends State<_ProviderDialog> {
  late final _nameCtrl =
      TextEditingController(text: widget.existing?.name ?? '');
  late final _urlCtrl =
      TextEditingController(text: widget.existing?.baseUrl ?? '');
  late final _modelsCtrl =
      TextEditingController(text: widget.existing?.models.join(', ') ?? '');
  late String _format = widget.existing?.format ?? 'openai';
  late final List<_KeyEntry> _keys;
  bool _saving = false;

  @override
  void initState() {
    super.initState();
    _keys = (widget.existing?.keys ?? [])
        .map((k) => _KeyEntry(
              label: k.label ?? '',
              key: k.key,
              enabled: k.enabled,
            ))
        .toList();
  }

  @override
  void dispose() {
    _nameCtrl.dispose();
    _urlCtrl.dispose();
    _modelsCtrl.dispose();
    for (final e in _keys) {
      e.dispose();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return AlertDialog(
      title: Text(widget.existing != null ? 'Edit Provider' : 'Add Provider'),
      content: SizedBox(
        width: 480,
        child: SingleChildScrollView(
          child: Column(mainAxisSize: MainAxisSize.min, children: [
            TextField(
              controller: _nameCtrl,
              decoration: const InputDecoration(labelText: 'Name'),
              enabled: widget.existing == null,
            ),
            const SizedBox(height: 12),
            TextField(
              controller: _urlCtrl,
              decoration: const InputDecoration(labelText: 'Base URL'),
            ),
            const SizedBox(height: 12),
            DropdownButtonFormField<String>(
              value: _format,
              decoration: const InputDecoration(labelText: 'Format'),
              items: const [
                DropdownMenuItem(value: 'openai', child: Text('OpenAI')),
                DropdownMenuItem(value: 'anthropic', child: Text('Anthropic')),
              ],
              onChanged: (v) => setState(() => _format = v!),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: _modelsCtrl,
              decoration:
                  const InputDecoration(labelText: 'Models (comma separated)'),
            ),
            const SizedBox(height: 20),
            // ── API Keys section ──────────────────────────────────────
            Align(
              alignment: Alignment.centerLeft,
              child: Text('API Keys',
                  style: theme.textTheme.titleSmall
                      ?.copyWith(fontWeight: FontWeight.w600)),
            ),
            const SizedBox(height: 8),
            if (_keys.isEmpty)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 4),
                child: Text(
                  'No keys configured',
                  style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant),
                ),
              ),
            ..._keys.asMap().entries.map((entry) {
              final i = entry.key;
              final e = entry.value;
              return Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.center,
                  children: [
                    // Label field
                    SizedBox(
                      width: 100,
                      child: TextField(
                        controller: e.labelCtrl,
                        decoration: const InputDecoration(
                          labelText: 'Label',
                          isDense: true,
                        ),
                      ),
                    ),
                    const SizedBox(width: 8),
                    // Key field
                    Expanded(
                      child: StatefulBuilder(
                        builder: (ctx, setLocal) => TextField(
                          controller: e.keyCtrl,
                          obscureText: e.obscure,
                          decoration: InputDecoration(
                            labelText: 'API Key',
                            isDense: true,
                            suffixIcon: IconButton(
                              icon: Icon(
                                e.obscure
                                    ? Icons.visibility_outlined
                                    : Icons.visibility_off_outlined,
                                size: 16,
                              ),
                              onPressed: () {
                                setLocal(() => e.obscure = !e.obscure);
                              },
                              visualDensity: VisualDensity.compact,
                            ),
                          ),
                        ),
                      ),
                    ),
                    const SizedBox(width: 4),
                    // Enable switch
                    StatefulBuilder(
                      builder: (ctx, setLocal) => Switch(
                        value: e.enabled,
                        onChanged: (v) {
                          setLocal(() => e.enabled = v);
                        },
                        materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                      ),
                    ),
                    // Delete button
                    IconButton(
                      icon: const Icon(Icons.delete_outline, size: 16),
                      color: theme.extension<AppSurfaces>()!.statusError,
                      visualDensity: VisualDensity.compact,
                      onPressed: () {
                        setState(() {
                          _keys.removeAt(i).dispose();
                        });
                      },
                    ),
                  ],
                ),
              );
            }),
            const SizedBox(height: 4),
            Align(
              alignment: Alignment.centerLeft,
              child: TextButton.icon(
                icon: const Icon(Icons.add, size: 16),
                label: const Text('Add Key'),
                onPressed: () => setState(() => _keys.add(_KeyEntry())),
              ),
            ),
          ]),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: _saving ? null : _save,
          child: _saving
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2))
              : const Text('Save'),
        ),
      ],
    );
  }

  Future<void> _save() async {
    setState(() => _saving = true);
    final models = _modelsCtrl.text
        .split(',')
        .map((s) => s.trim())
        .where((s) => s.isNotEmpty)
        .toList();
    final keys = _keys
        .where((e) => e.keyCtrl.text.trim().isNotEmpty)
        .map((e) => ApiKeyConfig(
              key: e.keyCtrl.text.trim(),
              label: e.labelCtrl.text.trim().isEmpty
                  ? null
                  : e.labelCtrl.text.trim(),
              enabled: e.enabled,
            ))
        .toList();
    final config = ProviderConfig(
      name: _nameCtrl.text.trim(),
      baseUrl: _urlCtrl.text.trim(),
      format: _format,
      models: models,
      keys: keys,
    );
    await widget.onSave(config);
    if (mounted) Navigator.pop(context);
  }
}
