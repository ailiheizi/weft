/**
 * OpenCLI 风格的浏览器自动化控制器
 * 灵感来源：opencli 开源项目
 *
 * 通过 Chrome DevTools Protocol (CDP) 直接控制 Chrome 浏览器。
 * 功能对标 opencli 的 operate 命令：
 *   - navigate(url)       → operate open <url>
 *   - click(selector)     → operate click <selector>
 *   - type(selector,text) → operate type <selector> <text>
 *   - screenshot()        → operate screenshot
 *   - eval(js)            → operate eval <js>
 *   - getState()          → operate state
 *   - scroll(dir, amount) → operate scroll
 *
 * 不依赖 Playwright，直接用 ws 包通过 WebSocket 连接 Chrome CDP。
 * 自动发现或启动 Chrome（headless 模式）。
 */

const http          = require('http');
const { spawn }     = require('child_process');
const WebSocket     = require('ws');
const fs            = require('fs');
const path          = require('path');
const os            = require('os');

// ── 配置 ──────────────────────────────────────────────────────────

const CDP_PORT = parseInt(process.env.CHROME_CDP_PORT) || 9222;

// Chrome 候选路径
const CHROME_CANDIDATES = [
  process.env.CHROME_PATH,
  'C:\\Users\\Administrator\\AppData\\Local\\Google\\Chrome\\Application\\chrome.exe',
  'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
  'C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe',
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  '/usr/bin/google-chrome',
  '/usr/bin/chromium-browser',
].filter(Boolean);

function findChrome() {
  for (const p of CHROME_CANDIDATES) {
    if (fs.existsSync(p)) return p;
  }
  return null;
}

// ── CDP 连接管理 ──────────────────────────────────────────────────

let _chromePid  = null;   // spawn 的 Chrome 进程 PID
let _ws         = null;   // 当前 WebSocket 连接
let _msgId      = 0;
const _pending  = new Map(); // msgId → { resolve, reject }
let _sessionId  = null;

/** 检测 CDP 端口是否可达 */
function isCdpReachable() {
  return new Promise(resolve => {
    const req = http.get(`http://localhost:${CDP_PORT}/json`, res => {
      resolve(res.statusCode === 200);
    });
    req.on('error', () => resolve(false));
    req.setTimeout(1000, () => { req.destroy(); resolve(false); });
  });
}

/** 等待 CDP 端口就绪（轮询） */
async function waitForCdp(maxMs = 10000) {
  const start = Date.now();
  while (Date.now() - start < maxMs) {
    if (await isCdpReachable()) return true;
    await new Promise(r => setTimeout(r, 500));
  }
  return false;
}

/** 启动 Chrome（headless 模式，后台运行） */
async function launchChrome() {
  const chromePath = findChrome();
  if (!chromePath) throw new Error('未找到 Chrome 安装，请设置 CHROME_PATH 环境变量');

  const tmpProfile = path.join(os.tmpdir(), 'czj-chrome-cdp');
  fs.mkdirSync(tmpProfile, { recursive: true });

  console.log(`[BrowserCDP] 启动 Chrome: ${chromePath}`);
  const proc = spawn(chromePath, [
    `--remote-debugging-port=${CDP_PORT}`,
    `--user-data-dir=${tmpProfile}`,
    '--headless=new',
    '--no-sandbox',
    '--disable-gpu',
    '--disable-extensions',
    '--disable-web-security',
  ], { detached: false, stdio: 'ignore' });

  _chromePid = proc.pid;
  proc.on('exit', () => { _chromePid = null; _ws = null; });

  const ready = await waitForCdp(10000);
  if (!ready) throw new Error('Chrome CDP 端口未就绪（超时 10s）');
  console.log(`[BrowserCDP] Chrome 已启动 (pid=${_chromePid}, port=${CDP_PORT})`);
}

/** 获取第一个可用页面的 WebSocket Debugger URL */
async function getPageWsUrl() {
  return new Promise((resolve, reject) => {
    const req = http.get(`http://localhost:${CDP_PORT}/json`, res => {
      let raw = '';
      res.on('data', d => raw += d);
      res.on('end', () => {
        try {
          const targets = JSON.parse(raw);
          const page = targets.find(t => t.type === 'page');
          if (!page) reject(new Error('未找到 page target'));
          else resolve(page.webSocketDebuggerUrl);
        } catch (e) {
          reject(e);
        }
      });
    });
    req.on('error', reject);
  });
}

/** 建立 CDP WebSocket 连接 */
async function connectCdp() {
  if (_ws && _ws.readyState === WebSocket.OPEN) return;

  if (!(await isCdpReachable())) {
    await launchChrome();
  }

  const wsUrl = await getPageWsUrl();
  _ws = new WebSocket(wsUrl);

  await new Promise((resolve, reject) => {
    _ws.once('open', resolve);
    _ws.once('error', reject);
    setTimeout(() => reject(new Error('CDP WebSocket 连接超时')), 5000);
  });

  _ws.on('message', raw => {
    try {
      const msg = JSON.parse(raw);
      if (msg.id && _pending.has(msg.id)) {
        const { resolve, reject } = _pending.get(msg.id);
        _pending.delete(msg.id);
        if (msg.error) reject(new Error(msg.error.message || JSON.stringify(msg.error)));
        else resolve(msg.result);
      }
    } catch {}
  });

  _ws.on('close', () => { _ws = null; });
  _ws.on('error', e => console.warn('[BrowserCDP] WS error:', e.message));

  // 启用 Page、Runtime domain
  await call('Page.enable');
  await call('Runtime.enable');
  console.log('[BrowserCDP] CDP 连接成功');
}

/** 发送 CDP 命令，返回 Promise<result> */
async function call(method, params = {}) {
  await connectCdp();
  return new Promise((resolve, reject) => {
    const id = ++_msgId;
    _pending.set(id, { resolve, reject });
    const msg = JSON.stringify({ id, method, params });
    if (_ws.readyState !== WebSocket.OPEN) {
      _pending.delete(id);
      reject(new Error('CDP WebSocket 未连接'));
      return;
    }
    _ws.send(msg);
    // 30s 超时
    setTimeout(() => {
      if (_pending.has(id)) {
        _pending.delete(id);
        reject(new Error(`CDP 命令超时: ${method}`));
      }
    }, 30000);
  });
}

// ── 页面操作 API ──────────────────────────────────────────────────

/**
 * 导航到指定 URL
 * @param {string} url
 */
async function navigate(url) {
  if (!url.startsWith('http')) url = 'https://' + url;
  console.log(`[BrowserCDP] navigate → ${url}`);
  await call('Page.navigate', { url });
  // 等待页面加载
  await new Promise(resolve => {
    const timer = setTimeout(resolve, 10000);
    const listener = (raw) => {
      try {
        const msg = JSON.parse(raw);
        if (msg.method === 'Page.loadEventFired') {
          clearTimeout(timer);
          if (_ws) _ws.off('message', listener);
          resolve();
        }
      } catch {}
    };
    if (_ws) _ws.on('message', listener);
    else resolve();
  });
}

/**
 * 获取当前页面状态
 * @returns {Promise<{url, title, readyState}>}
 */
async function getState() {
  const result = await call('Runtime.evaluate', {
    expression: `JSON.stringify({url: location.href, title: document.title, readyState: document.readyState})`,
    returnByValue: true,
  });
  try {
    return JSON.parse(result.result?.value || '{}');
  } catch {
    return { url: '', title: '', readyState: 'unknown' };
  }
}

/**
 * 截图（返回 Buffer）
 * @returns {Promise<Buffer>}
 */
async function screenshot(opts = {}) {
  const quality = Math.max(10, Math.min(100, Number(opts.quality || 70)));
  const result = await call('Page.captureScreenshot', { format: 'jpeg', quality });
  return Buffer.from(result.data, 'base64');
}

/**
 * 执行 JavaScript 并返回结果
 * @param {string} expression
 * @returns {Promise<any>}
 */
async function evaluate(expression) {
  const result = await call('Runtime.evaluate', {
    expression,
    returnByValue: true,
    awaitPromise: true,
  });
  if (result.exceptionDetails) {
    throw new Error(result.exceptionDetails.text || 'JS 执行异常');
  }
  return result.result?.value;
}

/**
 * 点击指定 CSS 选择器的元素
 * @param {string} selector
 */
async function click(selector) {
  const escaped = selector.replace(/'/g, "\\'");
  const rect = await evaluate(`
    (() => {
      const el = document.querySelector('${escaped}');
      if (!el) throw new Error('找不到元素: ${escaped}');
      const r = el.getBoundingClientRect();
      el.scrollIntoView({ block: 'center' });
      return { x: r.left + r.width/2, y: r.top + r.height/2 };
    })()
  `);

  if (!rect) throw new Error(`元素不存在: ${selector}`);

  await call('Input.dispatchMouseEvent', { type: 'mousePressed', x: rect.x, y: rect.y, button: 'left', clickCount: 1 });
  await call('Input.dispatchMouseEvent', { type: 'mouseReleased', x: rect.x, y: rect.y, button: 'left', clickCount: 1 });
}

/**
 * 在指定元素中输入文字
 * @param {string} selector
 * @param {string} text
 */
async function type(selector, text) {
  await click(selector);
  await new Promise(r => setTimeout(r, 200));
  // 清除已有内容
  await call('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Control', modifiers: 0 });
  await call('Input.dispatchKeyEvent', { type: 'keyDown', key: 'a', modifiers: 2 }); // Ctrl+A
  await call('Input.dispatchKeyEvent', { type: 'keyUp', key: 'a', modifiers: 2 });
  await call('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Control', modifiers: 0 });

  // 逐字符输入
  for (const char of text) {
    await call('Input.dispatchKeyEvent', { type: 'keyDown', text: char });
    await call('Input.dispatchKeyEvent', { type: 'keyUp', text: char });
  }
}

/**
 * 滚动页面
 * @param {'up'|'down'} direction
 * @param {number} amount - 像素数
 */
async function scroll(direction = 'down', amount = 500) {
  const delta = direction === 'up' ? -amount : amount;
  await call('Input.dispatchMouseEvent', {
    type: 'mouseWheel',
    x: 400, y: 400,
    deltaX: 0,
    deltaY: delta,
  });
}

/**
 * 等待元素出现（CSS 选择器）
 * @param {string} selector
 * @param {number} timeoutMs
 */
async function waitForElement(selector, timeoutMs = 5000) {
  const start = Date.now();
  const escaped = selector.replace(/'/g, "\\'");
  while (Date.now() - start < timeoutMs) {
    const found = await evaluate(`!!document.querySelector('${escaped}')`);
    if (found) return;
    await new Promise(r => setTimeout(r, 300));
  }
  throw new Error(`等待元素超时: ${selector}`);
}

/**
 * 关闭 CDP 连接（不关闭 Chrome 进程）
 */
function close() {
  if (_ws) {
    _ws.close();
    _ws = null;
  }
  console.log('[BrowserCDP] 连接已关闭');
}

/** 是否已启动 Chrome */
function isLaunched() {
  return !!_chromePid;
}

/** 是否已连接 CDP */
function isConnected() {
  return !!_ws && _ws.readyState === WebSocket.OPEN;
}

module.exports = {
  navigate,
  getState,
  screenshot,
  evaluate,
  click,
  type,
  scroll,
  waitForElement,
  close,
  isLaunched,
  isConnected,
  isCdpReachable,
  CDP_PORT,
};
