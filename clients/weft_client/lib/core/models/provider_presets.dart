/// Built-in AI provider presets shared by onboarding (OOBE) and the
/// Providers screen, so users can one-click a known provider instead of
/// hand-typing the base URL / format / model.
class ProviderPreset {
  const ProviderPreset({
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

const kProviderPresets = <ProviderPreset>[
  ProviderPreset(
    name: 'DeepSeek',
    baseUrl: 'https://api.deepseek.com',
    format: 'openai',
    defaultModel: 'deepseek-chat',
    keyHint: 'sk-...',
    docsUrl: 'https://platform.deepseek.com/api_keys',
  ),
  ProviderPreset(
    name: 'OpenAI',
    baseUrl: 'https://api.openai.com/v1',
    format: 'openai',
    defaultModel: 'gpt-4o',
    keyHint: 'sk-...',
    docsUrl: 'https://platform.openai.com/api-keys',
  ),
  ProviderPreset(
    name: 'OpenRouter',
    baseUrl: 'https://openrouter.ai/api/v1',
    format: 'openai',
    defaultModel: 'openai/gpt-4o',
    keyHint: 'sk-or-...',
    docsUrl: 'https://openrouter.ai/keys',
  ),
  ProviderPreset(
    name: 'Anthropic',
    baseUrl: 'https://api.anthropic.com',
    format: 'anthropic',
    defaultModel: 'claude-sonnet-4-20250514',
    keyHint: 'sk-ant-...',
    docsUrl: 'https://console.anthropic.com/settings/keys',
  ),
  ProviderPreset(
    name: 'Moonshot (Kimi)',
    baseUrl: 'https://api.moonshot.cn/v1',
    format: 'openai',
    defaultModel: 'moonshot-v1-8k',
    keyHint: 'sk-...',
    docsUrl: 'https://platform.moonshot.cn/console/api-keys',
  ),
  ProviderPreset(
    name: 'SiliconFlow',
    baseUrl: 'https://api.siliconflow.cn/v1',
    format: 'openai',
    defaultModel: 'deepseek-ai/DeepSeek-V3',
    keyHint: 'sk-...',
    docsUrl: 'https://cloud.siliconflow.cn/account/ak',
  ),
];
