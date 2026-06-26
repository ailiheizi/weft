import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/providers/data_providers.dart';
import '../../core/providers/core_repository.dart';
import '../../core/models/provider.dart';
import '../../core/models/provider_presets.dart';
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
      BuildContext context, WidgetRef ref, ProviderConfig? existing) async {
    // 编辑时:从后端拉完整 provider(含 keys),列表接口只给 key_count。
    var full = existing;
    if (existing != null) {
      try {
        full = await ref.read(coreRepositoryProvider).getProvider(existing.name);
      } catch (_) {
        full = existing; // 拉取失败退回列表数据(keys 可能为空)。
      }
    }
    if (!context.mounted) return;
    showDialog(
      context: context,
      builder: (_) => _ProviderDialog(
        existing: full,
        onSave: (config) async {
          final repo = ref.read(coreRepositoryProvider);
          if (existing != null) {
            await repo.updateProvider(existing.name, config);
          } else {
            await repo.createProvider(config);
          }
          ref.invalidate(providersProvider);
        },
        onFetchModels: (baseUrl, apiKey, format) => ref
            .read(coreRepositoryProvider)
            .fetchModels(baseUrl: baseUrl, apiKey: apiKey, format: format),
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
                  '${provider.keyCount} key${provider.keyCount == 1 ? '' : 's'} configured',
                  style: theme.textTheme.bodySmall?.copyWith(
                      color: provider.keyCount == 0
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
  const _ProviderDialog({
    this.existing,
    required this.onSave,
    required this.onFetchModels,
  });
  final ProviderConfig? existing;
  final Future<void> Function(ProviderConfig) onSave;

  /// 从 provider 拉取模型(base_url, apiKey, format)→ 模型 id 列表。
  final Future<List<String>> Function(String baseUrl, String apiKey,
      String format) onFetchModels;

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
  late String _format = widget.existing?.format ?? 'openai';
  late final List<_KeyEntry> _keys;
  bool _saving = false;
  String? _error;

  /// 已选模型(多选)。可用模型 = 已选 ∪ 拉取到的。
  late final Set<String> _selectedModels =
      (widget.existing?.models ?? const <String>[]).toSet();
  final Set<String> _availableModels = {};
  bool _fetchingModels = false;
  String? _fetchError;
  String _modelQuery = '';

  Future<void> _fetchModels() async {
    setState(() {
      _fetchingModels = true;
      _fetchError = null;
    });
    try {
      final key = _keys.isNotEmpty ? _keys.first.keyCtrl.text.trim() : '';
      final models = await widget.onFetchModels(
          _urlCtrl.text.trim(), key, _format);
      setState(() {
        _availableModels
          ..clear()
          ..addAll(models);
        _fetchingModels = false;
        if (models.isEmpty) _fetchError = '未获取到模型(检查 URL / Key)';
      });
    } catch (e) {
      setState(() {
        _fetchingModels = false;
        _fetchError = e.toString().length > 80
            ? '${e.toString().substring(0, 80)}…'
            : e.toString();
      });
    }
  }

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
            if (widget.existing == null) ...[
              Align(
                alignment: Alignment.centerLeft,
                child: Text('Quick start with a preset',
                    style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant)),
              ),
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: kProviderPresets.map((p) {
                  return ActionChip(
                    label: Text(p.name),
                    onPressed: () => _applyPreset(p),
                  );
                }).toList(),
              ),
              const SizedBox(height: 16),
            ],
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
            // ── Models(多选 chips + 手动获取)──────────────────────────────
            Row(
              children: [
                Text('Models',
                    style: theme.textTheme.titleSmall
                        ?.copyWith(fontWeight: FontWeight.w600)),
                const Spacer(),
                TextButton.icon(
                  onPressed: _fetchingModels ? null : _fetchModels,
                  icon: _fetchingModels
                      ? const SizedBox(
                          width: 14,
                          height: 14,
                          child: CircularProgressIndicator(strokeWidth: 2))
                      : const Icon(Icons.download_outlined, size: 16),
                  label: const Text('获取模型'),
                ),
              ],
            ),
            if (_fetchError != null)
              Padding(
                padding: const EdgeInsets.only(bottom: 4),
                child: Text(_fetchError!,
                    style: theme.textTheme.bodySmall
                        ?.copyWith(color: theme.colorScheme.error)),
              ),
            Builder(builder: (context) {
              final all = {..._selectedModels, ..._availableModels}.toList()
                ..sort();
              if (all.isEmpty && _modelQuery.isEmpty) {
                return Padding(
                  padding: const EdgeInsets.symmetric(vertical: 6),
                  child: Text('点「获取模型」从 provider 拉取,或在搜索框输入模型名手动添加',
                      style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant)),
                );
              }
              final q = _modelQuery.trim().toLowerCase();
              final filtered = q.isEmpty
                  ? all
                  : all.where((m) => m.toLowerCase().contains(q)).toList();
              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  // 搜索/手动输入框
                  TextField(
                    onChanged: (v) => setState(() => _modelQuery = v),
                    style: const TextStyle(fontSize: 13),
                    decoration: InputDecoration(
                      isDense: true,
                      prefixIcon: const Icon(Icons.search, size: 16),
                      hintText: '搜索或输入模型名后回车添加…',
                      contentPadding: const EdgeInsets.symmetric(
                          horizontal: 8, vertical: 8),
                      border: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(8)),
                      suffixIcon: q.isNotEmpty &&
                              !all.contains(_modelQuery.trim())
                          ? IconButton(
                              icon: const Icon(Icons.add, size: 16),
                              tooltip: '添加',
                              onPressed: () => setState(() {
                                _selectedModels.add(_modelQuery.trim());
                                _modelQuery = '';
                              }),
                            )
                          : null,
                    ),
                    onSubmitted: (v) {
                      final trimmed = v.trim();
                      if (trimmed.isNotEmpty) {
                        setState(() {
                          _selectedModels.add(trimmed);
                          _modelQuery = '';
                        });
                      }
                    },
                  ),
                  const SizedBox(height: 8),
                  // 已选 + 可选(可滚动,限高)
                  ConstrainedBox(
                    constraints: const BoxConstraints(maxHeight: 160),
                    child: SingleChildScrollView(
                      child: Wrap(
                        spacing: 6,
                        runSpacing: 6,
                        children: [
                          for (final m in filtered)
                            FilterChip(
                              label:
                                  Text(m, style: const TextStyle(fontSize: 12)),
                              selected: _selectedModels.contains(m),
                              onSelected: (sel) => setState(() {
                                if (sel) {
                                  _selectedModels.add(m);
                                } else {
                                  _selectedModels.remove(m);
                                }
                              }),
                            ),
                        ],
                      ),
                    ),
                  ),
                ],
              );
            }),
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
                    // Reorder buttons
                    Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        SizedBox(
                          height: 18,
                          width: 18,
                          child: IconButton(
                            padding: EdgeInsets.zero,
                            icon: Icon(Icons.arrow_drop_up,
                                size: 18,
                                color: i == 0
                                    ? theme.colorScheme.onSurface
                                        .withValues(alpha: 0.2)
                                    : theme.colorScheme.onSurface),
                            onPressed: i == 0
                                ? null
                                : () {
                                    setState(() {
                                      final item = _keys.removeAt(i);
                                      _keys.insert(i - 1, item);
                                    });
                                  },
                            visualDensity: VisualDensity.compact,
                          ),
                        ),
                        SizedBox(
                          height: 18,
                          width: 18,
                          child: IconButton(
                            padding: EdgeInsets.zero,
                            icon: Icon(Icons.arrow_drop_down,
                                size: 18,
                                color: i == _keys.length - 1
                                    ? theme.colorScheme.onSurface
                                        .withValues(alpha: 0.2)
                                    : theme.colorScheme.onSurface),
                            onPressed: i == _keys.length - 1
                                ? null
                                : () {
                                    setState(() {
                                      final item = _keys.removeAt(i);
                                      _keys.insert(i + 1, item);
                                    });
                                  },
                            visualDensity: VisualDensity.compact,
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(width: 4),
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
            if (_error != null) ...[
              const SizedBox(height: 8),
              Align(
                alignment: Alignment.centerLeft,
                child: Text(
                  _error!,
                  style: TextStyle(
                      color: theme.colorScheme.error, fontSize: 13),
                ),
              ),
            ],
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

  void _applyPreset(ProviderPreset p) {
    setState(() {
      if (_nameCtrl.text.trim().isEmpty) _nameCtrl.text = p.name;
      _urlCtrl.text = p.baseUrl;
      if (p.defaultModel.isNotEmpty) _selectedModels.add(p.defaultModel);
      _format = p.format;
      _error = null;
    });
  }

  Future<void> _save() async {
    setState(() {
      _saving = true;
      _error = null;
    });
    final models = _selectedModels.toList()..sort();
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
    try {
      await widget.onSave(config);
      if (mounted) Navigator.pop(context);
    } catch (e) {
      // Don't leave the dialog spinning forever on a failed save — surface the
      // error and let the user retry.
      if (mounted) {
        setState(() {
          _saving = false;
          _error = 'Save failed: $e';
        });
      }
    }
  }
}
