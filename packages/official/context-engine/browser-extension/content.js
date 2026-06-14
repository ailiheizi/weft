'use strict';

const MIN_ARTICLE_TEXT = 200;
const MIN_TITLE_LEN = 12;
const MIN_DWELL_SECONDS = 20;
const READ_CHECK_INTERVAL_MS = 5000;
const DEBUG_STORAGE_KEY = 'weft_context_debug';
const DEBUG_RECORD_LIMIT = 80;

let dwellSeconds = 0;
let lastSentSignature = '';
let lastPageContextSignature = '';
let timer = null;

async function appendDebugRecord(entry) {
  try {
    const stored = await chrome.storage.local.get(DEBUG_STORAGE_KEY);
    const current = Array.isArray(stored?.[DEBUG_STORAGE_KEY]) ? stored[DEBUG_STORAGE_KEY] : [];
    const next = [
      ...current,
      {
        ...entry,
        recorded_at: new Date().toISOString(),
      },
    ].slice(-DEBUG_RECORD_LIMIT);
    await chrome.storage.local.set({ [DEBUG_STORAGE_KEY]: next });
  } catch {}
}

void appendDebugRecord({
  side: 'content',
  stage: 'content_loaded',
  url: normalizeUrl(),
  title: String(document.title || ''),
});

function normalizeUrl(url = location.href) {
  try {
    const parsed = new URL(url);
    if (!/^https?:$/.test(parsed.protocol)) return '';
    return parsed.toString();
  } catch {
    return '';
  }
}

function collectVisibleText() {
  const body = document.body;
  if (!body) return '';
  const raw = body.innerText || body.textContent || '';
  return String(raw).replace(/\s+/g, ' ').trim();
}

function looksReadableByTitle(title = document.title || '') {
  const value = String(title || '').trim();
  if (value.length >= MIN_TITLE_LEN && /(guide|doc|wiki|manual|readme|教程|文档|指南|手册|说明)/i.test(value)) {
    return true;
  }
  return value.length >= MIN_TITLE_LEN;
}

function extractReadableArticle() {
  if (typeof Readability !== 'function' || !document?.cloneNode) {
    return null;
  }
  try {
    const clone = document.cloneNode(true);
    const article = new Readability(clone).parse();
    return article || null;
  } catch {
    return null;
  }
}

function buildPayload() {
  const url = normalizeUrl();
  if (!url) return null;

  const article = extractReadableArticle();
  const articleText = String(article?.textContent || '').replace(/\s+/g, ' ').trim();
  const bodyText = collectVisibleText();
  const text = articleText || bodyText;
  const title = String(document.title || article?.title || '').trim();
  const pathname = (() => {
    try {
      return new URL(url).pathname.toLowerCase();
    } catch {
      return '';
    }
  })();
  const looksReadableUrl = /readme|docs?|guide|manual|wiki|article|blog|issues?|pull|commit/.test(pathname);
  const isReadable = text.length >= MIN_ARTICLE_TEXT || looksReadableByTitle(title) || looksReadableUrl;

  return {
    url,
    title,
    page_title: title,
    article_text: text.slice(0, 8000),
    article_length: text.length,
    byline: String(article?.byline || '').trim(),
    excerpt: String(article?.excerpt || '').trim(),
    site_name: String(article?.siteName || '').trim(),
    dwell_seconds: dwellSeconds,
    readable: isReadable,
  };
}

function buildPageContextPayload() {
  const payload = buildPayload();
  if (!payload) return null;

  return {
    url: payload.url,
    title: payload.title,
    page_title: payload.page_title,
    article_length: payload.article_length,
    readable: payload.readable,
    observed_at: new Date().toISOString(),
    producer: 'browser-extension',
  };
}

async function emitPageContextChanged(force = false) {
  const payload = buildPageContextPayload();
  if (!payload) return;

  const signature = `${payload.url}|${payload.title}|${document.visibilityState}`;
  if (!force && signature === lastPageContextSignature) return;
  lastPageContextSignature = signature;

  await appendDebugRecord({
    side: 'content',
    stage: 'emitPageContextChanged',
    force,
    url: payload.url,
    title: payload.title,
    readable: payload.readable,
    article_length: payload.article_length,
  });

  chrome.runtime.sendMessage({
    type: 'page_context_changed',
    payload,
  }).catch(() => {});
}

async function maybeEmitReadingDetected(force = false) {
  if (document.visibilityState !== 'visible') return;
  const payload = buildPayload();
  if (!payload) return;

  const shouldSend = force || (payload.readable && (payload.article_length >= MIN_ARTICLE_TEXT || payload.dwell_seconds >= MIN_DWELL_SECONDS));
  if (!shouldSend) return;

  const signature = `${payload.url}|${Math.floor(payload.dwell_seconds / MIN_DWELL_SECONDS)}|${payload.article_length >= MIN_ARTICLE_TEXT}`;
  if (signature === lastSentSignature) return;
  lastSentSignature = signature;

  await appendDebugRecord({
    side: 'content',
    stage: 'maybeEmitReadingDetected',
    force,
    url: payload.url,
    title: payload.title,
    readable: payload.readable,
    dwell_seconds: payload.dwell_seconds,
    article_length: payload.article_length,
  });

  chrome.runtime.sendMessage({
    type: 'reading_page_detected',
    payload,
  }).catch(() => {});
}

function startTracking() {
  if (timer) clearInterval(timer);
  timer = setInterval(() => {
    if (document.visibilityState === 'visible') {
      dwellSeconds += READ_CHECK_INTERVAL_MS / 1000;
      void maybeEmitReadingDetected(false);
    }
  }, READ_CHECK_INTERVAL_MS);
}

document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'visible') {
    void emitPageContextChanged(false);
    void maybeEmitReadingDetected(false);
  }
});

window.addEventListener('focus', () => {
  void emitPageContextChanged(false);
});

window.addEventListener('pageshow', () => {
  void emitPageContextChanged(false);
});

window.addEventListener('beforeunload', () => {
  if (timer) clearInterval(timer);
});

const wrapHistoryMethod = (methodName) => {
  const original = history[methodName];
  if (typeof original !== 'function') return;
  history[methodName] = function wrappedHistoryMethod(...args) {
    const result = original.apply(this, args);
    setTimeout(() => {
      void emitPageContextChanged(true);
      void maybeEmitReadingDetected(true);
    }, 0);
    return result;
  };
};

wrapHistoryMethod('pushState');
wrapHistoryMethod('replaceState');

window.addEventListener('popstate', () => {
  void emitPageContextChanged(true);
  void maybeEmitReadingDetected(true);
});

startTracking();
void emitPageContextChanged(true);
setTimeout(() => { void maybeEmitReadingDetected(false); }, 3000);
