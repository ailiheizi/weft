// ==UserScript==
// @name         Weft RSS 阅读助手
// @namespace    https://weft.dev/rss-reader
// @version      1.0.0
// @description  划词问AI、渐进式分段展开、文章对话。配合 weft rss-reader 包使用。
// @author       weft
// @match        *://*/*
// @grant        GM_xmlhttpRequest
// @grant        GM_getValue
// @grant        GM_setValue
// @grant        GM_registerMenuCommand
// @grant        GM_addStyle
// @connect      127.0.0.1
// @run-at       document-idle
// ==/UserScript==

(function () {
  'use strict';

  // ── 配置 ──
  const CORE_BASE = GM_getValue('weft_core_base', 'http://127.0.0.1:17830');
  const TOKEN = GM_getValue('weft_token', '');
  const CAPABILITY = 'rss.reader';

  // 注册脚本猫菜单：配置 token 和 core 地址
  GM_registerMenuCommand('⚙️ 设置 Weft 连接', () => {
    const base = prompt('Core 地址:', CORE_BASE);
    if (base) GM_setValue('weft_core_base', base.trim());
    const token = prompt('Loopback Token (从 data/runtime-token 复制):', TOKEN);
    if (token) GM_setValue('weft_token', token.trim());
    alert('已保存，刷新页面生效');
  });

  if (!TOKEN) {
    console.log('[Weft RSS] 未配置 token，请通过脚本猫菜单设置');
    return;
  }

  // ── API 调用 ──
  function callAPI(action, data) {
    return new Promise((resolve, reject) => {
      GM_xmlhttpRequest({
        method: 'POST',
        url: `${CORE_BASE}/api/capabilities/${CAPABILITY}/call`,
        headers: {
          'Authorization': `Bearer ${TOKEN}`,
          'Content-Type': 'application/json',
        },
        data: JSON.stringify({ action, data }),
        onload: (resp) => {
          try {
            const envelope = JSON.parse(resp.responseText);
            const response = envelope?.response;
            if (response?.status && response.status !== 'ok') {
              reject(new Error(response.error || `${action} failed`));
            } else {
              resolve(response?.data || {});
            }
          } catch (e) {
            reject(new Error(`Parse error: ${e.message}`));
          }
        },
        onerror: (e) => reject(new Error(`Network error: ${e.statusText || 'unknown'}`)),
      });
    });
  }

  // ── 注入样式 ──
  GM_addStyle(`
    .weft-float-btn {
      position: absolute; z-index: 99999;
      background: #1a1a2e; color: #4fc3f7; border: 1px solid #4fc3f7;
      border-radius: 6px; padding: 4px 10px; font-size: 12px;
      cursor: pointer; box-shadow: 0 2px 8px rgba(0,0,0,0.4);
      font-family: -apple-system, sans-serif; display: none;
    }
    .weft-float-btn:hover { background: #4fc3f7; color: #111; }

    .weft-panel {
      position: fixed; right: 12px; top: 12px; bottom: 12px; width: 380px;
      background: #1a1a2e; border: 1px solid #2a2a4a; border-radius: 10px;
      box-shadow: 0 4px 20px rgba(0,0,0,0.5); z-index: 99998;
      display: flex; flex-direction: column; font-family: -apple-system, sans-serif;
      color: #e0e0e0; font-size: 13px; overflow: hidden;
      transition: transform 0.2s; transform: translateX(110%);
    }
    .weft-panel.open { transform: translateX(0); }

    .weft-panel-header {
      padding: 12px 16px; border-bottom: 1px solid #2a2a4a;
      display: flex; align-items: center; gap: 8px; flex-shrink: 0;
    }
    .weft-panel-header h3 { flex: 1; font-size: 14px; margin: 0; }
    .weft-panel-close {
      background: none; border: none; color: #a0a0a0; font-size: 18px;
      cursor: pointer; padding: 4px;
    }
    .weft-panel-close:hover { color: #fff; }

    .weft-panel-body {
      flex: 1; overflow-y: auto; padding: 12px 16px;
    }

    .weft-panel-footer {
      padding: 8px 12px; border-top: 1px solid #2a2a4a;
      display: flex; gap: 6px; flex-shrink: 0;
    }
    .weft-panel-footer input {
      flex: 1; background: #0f3460; border: 1px solid #2a2a4a;
      border-radius: 6px; padding: 6px 10px; color: #e0e0e0; font-size: 13px;
      outline: none;
    }
    .weft-panel-footer button {
      background: #4fc3f7; color: #111; border: none; border-radius: 6px;
      padding: 6px 12px; font-size: 12px; cursor: pointer; font-weight: 500;
    }
    .weft-panel-footer button:disabled { opacity: 0.5; }

    .weft-msg { margin-bottom: 10px; line-height: 1.5; }
    .weft-msg.user { color: #4fc3f7; }
    .weft-msg.assistant { color: #e0e0e0; }
    .weft-msg .role { font-size: 11px; color: #707070; margin-bottom: 2px; }

    .weft-sections { margin-top: 8px; }
    .weft-section-item {
      padding: 6px 0; border-bottom: 1px solid #2a2a4a; cursor: pointer;
    }
    .weft-section-item:hover { color: #4fc3f7; }
    .weft-section-item .sec-idx {
      display: inline-block; background: #4fc3f7; color: #111;
      width: 20px; height: 20px; border-radius: 50%; text-align: center;
      line-height: 20px; font-size: 11px; margin-right: 8px; font-weight: 600;
    }
    .weft-section-item .sec-summary { font-size: 12px; }
    .weft-section-item .sec-keywords {
      font-size: 11px; color: #707070; margin-top: 2px;
    }

    .weft-overview {
      background: #16213e; border-radius: 6px; padding: 10px;
      margin-bottom: 12px; line-height: 1.6;
    }

    .weft-tab-bar {
      display: flex; border-bottom: 1px solid #2a2a4a; flex-shrink: 0;
    }
    .weft-tab {
      padding: 6px 14px; font-size: 12px; cursor: pointer;
      border-bottom: 2px solid transparent; color: #a0a0a0;
    }
    .weft-tab.active { color: #4fc3f7; border-bottom-color: #4fc3f7; }

    .weft-loading { color: #707070; padding: 12px; text-align: center; }
  `);

  // ── 创建面板 ──
  const panel = document.createElement('div');
  panel.className = 'weft-panel';
  panel.innerHTML = `
    <div class="weft-panel-header">
      <h3>📖 Weft 阅读助手</h3>
      <button class="weft-panel-close">✕</button>
    </div>
    <div class="weft-tab-bar">
      <div class="weft-tab active" data-tab="chat">💬 对话</div>
      <div class="weft-tab" data-tab="sections">📑 分段</div>
    </div>
    <div class="weft-panel-body" id="weft-body"></div>
    <div class="weft-panel-footer">
      <input id="weft-input" placeholder="输入问题..." />
      <button id="weft-send">发送</button>
    </div>
  `;
  document.body.appendChild(panel);

  const closeBtn = panel.querySelector('.weft-panel-close');
  const bodyEl = panel.querySelector('#weft-body');
  const inputEl = panel.querySelector('#weft-input');
  const sendBtn = panel.querySelector('#weft-send');
  const tabs = panel.querySelectorAll('.weft-tab');

  let currentTab = 'chat';
  let articleId = null; // 从 URL params 或页面 meta 获取
  let chatHistory = [];
  let sectionsData = null;

  // 尝试从 URL 获取 article_id（如果 weft 在 URL 里传了）
  const urlParams = new URLSearchParams(window.location.search);
  articleId = parseInt(urlParams.get('weft_article_id') || '0') || null;

  closeBtn.onclick = () => panel.classList.remove('open');

  // Tab 切换
  tabs.forEach(tab => {
    tab.onclick = () => {
      tabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
      currentTab = tab.dataset.tab;
      renderBody();
    };
  });

  // ── 渲染 ──
  function renderBody() {
    if (currentTab === 'chat') renderChat();
    else if (currentTab === 'sections') renderSections();
  }

  function renderChat() {
    let html = '';
    if (chatHistory.length === 0) {
      html = '<div class="weft-msg" style="color:#707070">选中文本后点击浮窗提问，或直接在下方输入问题</div>';
    }
    chatHistory.forEach(msg => {
      html += `<div class="weft-msg ${msg.role}"><div class="role">${msg.role === 'user' ? '你' : 'AI'}</div>${escapeHtml(msg.content)}</div>`;
    });
    bodyEl.innerHTML = html;
    bodyEl.scrollTop = bodyEl.scrollHeight;
  }

  function renderSections() {
    if (!sectionsData) {
      bodyEl.innerHTML = '<div class="weft-loading">点击下方「分析文章」加载...</div>' +
        '<div style="text-align:center;margin-top:8px"><button id="weft-analyze-btn" style="background:#4fc3f7;color:#111;border:none;border-radius:6px;padding:6px 14px;cursor:pointer;font-size:12px">📑 分析文章结构</button></div>';
      const btn = bodyEl.querySelector('#weft-analyze-btn');
      if (btn) btn.onclick = loadSections;
      return;
    }
    let html = '';
    if (sectionsData.overview) {
      html += `<div class="weft-overview"><strong>概要：</strong>${escapeHtml(sectionsData.overview)}</div>`;
    }
    html += '<div class="weft-sections">';
    (sectionsData.sections || []).forEach(sec => {
      html += `<div class="weft-section-item" data-start="${escapeHtml(sec.start_text || '')}">
        <span class="sec-idx">${sec.index}</span>
        <span class="sec-summary">${escapeHtml(sec.summary)}</span>
        <div class="sec-keywords">${(sec.keywords || []).join(' · ')}</div>
      </div>`;
    });
    html += '</div>';
    bodyEl.innerHTML = html;

    // 点击段落索引跳到原文
    bodyEl.querySelectorAll('.weft-section-item').forEach(el => {
      el.onclick = () => {
        const startText = el.dataset.start;
        if (startText) scrollToText(startText);
      };
    });
  }

  async function loadSections() {
    if (!articleId) {
      bodyEl.innerHTML = '<div class="weft-loading">⚠️ 无法确定文章ID，请从 weft 客户端打开文章</div>';
      return;
    }
    bodyEl.innerHTML = '<div class="weft-loading">AI 正在分析文章结构...</div>';
    try {
      const res = await callAPI('analyze_sections', { article_id: articleId });
      sectionsData = res.analysis || JSON.parse(res.analysis_raw || '{}');
      renderSections();
    } catch (e) {
      bodyEl.innerHTML = `<div class="weft-loading" style="color:#ef5350">分析失败：${escapeHtml(e.message)}</div>`;
    }
  }

  // ── 发送消息 ──
  async function sendMessage(text) {
    if (!text.trim()) return;
    chatHistory.push({ role: 'user', content: text });
    renderChat();
    inputEl.value = '';
    sendBtn.disabled = true;

    try {
      if (articleId) {
        const res = await callAPI('chat_with_article', { article_id: articleId, message: text });
        chatHistory.push({ role: 'assistant', content: res.reply });
      } else {
        // 无 article_id 则用 explain_selection
        const res = await callAPI('explain_selection', { text, question: text });
        chatHistory.push({ role: 'assistant', content: res.explanation });
      }
    } catch (e) {
      chatHistory.push({ role: 'assistant', content: `⚠️ ${e.message}` });
    }
    renderChat();
    sendBtn.disabled = false;
  }

  sendBtn.onclick = () => sendMessage(inputEl.value);
  inputEl.onkeydown = (e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); sendMessage(inputEl.value); } };

  // ── 划词浮窗 ──
  const floatBtn = document.createElement('div');
  floatBtn.className = 'weft-float-btn';
  floatBtn.textContent = '🤖 问 AI';
  document.body.appendChild(floatBtn);

  let selectedText = '';

  document.addEventListener('mouseup', (e) => {
    const sel = window.getSelection();
    const text = sel?.toString().trim();
    if (text && text.length > 2 && !panel.contains(e.target) && !floatBtn.contains(e.target)) {
      selectedText = text;
      const rect = sel.getRangeAt(0).getBoundingClientRect();
      floatBtn.style.left = `${rect.left + window.scrollX}px`;
      floatBtn.style.top = `${rect.bottom + window.scrollY + 6}px`;
      floatBtn.style.display = 'block';
    } else if (!floatBtn.contains(e.target)) {
      floatBtn.style.display = 'none';
    }
  });

  floatBtn.onclick = async () => {
    floatBtn.style.display = 'none';
    panel.classList.add('open');
    currentTab = 'chat';
    tabs.forEach(t => t.classList.toggle('active', t.dataset.tab === 'chat'));

    chatHistory.push({ role: 'user', content: `请解释：「${selectedText}」` });
    renderChat();
    sendBtn.disabled = true;

    try {
      const res = await callAPI('explain_selection', {
        text: selectedText,
        question: '请解释这段内容，展开分析其含义和背景',
      });
      chatHistory.push({ role: 'assistant', content: res.explanation });
    } catch (e) {
      chatHistory.push({ role: 'assistant', content: `⚠️ ${e.message}` });
    }
    renderChat();
    sendBtn.disabled = false;
  };

  // ── 打开面板快捷键 (Ctrl+Shift+W) ──
  document.addEventListener('keydown', (e) => {
    if (e.ctrlKey && e.shiftKey && e.key === 'W') {
      e.preventDefault();
      panel.classList.toggle('open');
    }
  });

  // ── 注册菜单命令 ──
  GM_registerMenuCommand('📖 打开阅读助手', () => {
    panel.classList.add('open');
    renderBody();
  });

  GM_registerMenuCommand('📑 分析文章结构', () => {
    panel.classList.add('open');
    currentTab = 'sections';
    tabs.forEach(t => t.classList.toggle('active', t.dataset.tab === 'sections'));
    renderBody();
    if (!sectionsData) loadSections();
  });

  // ── 工具函数 ──
  function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str || '';
    return div.innerHTML;
  }

  function scrollToText(text) {
    // 在页面正文里找到这段文字并滚动到它
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, null);
    while (walker.nextNode()) {
      if (walker.currentNode.textContent.includes(text)) {
        const el = walker.currentNode.parentElement;
        if (el) {
          el.scrollIntoView({ behavior: 'smooth', block: 'center' });
          el.style.transition = 'background 0.3s';
          el.style.background = 'rgba(79,195,247,0.2)';
          setTimeout(() => { el.style.background = ''; }, 2000);
        }
        break;
      }
    }
  }

  // 初始化渲染
  renderBody();
  console.log('[Weft RSS] 阅读助手已加载');
})();
