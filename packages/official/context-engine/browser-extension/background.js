'use strict';

const EVENT_ENDPOINT = 'http://127.0.0.1:43131/webhook';
const RECENT_URLS = new Map();
const DEBUG_STORAGE_KEY = 'weft_context_debug';
const DEBUG_RECORD_LIMIT = 80;

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
  side: 'background',
  stage: 'background_loaded',
});

function normalizeUrl(url = '') {
  try {
    const parsed = new URL(url);
    if (!/^https?:$/.test(parsed.protocol)) return '';
    return parsed.toString();
  } catch {
    return '';
  }
}

function extractDomain(url = '') {
  try {
    return new URL(url).hostname || '';
  } catch {
    return '';
  }
}

async function postSkillEvent(eventType, payload) {
  try {
    const response = await fetch(EVENT_ENDPOINT, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        action: 'ingest_external_event',
        data: {
          event_type: eventType,
          payload,
        },
      }),
    });
    await appendDebugRecord({
      side: 'background',
      stage: 'postSkillEvent',
      event_type: eventType,
      ok: response.ok,
      status: response.status,
      url: String(payload?.url || ''),
      title: String(payload?.title || payload?.page_title || ''),
    });
    return response.ok;
  } catch (error) {
    await appendDebugRecord({
      side: 'background',
      stage: 'postSkillEvent',
      event_type: eventType,
      ok: false,
      error: String(error?.message || error || 'unknown error'),
      url: String(payload?.url || ''),
      title: String(payload?.title || payload?.page_title || ''),
    });
    return false;
  }
}

async function emitActiveUrlChanged(tabId) {
  if (typeof tabId !== 'number') return;
  const tab = await chrome.tabs.get(tabId).catch(() => null);
  if (!tab) return;

  const url = normalizeUrl(tab.url || '');
  if (!url) return;

  const dedupeKey = `${tabId}:${url}`;
  if (RECENT_URLS.get(tabId) === dedupeKey) return;
  RECENT_URLS.set(tabId, dedupeKey);

  await postSkillEvent('active_url_changed', {
    url,
    title: String(tab.title || '').trim(),
    page_title: String(tab.title || '').trim(),
    domain: extractDomain(url),
    tab_id: tabId,
    observed_at: new Date().toISOString(),
    producer: 'browser-extension',
  });
}

async function emitActiveUrlChangedFromPayload(payload, tabId) {
  const url = normalizeUrl(payload?.url || '');
  if (!url) return false;

  const resolvedTabId = typeof tabId === 'number' ? tabId : Number(payload?.tab_id);
  const dedupeKey = `${Number.isFinite(resolvedTabId) ? resolvedTabId : 'page'}:${url}`;
  if (RECENT_URLS.get(resolvedTabId ?? dedupeKey) === dedupeKey) return true;
  RECENT_URLS.set(resolvedTabId ?? dedupeKey, dedupeKey);

  return await postSkillEvent('active_url_changed', {
    url,
    title: String(payload?.tab_title || payload?.title || payload?.page_title || '').trim(),
    page_title: String(payload?.tab_title || payload?.page_title || payload?.title || '').trim(),
    domain: extractDomain(url),
    tab_id: Number.isFinite(resolvedTabId) ? resolvedTabId : undefined,
    observed_at: String(payload?.observed_at || new Date().toISOString()),
    producer: String(payload?.producer || 'browser-extension'),
    readable: Boolean(payload?.readable),
    article_length: Number(payload?.article_length || 0),
  });
}

chrome.tabs.onActivated.addListener(async ({ tabId }) => {
  await appendDebugRecord({
    side: 'background',
    stage: 'tabs.onActivated',
    tab_id: tabId,
  });
  await emitActiveUrlChanged(tabId);
});

chrome.tabs.onUpdated.addListener(async (tabId, changeInfo, tab) => {
  await appendDebugRecord({
    side: 'background',
    stage: 'tabs.onUpdated',
    tab_id: tabId,
    change_status: String(changeInfo?.status || ''),
    change_url: String(changeInfo?.url || ''),
    tab_active: Boolean(tab?.active),
    tab_title: String(tab?.title || ''),
  });
  if (changeInfo.status === 'complete' || typeof changeInfo.url === 'string') {
    await emitActiveUrlChanged(tabId);
  } else if (tab?.active && changeInfo.title) {
    await emitActiveUrlChanged(tabId);
  }
});

chrome.webNavigation.onHistoryStateUpdated.addListener(async ({ tabId, frameId }) => {
  await appendDebugRecord({
    side: 'background',
    stage: 'webNavigation.onHistoryStateUpdated',
    tab_id: tabId,
    frame_id: frameId,
  });
  if (frameId !== 0) return;
  await emitActiveUrlChanged(tabId);
});

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  (async () => {
    if (!message || typeof message !== 'object') {
      sendResponse({ ok: false, error: 'invalid message' });
      return;
    }

    await appendDebugRecord({
      side: 'background',
      stage: 'runtime.onMessage',
      message_type: String(message.type || ''),
      tab_id: sender?.tab?.id,
      url: String(message?.payload?.url || sender?.tab?.url || ''),
      title: String(message?.payload?.title || message?.payload?.page_title || sender?.tab?.title || ''),
    });

    if (message.type === 'reading_page_detected') {
      const payload = message.payload && typeof message.payload === 'object' ? message.payload : {};
      const tabId = sender?.tab?.id;
      const currentUrl = normalizeUrl(payload.url || sender?.tab?.url || '');
      const responseOk = await postSkillEvent('reading_page_detected', {
        ...payload,
        title: String(sender?.tab?.title || payload.title || payload.page_title || '').trim(),
        page_title: String(sender?.tab?.title || payload.page_title || payload.title || '').trim(),
        url: currentUrl,
        domain: extractDomain(currentUrl),
        tab_id: tabId,
        observed_at: new Date().toISOString(),
        producer: 'browser-extension',
      });
      sendResponse({ ok: responseOk });
      return;
    }

    if (message.type === 'page_context_changed') {
      const payload = message.payload && typeof message.payload === 'object' ? message.payload : {};
      const tabId = sender?.tab?.id;
      const responseOk = await emitActiveUrlChangedFromPayload({
        ...payload,
        url: payload.url || sender?.tab?.url || '',
        tab_title: String(sender?.tab?.title || '').trim(),
        title: payload.title || sender?.tab?.title || '',
        page_title: payload.page_title || sender?.tab?.title || '',
      }, tabId);
      sendResponse({ ok: responseOk });
      return;
    }

    sendResponse({ ok: false, error: 'unsupported message type' });
  })();

  return true;
});
