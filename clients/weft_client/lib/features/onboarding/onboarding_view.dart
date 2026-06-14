import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/models/provider.dart';
import '../../core/providers/core_repository.dart';
import '../../core/providers/data_providers.dart';
import '../../core/providers/preferences_provider.dart';

/// A built-in provider preset shown on the OOBE provider step.
class _ProviderPreset {
  const _ProviderPreset({
    required this.name,
    required this.baseUrl,
    required this.format,
    required this.defaultModel,
    required this.keyHint,
    this.docsUrl,
  });

  final String name;
  final String baseUrl;
  final String format; // 'openai' | 'anthropic'
  final String defaultModel;
  final String keyHint;
  final String? docsUrl;
}

const _presets = <_ProviderPreset>[
  _ProviderPreset(
    name: 'DeepSeek',
    baseUrl: 'https://api.deepseek.com',
    format: 'openai',
    defaultModel: 'deepseek-chat',
    keyHint: 'sk-...',
    docsUrl: 'https://platform.deepseek.com/api_keys',
  ),
  _ProviderPreset(
    name: 'OpenAI',
    baseUrl: 'https://api.openai.com/v1',
    format: 'openai',
    defaultModel: 'gpt-4o',
    keyHint: 'sk-...',
    docsUrl: 'https://platform.openai.com/api-keys',
  ),
  _ProviderPreset(
    name: 'OpenRouter',
    baseUrl: 'https://openrouter.ai/api/v1',
    format: 'openai',
    defaultModel: 'openai/gpt-4o',
    keyHint: 'sk-or-...',
    docsUrl: 'https://openrouter.ai/keys',
  ),
  _ProviderPreset(
    name: 'Anthropic',
    baseUrl: 'https://api.anthropic.com',
    format: 'anthropic',
    defaultModel: 'claude-sonnet-4-20250514',
    keyHint: 'sk-ant-...',
    docsUrl: 'https://console.anthropic.com/settings/keys',
  ),
];

/// Full-screen first-run onboarding (OOBE).
///
/// Step 1: welcome + what Weft does.
/// Step 2: pick a provider preset, paste an API key (and optionally a model).
/// On finish it creates the provider via the core API and marks onboarding done.
class OnboardingView extends ConsumerStatefulWidget {
  const OnboardingView({super.key, required this.onComplete});

  /// Called once the user has finished (or skipped) onboarding.
  final VoidCallback onComplete;

  @override
  ConsumerState<OnboardingView> createState() => _OnboardingViewState();
}

class _OnboardingViewState extends ConsumerState<OnboardingView> {
  int _step = 0; // 0 = welcome, 1 = provider
  _ProviderPreset _selected = _presets.first;
  final _keyCtrl = TextEditingController();
  final _modelCtrl = TextEditingController(text: _presets.first.defaultModel);
  bool _saving = false;
  String? _error;

  @override
  void dispose() {
    _keyCtrl.dispose();
    _modelCtrl.dispose();
    super.dispose();
  }

  void _pick(_ProviderPreset p) {
    setState(() {
      _selected = p;
      _modelCtrl.text = p.defaultModel;
      _error = null;
    });
  }

  Future<void> _finish() async {
    final key = _keyCtrl.text.trim();
    if (key.isEmpty) {
      setState(() => _error = 'Please paste an API key to continue.');
      return;
    }
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      final model = _modelCtrl.text.trim();
      final config = ProviderConfig(
        name: _selected.name,
        baseUrl: _selected.baseUrl,
        format: _selected.format,
        models: model.isEmpty ? const [] : [model],
        keys: [ApiKeyConfig(key: key)],
      );
      await ref.read(coreRepositoryProvider).createProvider(config);
      ref.invalidate(providersProvider);
      await ref
          .read(preferencesProvider.notifier)
          .setOnboardingCompleted(true);
      widget.onComplete();
    } catch (e) {
      setState(() {
        _saving = false;
        _error = 'Could not save provider: $e';
      });
    }
  }

  Future<void> _skip() async {
    await ref.read(preferencesProvider.notifier).setOnboardingCompleted(true);
    widget.onComplete();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Scaffold(
      body: Container(
        decoration: BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
            colors: [
              theme.colorScheme.surface,
              theme.colorScheme.primary.withValues(alpha: 0.06),
            ],
          ),
        ),
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 560),
            child: Padding(
              padding: const EdgeInsets.all(40),
              child: _step == 0 ? _buildWelcome(theme) : _buildProvider(theme),
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildWelcome(ThemeData theme) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(Icons.auto_awesome,
            size: 56, color: theme.colorScheme.primary),
        const SizedBox(height: 24),
        Text('Welcome to Weft',
            style: theme.textTheme.headlineMedium
                ?.copyWith(fontWeight: FontWeight.w700)),
        const SizedBox(height: 12),
        Text(
          'A modular AI agent platform. Chat with multiple LLM providers, run '
          'multi-agent teams, use tools, and extend it with packages — all '
          'driven by a local core that runs on your machine.',
          textAlign: TextAlign.center,
          style: theme.textTheme.bodyMedium
              ?.copyWith(color: theme.colorScheme.onSurfaceVariant),
        ),
        const SizedBox(height: 16),
        _StepDots(count: 2, active: 0),
        const SizedBox(height: 28),
        SizedBox(
          width: double.infinity,
          child: FilledButton(
            onPressed: () => setState(() => _step = 1),
            child: const Padding(
              padding: EdgeInsets.symmetric(vertical: 12),
              child: Text('Get started'),
            ),
          ),
        ),
        TextButton(
          onPressed: _skip,
          child: const Text('Skip for now'),
        ),
      ],
    );
  }

  Widget _buildProvider(ThemeData theme) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('Connect an AI provider',
            style: theme.textTheme.headlineSmall
                ?.copyWith(fontWeight: FontWeight.w700)),
        const SizedBox(height: 6),
        Text(
          'Weft needs at least one LLM provider and API key. Pick a provider '
          'and paste your key — you can add more later in Settings.',
          style: theme.textTheme.bodySmall
              ?.copyWith(color: theme.colorScheme.onSurfaceVariant),
        ),
        const SizedBox(height: 20),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: _presets.map((p) {
            final selected = p.name == _selected.name;
            return ChoiceChip(
              label: Text(p.name),
              selected: selected,
              onSelected: (_) => _pick(p),
            );
          }).toList(),
        ),
        const SizedBox(height: 20),
        TextField(
          controller: _keyCtrl,
          autofocus: true,
          obscureText: true,
          decoration: InputDecoration(
            labelText: 'API key',
            hintText: _selected.keyHint,
            border: const OutlineInputBorder(),
            helperText: _selected.docsUrl == null
                ? 'Endpoint: ${_selected.baseUrl}'
                : 'Get a key: ${_selected.docsUrl}',
          ),
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _modelCtrl,
          decoration: const InputDecoration(
            labelText: 'Default model',
            border: OutlineInputBorder(),
            isDense: true,
          ),
        ),
        if (_error != null) ...[
          const SizedBox(height: 12),
          Text(_error!,
              style: TextStyle(color: theme.colorScheme.error, fontSize: 13)),
        ],
        const SizedBox(height: 20),
        _StepDots(count: 2, active: 1),
        const SizedBox(height: 20),
        Row(
          children: [
            TextButton(
              onPressed: _saving ? null : () => setState(() => _step = 0),
              child: const Text('Back'),
            ),
            const Spacer(),
            TextButton(
              onPressed: _saving ? null : _skip,
              child: const Text('Skip'),
            ),
            const SizedBox(width: 8),
            FilledButton(
              onPressed: _saving ? null : _finish,
              child: _saving
                  ? const SizedBox(
                      width: 18,
                      height: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Text('Finish'),
            ),
          ],
        ),
      ],
    );
  }
}

class _StepDots extends StatelessWidget {
  const _StepDots({required this.count, required this.active});
  final int count;
  final int active;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: List.generate(count, (i) {
        final on = i == active;
        return AnimatedContainer(
          duration: const Duration(milliseconds: 200),
          margin: const EdgeInsets.symmetric(horizontal: 3),
          width: on ? 18 : 6,
          height: 6,
          decoration: BoxDecoration(
            color: on
                ? theme.colorScheme.primary
                : theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.3),
            borderRadius: BorderRadius.circular(3),
          ),
        );
      }),
    );
  }
}
