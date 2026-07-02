import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import '../../core/models/provider.dart';
import '../../core/models/provider_presets.dart';
import '../../core/providers/core_repository.dart';
import '../../core/providers/data_providers.dart';
import '../../core/providers/preferences_provider.dart';
import '../../shared/theme/spacing.dart';

/// 首次启动引导 (OOBE):当用户还没配置任何 provider 时,引导他配置第一个
/// LLM provider,而不是直接进入空白 dashboard。
///
/// 路由通过 [routerProvider] 的 redirect 逻辑控制:providers 为空且
/// onboarding 未完成时,自动跳到 `/onboarding`。
class OnboardingScreen extends ConsumerStatefulWidget {
  const OnboardingScreen({super.key});

  @override
  ConsumerState<OnboardingScreen> createState() => _OnboardingScreenState();
}

class _OnboardingScreenState extends ConsumerState<OnboardingScreen> {
  final _nameCtrl = TextEditingController();
  final _urlCtrl = TextEditingController();
  final _keyCtrl = TextEditingController();
  final _modelCtrl = TextEditingController();

  /// 选中的预设;null 表示「自定义」。
  ProviderPreset? _preset = kProviderPresets.first;
  String _format = 'openai';
  bool _obscureKey = true;
  bool _saving = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    // 默认选中第一个预设,预填表单。
    if (_preset != null) _applyPreset(_preset!);
  }

  @override
  void dispose() {
    _nameCtrl.dispose();
    _urlCtrl.dispose();
    _keyCtrl.dispose();
    _modelCtrl.dispose();
    super.dispose();
  }

  void _applyPreset(ProviderPreset p) {
    setState(() {
      _preset = p;
      _nameCtrl.text = p.name;
      _urlCtrl.text = p.baseUrl;
      _modelCtrl.text = p.defaultModel;
      _format = p.format;
      _error = null;
    });
  }

  void _selectCustom() {
    setState(() {
      _preset = null;
      _nameCtrl.clear();
      _urlCtrl.clear();
      _modelCtrl.clear();
      _format = 'openai';
      _error = null;
    });
  }

  /// 跳过:标记 onboarding 完成,直接进 dashboard(用户可稍后在设置里配)。
  Future<void> _skip() async {
    await ref.read(preferencesProvider.notifier).setOnboardingCompleted(true);
    if (!mounted) return;
    context.go('/dashboard');
  }

  Future<void> _finish() async {
    final name = _nameCtrl.text.trim();
    final url = _urlCtrl.text.trim();
    final key = _keyCtrl.text.trim();
    final model = _modelCtrl.text.trim();

    if (name.isEmpty) {
      setState(() => _error = '请填写 Provider 名称');
      return;
    }
    if (url.isEmpty) {
      setState(() => _error = '请填写 Base URL');
      return;
    }
    if (key.isEmpty) {
      setState(() => _error = '请填写 API Key');
      return;
    }

    setState(() {
      _saving = true;
      _error = null;
    });

    final config = ProviderConfig(
      name: name,
      baseUrl: url,
      format: _format,
      models: model.isEmpty ? const [] : [model],
      keys: [ApiKeyConfig(key: key)],
    );

    try {
      await ref.read(coreRepositoryProvider).createProvider(config);
      await ref.read(preferencesProvider.notifier).setOnboardingCompleted(true);
      // 让 dashboard / providers 页读到新配置。
      ref.invalidate(providersProvider);
      if (!mounted) return;
      context.go('/dashboard');
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _saving = false;
        _error = '保存失败: $e';
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      backgroundColor: Colors.transparent,
      body: Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(Spacing.xl),
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 520),
            child: Card(
              margin: EdgeInsets.zero,
              child: Padding(
                padding: const EdgeInsets.all(Spacing.xl),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    // ── 欢迎标题 ────────────────────────────────────────
                    Text('欢迎使用 Weft',
                        style: theme.textTheme.displaySmall),
                    const SizedBox(height: Spacing.sm),
                    Text(
                      '先配置一个 AI Provider,即可开始对话与编排。稍后可在设置中添加更多。',
                      style: theme.textTheme.bodyMedium?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant),
                    ),
                    const SizedBox(height: Spacing.xl),

                    // ── 预设选择 ────────────────────────────────────────
                    Text('选择 Provider',
                        style: theme.textTheme.labelLarge),
                    const SizedBox(height: Spacing.sm),
                    Wrap(
                      spacing: Spacing.sm,
                      runSpacing: Spacing.sm,
                      children: [
                        ...kProviderPresets.map((p) => ChoiceChip(
                              label: Text(p.name),
                              selected: _preset?.name == p.name,
                              onSelected: (_) => _applyPreset(p),
                            )),
                        ChoiceChip(
                          label: const Text('自定义'),
                          selected: _preset == null,
                          onSelected: (_) => _selectCustom(),
                        ),
                      ],
                    ),
                    const SizedBox(height: Spacing.lg),

                    // ── 表单 ────────────────────────────────────────────
                    TextField(
                      controller: _nameCtrl,
                      decoration: const InputDecoration(
                        labelText: 'Name',
                        hintText: '例如 DeepSeek',
                      ),
                    ),
                    const SizedBox(height: Spacing.md),
                    TextField(
                      controller: _urlCtrl,
                      decoration: const InputDecoration(labelText: 'Base URL'),
                    ),
                    const SizedBox(height: Spacing.md),
                    TextField(
                      controller: _keyCtrl,
                      obscureText: _obscureKey,
                      decoration: InputDecoration(
                        labelText: 'API Key',
                        hintText: _preset?.keyHint ?? 'sk-...',
                        suffixIcon: IconButton(
                          icon: Icon(
                            _obscureKey
                                ? Icons.visibility_outlined
                                : Icons.visibility_off_outlined,
                            size: 16,
                          ),
                          onPressed: () =>
                              setState(() => _obscureKey = !_obscureKey),
                          visualDensity: VisualDensity.compact,
                        ),
                      ),
                    ),
                    const SizedBox(height: Spacing.md),
                    TextField(
                      controller: _modelCtrl,
                      decoration: const InputDecoration(
                        labelText: 'Model (可选)',
                        hintText: '例如 deepseek-chat',
                      ),
                    ),

                    if (_error != null) ...[
                      const SizedBox(height: Spacing.md),
                      Text(
                        _error!,
                        style: TextStyle(
                            color: theme.colorScheme.error, fontSize: 13),
                      ),
                    ],

                    const SizedBox(height: Spacing.xl),

                    // ── 操作按钮 ────────────────────────────────────────
                    Row(
                      children: [
                        TextButton(
                          onPressed: _saving ? null : _skip,
                          child: const Text('跳过'),
                        ),
                        const Spacer(),
                        FilledButton(
                          onPressed: _saving ? null : _finish,
                          child: _saving
                              ? const SizedBox(
                                  width: 16,
                                  height: 16,
                                  child: CircularProgressIndicator(
                                      strokeWidth: 2),
                                )
                              : const Text('完成'),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
