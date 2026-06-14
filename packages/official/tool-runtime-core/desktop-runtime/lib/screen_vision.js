/**
 * 屏幕视觉分析模块
 *
 * 能力：
 *   1. 浏览器截图（CDP，推荐）或全屏截图（PowerShell，降级）
 *   2. 自动滚动 + 多屏连续截图
 *   3. 逐屏调用 Claude Vision 提取信息
 *   4. 汇总聚合后生成 Word 文档
 */

const path         = require('path');
const fs           = require('fs');
const os           = require('os');
const { execSync } = require('child_process');
const axios        = require('axios');
const config       = require('./config');
const cdp          = require('./browser_cdp');

const WORK_DIR = process.env.DEEPSEEK_WORK_DIR || path.join(os.homedir(), 'Desktop');

function writableTempDir() {
  const candidates = [
    process.env.LOCALAPPDATA ? path.join(process.env.LOCALAPPDATA, 'Temp') : '',
    path.join(os.homedir(), 'AppData', 'Local', 'Temp'),
    os.tmpdir(),
    process.env.TEMP,
    process.env.TMP,
    process.env.WEFT_HOST_TMP_DIR,
  ]
    .filter(Boolean)
  for (const candidate of candidates) {
    try {
      fs.mkdirSync(candidate, { recursive: true })
      fs.accessSync(candidate, fs.constants.W_OK)
      return candidate
    } catch {}
  }
  return os.tmpdir()
}

// ── Vision API 调用 ───────────────────────────────────────────────

function runtimeRootFromPluginDir() {
  const pluginDir = process.env.WEFT_PACKAGE_DIR || '';
  if (pluginDir) return path.resolve(pluginDir, '..', '..', '..');
  return path.resolve(__dirname, '..', '..', '..', '..', '..');
}

function firstTomlString(block, key) {
  const match = block.match(new RegExp(`${key}\\s*=\\s*"([^"]+)"`));
  return match ? match[1].trim() : '';
}

function envArkApiKey(toml) {
  const envBlock = toml.match(/\[env\][\s\S]*?(?=\n\[[^\[]|$)/)?.[0] || '';
  return firstTomlString(envBlock, 'ARK_API_KEY');
}

function envArkVisionModel(toml) {
  const envBlock = toml.match(/\[env\][\s\S]*?(?=\n\[[^\[]|$)/)?.[0] || '';
  return firstTomlString(envBlock, 'ARK_VISION_MODEL');
}

function loadWeftCoreVisionProvider() {
  const configPath = path.join(runtimeRootFromPluginDir(), 'config', 'config.toml');
  if (!fs.existsSync(configPath)) return null;
  const toml = fs.readFileSync(configPath, 'utf8');
  const arkApiKey = envArkApiKey(toml);
  if (arkApiKey) {
    return {
      baseUrl: 'https://ark.cn-beijing.volces.com/api/v3',
      apiKey: arkApiKey,
      model: envArkVisionModel(toml) || 'doubao-seed-2-0-mini-260215',
      name: 'ark',
      format: 'responses',
    };
  }
  const providerBlocks = toml.match(/\[\[providers\]\][\s\S]*?(?=\n\[\[providers\]\]|\n\[[^\[]|$)/g) || [];
  for (const block of providerBlocks) {
    const name = firstTomlString(block, 'name').toLowerCase();
    const format = firstTomlString(block, 'format').toLowerCase();
    const supportsProviderConfig = ['openai', 'anthropic', 'claude', 'ark', 'doubao'].includes(name)
      || format === 'openai'
      || format === 'responses';
    if (!supportsProviderConfig) continue;
    const baseUrl = firstTomlString(block, 'base_url').replace(/\/$/, '');
    const keyMatch = block.match(/\[\[providers\.keys\]\][\s\S]*?value\s*=\s*"([^"]+)"/);
    const apiKey = keyMatch ? keyMatch[1].trim() : envArkApiKey(toml);
    const modelMatch = block.match(/models\s*=\s*\[\s*"([^"]+)"/);
    const model = modelMatch ? modelMatch[1].trim() : envArkVisionModel(toml);
    if (baseUrl && apiKey && model) return { baseUrl, apiKey, model, name, format };
  }
  return null;
}

function loadArkEnvVisionProvider() {
  const apiKey = (process.env.ARK_API_KEY || '').trim();
  if (!apiKey) return null;
  return {
    name: 'ark',
    format: 'responses',
    baseUrl: (process.env.ARK_BASE_URL || 'https://ark.cn-beijing.volces.com/api/v3').replace(/\/$/, ''),
    apiKey,
    model: (process.env.ARK_VISION_MODEL || 'doubao-seed-2-0-mini-260215').trim(),
  };
}

function isArkResponsesProvider(provider) {
  const baseUrl = String(provider.baseUrl || '').toLowerCase();
  const name = String(provider.name || '').toLowerCase();
  const model = String(provider.model || '').toLowerCase();
  return name === 'ark'
    || name === 'doubao'
    || baseUrl.includes('ark.cn-beijing.volces.com')
    || model.startsWith('doubao-');
}

function extractResponsesText(data) {
  if (typeof data?.output_text === 'string') return data.output_text;
  const chunks = [];
  for (const item of data?.output || []) {
    for (const content of item?.content || []) {
      if (typeof content?.text === 'string') chunks.push(content.text);
    }
  }
  return chunks.join('\n');
}

async function callArkResponsesVision(provider, imageBuffer, prompt, maxTokens = 2048) {
  const resp = await axios.post(
    `${provider.baseUrl}/responses`,
    {
      model: provider.model,
      max_output_tokens: maxTokens,
      input: [{
        role: 'user',
        content: [
          { type: 'input_image', image_url: `data:image/jpeg;base64,${imageBuffer.toString('base64')}` },
          { type: 'input_text', text: prompt },
        ],
      }],
    },
    {
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${provider.apiKey}` },
      timeout: 90000,
    }
  );
  return extractResponsesText(resp.data);
}

async function callOpenAiCompatibleVision(provider, imageBuffer, prompt, maxTokens = 2048) {
  if (isArkResponsesProvider(provider)) {
    return await callArkResponsesVision(provider, imageBuffer, prompt, maxTokens);
  }
  const resp = await axios.post(
    `${provider.baseUrl}/chat/completions`,
    {
      model: provider.model,
      max_tokens: maxTokens,
      messages: [{
        role: 'user',
        content: [
          { type: 'text', text: prompt },
          { type: 'image_url', image_url: { url: `data:image/jpeg;base64,${imageBuffer.toString('base64')}` } },
        ],
      }],
    },
    {
      headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${provider.apiKey}` },
      timeout: 90000,
    }
  );
  return resp.data.choices?.[0]?.message?.content || '';
}

/** 带图片的 Vision 分析 */
async function callVision(imageBuffer, prompt, maxTokens = 2048) {
  const weftProvider = loadArkEnvVisionProvider() || loadWeftCoreVisionProvider();
  if (weftProvider) {
    return await callOpenAiCompatibleVision(weftProvider, imageBuffer, prompt, maxTokens);
  }
  const resp = await axios.post(
    `${config.claude.baseUrl}/v1/messages`,
    {
      model: config.claude.model,
      max_tokens: maxTokens,
      messages: [{
        role: 'user',
        content: [
          { type: 'image', source: { type: 'base64', media_type: 'image/jpeg', data: imageBuffer.toString('base64') } },
          { type: 'text', text: prompt },
        ],
      }],
    },
    {
      headers: { 'Content-Type': 'application/json', 'x-api-key': config.claude.apiKey, 'anthropic-version': '2023-06-01' },
      timeout: 90000,
    }
  );
  return resp.data.content?.[0]?.text || '';
}

/** 纯文本汇总（无图片，用于最终聚合） */
async function callText(prompt, maxTokens = 4096) {
  const resp = await axios.post(
    `${config.claude.baseUrl}/v1/messages`,
    {
      model: config.claude.model,
      max_tokens: maxTokens,
      messages: [{ role: 'user', content: [{ type: 'text', text: prompt }] }],
    },
    {
      headers: { 'Content-Type': 'application/json', 'x-api-key': config.claude.apiKey, 'anthropic-version': '2023-06-01' },
      timeout: 90000,
    }
  );
  return resp.data.content?.[0]?.text || '';
}

// ── 截图 ──────────────────────────────────────────────────────────

/** 通过 CDP 截取浏览器当前页 */
async function takeBrowserShot() {
  return await cdp.screenshot();
}

/** 通过 PowerShell 截取全屏（降级方案） */
async function takeSystemShot() {
  const tmp = path.join(writableTempDir(), `czj_sv_${Date.now()}.jpg`);
  // 用 PNG 流写入内存再另存为 JPEG，完全避免 ImageCodecInfo 的引号问题
  const script = `
Add-Type -AssemblyName System.Windows.Forms,System.Drawing
$s=[System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$b=New-Object System.Drawing.Bitmap($s.Width,$s.Height)
$g=[System.Drawing.Graphics]::FromImage($b)
$g.CopyFromScreen($s.Location,[System.Drawing.Point]::Empty,$s.Size)
$ms=New-Object System.IO.MemoryStream
$b.Save($ms,[System.Drawing.Imaging.ImageFormat]::Jpeg)
$bytes=$ms.ToArray()
[System.IO.File]::WriteAllBytes('${tmp.replace(/\\/g, '\\\\')}', $bytes)
$g.Dispose();$b.Dispose();$ms.Dispose()
`.trim();
  // Base64-encode as UTF-16LE for -EncodedCommand (no file, no quoting issues)
  const encoded = Buffer.from(script, 'utf16le').toString('base64');
  execSync(`powershell -NoProfile -EncodedCommand ${encoded}`, { timeout: 15000 });
  const buf = fs.readFileSync(tmp);
  try { fs.unlinkSync(tmp); } catch {}
  return buf;
}

// ── 滚动 ──────────────────────────────────────────────────────────

async function scrollBrowser(amount = 800) {
  await cdp.scroll('down', amount);
}

function scrollSystem(targetProcess) {
  // 先激活目标窗口（WPS 等），再发 PgDn，避免焦点丢失
  const procNames = targetProcess
    ? [targetProcess]
    : ['wps', 'et', 'wpp', 'WINWORD', 'EXCEL', 'notepad'];

  const script = `
Add-Type -AssemblyName System.Windows.Forms
$names = @(${procNames.map(n => `'${n}'`).join(',')})
$proc = $null
foreach ($n in $names) {
  $proc = Get-Process -Name $n -ErrorAction SilentlyContinue | Where-Object {$_.MainWindowHandle -ne 0} | Select-Object -First 1
  if ($proc) { break }
}
if ($proc) {
  $sh = New-Object -ComObject WScript.Shell
  $sh.AppActivate($proc.Id) | Out-Null
  Start-Sleep -Milliseconds 400
}
[System.Windows.Forms.SendKeys]::SendWait('{PGDN}')
Start-Sleep -Milliseconds 300
`.trim();
  const encoded = Buffer.from(script, 'utf16le').toString('base64');
  execSync(`powershell -NoProfile -EncodedCommand ${encoded}`, { timeout: 8000 });
}

// ── 主函数 ────────────────────────────────────────────────────────

/**
 * 多屏截图分析并归档为 Word 文档
 *
 * @param {string} task         用户任务描述（原始语音文字）
 * @param {object} opts
 * @param {number} opts.scrollCount   滚动次数，默认 5（总共截 scrollCount+1 屏）
 * @param {number} opts.scrollAmount  每次滚动像素，默认 800
 * @param {number} opts.delayMs       截图间隔毫秒，默认 1500
 * @param {boolean} opts.forceSystem  强制系统截图（非浏览器场景）
 * @param {Function} opts.onProgress  进度回调 (step, total, msg)
 */
async function analyzeAndArchive(task, opts = {}) {
  const {
    scrollCount     = 5,
    scrollAmount    = 800,
    delayMs         = 1500,
    forceSystem     = false,
    targetProcess   = null,   // 指定要激活/滚动的进程名（如 'wps'）
    onProgress      = () => {},
  } = opts;

  // 确定截图模式
  let useBrowser = !forceSystem;
  if (useBrowser) {
    const reachable = await cdp.isCdpReachable();
    if (!reachable) {
      console.log('[ScreenVision] CDP 不可达，降级系统截图');
      useBrowser = false;
    }
  }
  console.log(`[ScreenVision] 模式: ${useBrowser ? 'CDP浏览器' : '系统全屏'}`);

  const totalScreens = scrollCount + 1;
  const totalSteps   = totalScreens + 1; // +1 for final synthesis
  const extracted    = [];

  // ── 逐屏截图循环 ──────────────────────────────────────────────
  for (let i = 0; i < totalScreens; i++) {
    onProgress(i + 1, totalSteps, `第 ${i + 1}/${totalScreens} 屏，截图分析中…`);

    // 截图
    let buf;
    try {
      buf = useBrowser ? await takeBrowserShot() : await takeSystemShot();
    } catch (e) {
      if (useBrowser) {
        console.warn('[ScreenVision] CDP截图失败，降级系统截图:', e.message);
        useBrowser = false;
        buf = await takeSystemShot();
      } else {
        throw e;
      }
    }

    // Vision 分析
    const prompt = i === 0
      ? `这是屏幕截图。用户任务：${task}

请从截图中提取所有可见的相关信息，要求：
- 逐条列出每个条目（商品/文章/数据等）
- 保留所有关键字段（名称、价格、销量/月销、评分、店铺、标签等）
- 不要总结归纳，原始提取
- 如果是商品列表，每个商品独占一条
- 格式：「序号. 名称 | 价格 | 销量 | 其他」`
      : `这是向下滚动后的第 ${i + 1} 屏截图，继续提取信息。

任务：${task}

要求：
- 只提取本屏新出现的内容，跳过已在上方出现过的重复商品
- 格式与前面一致：「序号. 名称 | 价格 | 销量 | 其他」
- 如果整屏内容与前面重复，输出「（本屏无新内容）」`;

    // Vision API 调用，429 时自动重试（最多 2 次，间隔 8s）
    let result = '';
    for (let attempt = 0; attempt < 3; attempt++) {
      try {
        result = await callVision(buf, prompt, 2048);
        break;
      } catch (e) {
        const is429 = e?.response?.status === 429 || /429/.test(e.message);
        if (is429 && attempt < 2) {
          const wait = (attempt + 1) * 8000;
          console.warn(`[ScreenVision] 429限流，${wait/1000}s后重试…`);
          await new Promise(r => setTimeout(r, wait));
        } else {
          throw e;
        }
      }
    }
    extracted.push({ screen: i + 1, content: result });
    console.log(`[ScreenVision] 第${i + 1}屏完成 (${result.length}字)`);

    // 滚动到下一屏 + API 冷却间隔
    if (i < totalScreens - 1) {
      try {
        if (useBrowser) await scrollBrowser(scrollAmount);
        else scrollSystem(targetProcess);
      } catch (e) {
        console.warn('[ScreenVision] 滚动失败，继续:', e.message);
      }
      // 截图间隔 + API 冷却（至少 4s，避免连续请求触发限流）
      await new Promise(r => setTimeout(r, Math.max(delayMs, 4000)));
    }
  }

  // ── 汇总合成 ─────────────────────────────────────────────────
  onProgress(totalSteps, totalSteps, '汇总生成文档中…');

  const rawText = extracted
    .map(d => `【第${d.screen}屏数据】\n${d.content}`)
    .join('\n\n---\n\n');

  const synthPrompt = `以下是从屏幕连续截图中逐屏提取的原始数据：

${rawText}

---
用户原始任务：${task}
分析时间：${new Date().toLocaleString('zh-CN')}

请将上述数据整合成一份完整的分析报告：

1. 合并重复条目，保留最完整的信息
2. 按热度/销量/相关性排序（有数据时）
3. 输出格式为 Markdown，结构如下：

# [报告标题，简洁概括品类/主题]

## 概述
（分析时间、品类、数据量、来源等）

## 完整列表
| 序号 | 名称 | 价格 | 销量/月销 | 评分 | 店铺 | 备注 |
|------|------|------|---------|------|------|------|
（所有商品，一行一个）

## 价格分析
（价格区间、均价、高低端分布等）

## 品牌/商家分布
（主要品牌、头部商家等）

## 关键发现
（3-5条核心洞察）

要求：数字加粗，语言简练，保留所有真实数据不编造`;

  const report = await callText(synthPrompt, 4096);

  // ── 生成 Word ─────────────────────────────────────────────────
  const docResult = await genWordDoc(report, task);

  return {
    success: true,
    content: docResult.content,
    files:   docResult.files,
    screens: totalScreens,
    report,
  };
}

// ── Word 文档生成 ─────────────────────────────────────────────────

async function genWordDoc(markdown, userTask) {
  const {
    Document, Packer, Paragraph, TextRun, HeadingLevel, AlignmentType, convertInchesToTwip,
  } = require('docx');

  if (!fs.existsSync(WORK_DIR)) fs.mkdirSync(WORK_DIR, { recursive: true });

  const titleM    = markdown.match(/^#\s+(.+)/m);
  const title     = titleM?.[1]?.trim() || userTask.substring(0, 30).replace(/[\\/:*?"<>|]/g, '') || '屏幕分析报告';
  const mdContent = markdown.replace(/^#\s+.+\n?/, '').trim();

  const children = [];
  children.push(new Paragraph({
    children: [new TextRun({ text: title, bold: true, size: 52, color: '1F6FEB' })],
    alignment: AlignmentType.CENTER,
    spacing: { before: 600, after: 400 },
  }));
  children.push(new Paragraph({
    children: [new TextRun({ text: new Date().toLocaleDateString('zh-CN'), size: 22, color: '8B949E' })],
    alignment: AlignmentType.CENTER,
    spacing: { after: 600 },
  }));

  for (const line of mdContent.split('\n')) {
    const t = line.trim();
    if (!t) { children.push(new Paragraph({ spacing: { after: 80 } })); continue; }
    if      (t.startsWith('### ')) children.push(new Paragraph({ text: t.slice(4),  heading: HeadingLevel.HEADING_3, spacing: { before: 180, after: 80  } }));
    else if (t.startsWith('## '))  children.push(new Paragraph({ text: t.slice(3),  heading: HeadingLevel.HEADING_2, spacing: { before: 260, after: 120 } }));
    else if (t.startsWith('# '))   children.push(new Paragraph({ text: t.slice(2),  heading: HeadingLevel.HEADING_1, spacing: { before: 360, after: 180 } }));
    else if (/^[-*•]\s/.test(t))   children.push(new Paragraph({ bullet: { level: 0 }, children: inlineRuns(t.replace(/^[-*•]\s/, '')), spacing: { after: 60 } }));
    else if (/^\|/.test(t) && !t.match(/^\|[-: ]+\|/)) // 表格行（跳过分隔行）
      children.push(new Paragraph({ children: inlineRuns(t.replace(/^\||\|$/g, '').split('|').map(c => c.trim()).join('    ')), spacing: { after: 60 } }));
    else                           children.push(new Paragraph({ children: inlineRuns(t), spacing: { after: 120 } }));
  }

  const doc = new Document({
    sections: [{
      properties: { page: { margin: { top: convertInchesToTwip(1), bottom: convertInchesToTwip(1), left: convertInchesToTwip(1.25), right: convertInchesToTwip(1.25) } } },
      children,
    }],
  });

  const ts       = new Date().toISOString().replace(/[:.]/g, '-').substring(0, 16);
  const filename = `${title}_${ts}.docx`.replace(/[\\/:*?"<>|]/g, '_');
  const fpath    = path.join(WORK_DIR, filename);
  fs.writeFileSync(fpath, await Packer.toBuffer(doc));
  console.log(`[ScreenVision] 📄 Word: ${fpath}`);

  const summary = markdown.replace(/^#+\s*/gm, '').substring(0, 200).replace(/\n/g, ' ');
  return {
    success: true,
    content: `分析报告已生成：${title}\n文件：${fpath}\n\n摘要：${summary}……`,
    files:   [fpath],
  };
}

function inlineRuns(text) {
  const { TextRun } = require('docx');
  const runs  = [];
  const parts = text.split(/(\*\*[^*]+\*\*|\*[^*]+\*|`[^`]+`)/);
  for (const p of parts) {
    if (!p) continue;
    if      (p.startsWith('**') && p.endsWith('**')) runs.push(new TextRun({ text: p.slice(2, -2), bold: true }));
    else if (p.startsWith('*')  && p.endsWith('*'))  runs.push(new TextRun({ text: p.slice(1, -1), italics: true }));
    else if (p.startsWith('`')  && p.endsWith('`'))  runs.push(new TextRun({ text: p.slice(1, -1), font: 'Courier New', size: 18 }));
    else                                              runs.push(new TextRun({ text: p }));
  }
  return runs.length ? runs : [new TextRun({ text })];
}

// ── 眨眼：单次截图 + Vision 分析（不生成 Word） ──────────────────────
/**
 * 单次截图并用 Claude Vision 分析，返回文本结果。
 * 用于"眨眼"功能：proactive 确认后立即看一眼屏幕。
 *
 * @param {string}   task         用户任务描述
 * @param {object}   opts
 * @param {boolean}  opts.forceSystem  强制系统截图（WPS/非浏览器场景）
 * @param {number}   opts.screenIndex  第几张截图（1起），用于构造 prompt
 * @param {string[]} opts.prevSummaries 前几屏的摘要，用于多屏拼接提示
 */
async function blinkOnce(task, opts = {}) {
  const { forceSystem = false, screenIndex = 1, prevSummaries = [] } = opts;

  let useBrowser = !forceSystem;
  if (useBrowser) {
    const reachable = await cdp.isCdpReachable();
    if (!reachable) useBrowser = false;
  }
  console.log(`[眨眼] 第${screenIndex}张 | 模式: ${useBrowser ? 'CDP' : '系统全屏'}`);

  let buf;
  try {
    buf = useBrowser ? await takeBrowserShot() : await takeSystemShot();
  } catch (e) {
    if (useBrowser) { useBrowser = false; buf = await takeSystemShot(); }
    else throw e;
  }

  const prevCtx = prevSummaries.length
    ? `\n\n前面已分析的内容摘要：\n${prevSummaries.map((s, i) => `【第${i + 1}张】${s}`).join('\n')}\n\n请只提取本张截图中新出现的内容。`
    : '';

  const prompt = `companion-context screen glance + full-screen OCR.
这是第${screenIndex}张屏幕截图。用户任务：${task}${prevCtx}

请一次性返回 companion 可直接使用的屏幕上下文，必须按这个顺序：

1. 画面概览：用 2-4 句中文描述屏幕上正在发生什么。优先描述主画面、人物/物体、动作、状态、明显 UI 区域。如果是视频/游戏，描述当前画面内容，不要只复述页面标题。
2. 整屏可见文字 OCR：抄录屏幕里能看清的所有文字，尽量保持原顺序和分区。看不清的不要编造，可以标注「看不清」。

不要调用或建议调用别的工具。不要写分析报告，不要猜测截图外的信息。`;

  let result = '';
  for (let attempt = 0; attempt < 3; attempt++) {
    try {
      result = await callVision(buf, prompt, 5000);
      break;
    } catch (e) {
      const is429 = e?.response?.status === 429 || /429/.test(e.message);
      if (is429 && attempt < 2) {
        const wait = (attempt + 1) * 8000;
        console.warn(`[眨眼] 429限流，${wait / 1000}s后重试…`);
        await new Promise(r => setTimeout(r, wait));
      } else throw e;
    }
  }

  console.log(`[眨眼] 第${screenIndex}张完成 (${result.length}字)`);
  return { content: result, useBrowser };
}

module.exports = { analyzeAndArchive, blinkOnce, takeBrowserShot, takeSystemShot };
