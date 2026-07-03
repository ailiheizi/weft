/**
 * Weft FFI WebView Bridge Shim
 *
 * 注入到 WebView2 页面中，拦截指向 Core API 的 fetch 请求，
 * 通过 chrome.webview.postMessage 转发给 Dart FFI 层。
 *
 * 注入方式: controller.addScriptToExecuteOnDocumentCreated(shimSource)
 */
(function () {
  'use strict';

  // --- 配置 ---
  // CORE_BASE 由 Dart 注入时动态替换，或留空表示拦截所有 /api/* 相对路径
  const CORE_BASE = '{{CORE_BASE}}';
  const REQUEST_TIMEOUT_MS = 30000;

  // --- 内部状态 ---
  const _originalFetch = window.fetch.bind(window);
  const _pendingRequests = new Map(); // id → { resolve, reject, timer }
  let _requestCounter = 0;

  /**
   * 生成唯一请求 ID
   */
  function generateId() {
    return `weft_${Date.now()}_${++_requestCounter}`;
  }

  /**
   * 判断 URL 是否应被拦截
   */
  function shouldIntercept(url) {
    // 相对路径 /api/* 直接拦截
    if (url.startsWith('/api/')) return true;
    // 绝对路径匹配 CORE_BASE
    if (CORE_BASE && url.startsWith(CORE_BASE + '/api/')) return true;
    // http://localhost 或 127.0.0.1 + /api/*
    const match = url.match(/^https?:\/\/(localhost|127\.0\.0\.1)(:\d+)?\/api\//);
    return !!match;
  }

  /**
   * 从完整 URL 中提取 /api/... 路径
   */
  function extractPath(url) {
    if (url.startsWith('/api/')) return url;
    const idx = url.indexOf('/api/');
    return idx >= 0 ? url.substring(idx) : url;
  }

  /**
   * 拦截版 fetch
   */
  window.fetch = async function (input, init) {
    const url = typeof input === 'string' ? input : input.url;

    if (!shouldIntercept(url)) {
      return _originalFetch(input, init);
    }

    const method = (init?.method || 'GET').toUpperCase();
    const headers = Object.fromEntries(new Headers(init?.headers || {}).entries());

    let body = null;
    if (init?.body) {
      try {
        body = typeof init.body === 'string' ? JSON.parse(init.body) : init.body;
      } catch {
        body = { _raw: init.body };
      }
    }

    const id = generateId();
    const path = extractPath(url);

    // 构造请求消息
    const message = {
      id,
      type: 'request',
      method,
      url: path,
      headers,
      body,
    };

    // 返回 Promise，等待 Dart 回传
    return new Promise((resolve, reject) => {
      // 超时处理
      const timer = setTimeout(() => {
        _pendingRequests.delete(id);
        reject(new Error(`Weft bridge timeout after ${REQUEST_TIMEOUT_MS}ms: ${method} ${path}`));
      }, REQUEST_TIMEOUT_MS);

      _pendingRequests.set(id, { resolve, reject, timer });

      // 发送到 Dart 层
      window.chrome.webview.postMessage(JSON.stringify(message));
    });
  };

  /**
   * 监听 Dart 回传的响应消息
   */
  window.chrome.webview.addEventListener('message', function (event) {
    let data;
    try {
      data = typeof event.data === 'string' ? JSON.parse(event.data) : event.data;
    } catch {
      return; // 非 JSON 消息，忽略
    }

    if (data.type !== 'response') return;

    const pending = _pendingRequests.get(data.id);
    if (!pending) return; // 已超时或重复消息

    _pendingRequests.delete(data.id);
    clearTimeout(pending.timer);

    // 构造标准 Response 对象
    const responseBody = data.body != null ? JSON.stringify(data.body) : null;
    const response = new Response(responseBody, {
      status: data.status || 200,
      statusText: data.status >= 400 ? 'Error' : 'OK',
      headers: data.headers || { 'Content-Type': 'application/json' },
    });

    pending.resolve(response);
  });

  // 标记 bridge 已就绪
  window.__weftBridgeReady = true;
  window.dispatchEvent(new CustomEvent('weft-bridge-ready'));

  console.debug('[Weft Bridge] Shim initialized, intercepting /api/* requests via FFI');
})();
