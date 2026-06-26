// RSS Reader API — 调 core 的 /api/capabilities/rss.reader/call
// token 和 core base URL 从 URL query 或 window 全局变量获取

const params = new URLSearchParams(window.location.search);

const CORE_BASE = params.get('core') || (window as any).WEFT_BASE_URL || 'http://127.0.0.1:17830';

// Token: URL query > window global > localStorage fallback
function resolveToken(): string {
  return params.get('token')
    || (window as any).WEFT_TOKEN
    || localStorage.getItem('weft_token')
    || '';
}

let TOKEN = resolveToken();

/** Allow setting token at runtime (from UI input). */
export function setToken(t: string) {
  TOKEN = t.trim();
  localStorage.setItem('weft_token', TOKEN);
}

export function getToken() { return TOKEN; }
export function hasToken() { return TOKEN.length > 0; }

const CAPABILITY = 'rss.reader';

export interface ApiResult<T = Record<string, any>> {
  data: T;
  error?: string;
}

export async function call<T = Record<string, any>>(
  action: string,
  data: Record<string, any> = {},
): Promise<T> {
  const resp = await fetch(`${CORE_BASE}/api/capabilities/${CAPABILITY}/call`, {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${TOKEN}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ action, data }),
  });

  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}: ${await resp.text()}`);
  }

  const envelope = await resp.json();
  const response = envelope?.response;
  if (response?.status && response.status !== 'ok') {
    throw new Error(response.error || `${action} failed`);
  }
  return (response?.data ?? {}) as T;
}

// ── Typed API methods ──

export interface Feed {
  id: number;
  url: string;
  title: string;
  added_at: number;
  unread: number;
}

export interface Article {
  id: number;
  feed_id: number;
  guid: string;
  title: string;
  link: string;
  published_at: string;
  content: string;
  summary: string;
  is_read: number;
  is_favorite: number;
  fetched_at: number;
}

export interface Recommendation {
  article_id: number;
  title: string;
  link: string;
  score: number;
  confidence: number;
  matched_keywords: string[];
  rationale_zh: string;
  aversion: string;
  grounded: boolean;
}

export interface RecommendResult {
  recommendations: Recommendation[];
  token_used: boolean;
  mode: 'reasoned' | 'cold_start' | 'l2_fallback';
  profile_size: number;
}

export const api = {
  listFeeds: () => call<{ feeds: Feed[] }>('list_feeds'),
  addFeed: (url: string) => call('add_feed', { url }),
  removeFeed: (feedId: number) => call('remove_feed', { feed_id: feedId }),
  refreshFeed: (feedId: number) => call('refresh_feed', { feed_id: feedId }),
  refreshAll: () => call('refresh_all'),
  listArticles: (opts: { feed_id?: number; unread_only?: boolean; favorites_only?: boolean; limit?: number } = {}) =>
    call<{ articles: Article[] }>('list_articles', opts),
  markRead: (articleId: number, isRead = true) => call('mark_read', { article_id: articleId, is_read: isRead }),
  markAllRead: (feedId?: number) => call('mark_all_read', feedId != null ? { feed_id: feedId } : {}),
  markFavorite: (articleId: number, isFavorite = true) => call('mark_favorite', { article_id: articleId, is_favorite: isFavorite }),
  summarize: (articleId: number, mode: 'summary' | 'translate' = 'summary') =>
    call<{ summary: string }>('summarize_article', { article_id: articleId, mode }),
  recommend: (opts: { feed_id?: number; limit?: number; force_refresh?: boolean } = {}) =>
    call<RecommendResult>('recommend_articles', opts),
  chatWithArticle: (articleId: number, message: string) =>
    call<{ reply: string; article_id: number }>('chat_with_article', { article_id: articleId, message }),
  analyzeSections: (articleId: number) =>
    call<{ analysis?: any; analysis_raw?: string; article_id: number }>('analyze_sections', { article_id: articleId }),
  explainSelection: (text: string, question?: string, context?: string) =>
    call<{ explanation: string }>('explain_selection', { text, question: question || '请解释这段内容', context: context || '' }),
  proxyPage: (url: string) =>
    call<{ html: string; url: string }>('proxy_page', { url }),
  webSearch: (query: string) =>
    call<{ query: string; results: any }>('web_search', { query }),
  translateText: (text: string, target = 'zh', source = 'auto') =>
    call<{ translated: string; provider: string; target: string }>('translate_text', { text, target, source }),
  getAiConfig: () => call<{ base_url: string; model: string; has_api_key: boolean; api_key_masked: string }>('get_ai_config'),
  setAiConfig: (config: { base_url?: string; api_key?: string; model?: string; provider?: string }) => call('set_ai_config', config),
  getTranslateConfig: () => call<{ provider: string; target: string; api_key?: string; region?: string }>('get_ai_config').then(res => ({
    provider: (res as any)['translate.provider'] || 'mymemory',
    target: (res as any)['translate.target'] || 'zh',
    api_key: (res as any)['translate.api_key'] || '',
    region: (res as any)['translate.region'] || '',
  })),
  setTranslateConfig: (config: { provider?: string; target?: string; api_key?: string; region?: string }) =>
    call('set_ai_config', {
      'translate.provider': config.provider,
      'translate.target': config.target,
      'translate.api_key': config.api_key,
      'translate.region': config.region,
    }),
};

export function getCoreBase() { return CORE_BASE; }

// ── Core-level APIs (not through capability, direct core endpoints) ──

export interface Provider {
  name: string;
  base_url: string;
  format: string;
  models: string[];
  key_count: number;
}

export async function listProviders(): Promise<Provider[]> {
  const resp = await fetch(`${CORE_BASE}/api/providers`, {
    headers: { 'Authorization': `Bearer ${TOKEN}` },
  });
  if (!resp.ok) throw new Error(`listProviders: HTTP ${resp.status}`);
  const data = await resp.json();
  return data.providers || [];
}

/** Get all chat-capable models from providers (exclude image-only providers). */
export async function listChatModels(): Promise<string[]> {
  const providers = await listProviders();
  const imageKeywords = ['image', 'flux', 'dall', 'gpt-image'];
  return providers
    .filter(p => !imageKeywords.some(k => p.name.includes(k) || p.models.some(m => m.includes(k))))
    .flatMap(p => p.models);
}
