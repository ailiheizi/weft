require('dotenv').config({ quiet: true });

const companionProvider = (process.env.COMPANION_PROVIDER || '').trim().toLowerCase();

module.exports = {
  user: {
    name: (process.env.USER_NAME || process.env.CHANGZHENG_USER_NAME || '').trim(),
  },
  claude: {
    baseUrl: process.env.CLAUDE_BASE_URL || 'https://api.anthropic.com',
    apiKey:  process.env.CLAUDE_API_KEY,
    model:   process.env.CLAUDE_MODEL || 'claude-opus-4-6',
  },
  companion: {
    provider: companionProvider || (process.env.DEEPSEEK_API_KEY ? 'deepseek' : 'claude'),
    claude: {
      baseUrl: process.env.CLAUDE_COMPANION_BASE_URL || process.env.CLAUDE_BASE_URL || 'https://api.anthropic.com',
      apiKey:  process.env.CLAUDE_COMPANION_API_KEY  || process.env.CLAUDE_API_KEY,
      model:   process.env.CLAUDE_COMPANION_MODEL    || process.env.CLAUDE_MODEL || 'claude-opus-4-6',
    },
    deepseek: {
      apiKey:  process.env.DEEPSEEK_COMPANION_API_KEY  || process.env.DEEPSEEK_API_KEY,
      baseUrl: process.env.DEEPSEEK_COMPANION_BASE_URL || process.env.DEEPSEEK_BASE_URL || 'https://api.deepseek.com',
      model:   process.env.DEEPSEEK_COMPANION_MODEL    || process.env.DEEPSEEK_MODEL || 'deepseek-chat',
    },
  },
  deepseek: {
    apiKey:  process.env.DEEPSEEK_API_KEY,
    baseUrl: process.env.DEEPSEEK_BASE_URL || 'https://api.deepseek.com',
    model:   process.env.DEEPSEEK_MODEL   || 'deepseek-chat',
  },
  aliyun: {
    appKey:    process.env.ALIYUN_NLS_APP_KEY,
    akId:      process.env.ALIYUN_AK_ID,
    akSecret:  process.env.ALIYUN_AK_SECRET,
    token:     process.env.ALIYUN_NLS_TOKEN,   // 静态 token（备用）
    ttsVoice:  process.env.ALIYUN_TTS_VOICE || 'xiaoyun',
    region:    'cn-shanghai',
  },
  server: {
    wsPort: parseInt(process.env.WS_PORT) || 8765,
  },
};
