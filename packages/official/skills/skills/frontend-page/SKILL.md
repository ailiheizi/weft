---
name: frontend-page
description: 生成美观现代的 HTML 网页/报告/仪表盘/展示页时使用。当用户要求"写网页"、"做一个 HTML 页面"、"生成报告"、"dashboard"、"展示页"、"前端"时触发。基于本技能的精致深色模板来写，而不是裸写朴素 HTML，这样产出的页面自带统一的现代设计风格。
---

# 前端页面生成

## 角色
前端页面生成器。产出**自带样式、视觉精致**的现代 HTML（深色主题、渐变标题、卡片、美化表格），而非黑字白底的朴素页面。

## 触发场景
用户要生成任何 HTML 页面：报告、仪表盘、展示页、说明页、数据可视化页。

## 核心规则
1. **基于本技能的样式模板写**，不要裸写朴素 HTML。
2. 一步到位 `fs_write` 完整 HTML：在 `<head>` 内嵌下面这套 `<style>`，`<body>` 用它的 class 填真实内容。不要停下来说"请稍候"，直接写完。
3. 填真实内容，删掉用不到的示例区块，不留占位文字。

## 必用的样式模板（复制到 `<head>` 的 `<style>`）

```css
:root{--bg:#0b0f17;--surface:#131925;--surface2:#1b2333;--border:#283142;--text:#e6edf6;--dim:#9aa7b8;--accent:#3b82f6;--ok:#22c55e;--radius:14px}
*{box-sizing:border-box;margin:0;padding:0}
body{font-family:-apple-system,'Segoe UI','Microsoft YaHei','PingFang SC',sans-serif;background:radial-gradient(1200px 600px at 70% -10%,#1a2436,var(--bg) 55%);color:var(--text);line-height:1.7;padding:48px 24px;min-height:100vh}
.container{max-width:880px;margin:0 auto}
.eyebrow{display:inline-block;font-size:13px;letter-spacing:.08em;text-transform:uppercase;color:var(--accent);background:rgba(59,130,246,.12);padding:4px 12px;border-radius:999px;margin-bottom:16px}
h1{font-size:38px;font-weight:700;background:linear-gradient(135deg,var(--text) 30%,var(--accent) 120%);-webkit-background-clip:text;background-clip:text;-webkit-text-fill-color:transparent;margin-bottom:12px}
.subtitle{font-size:18px;color:var(--dim)}
section{margin:36px 0}h2{font-size:24px;margin-bottom:16px}h3{font-size:18px;color:var(--text);margin:20px 0 10px}
p{color:var(--dim);margin-bottom:14px}a{color:var(--accent);text-decoration:none}strong{color:var(--text)}
.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(220px,1fr));gap:16px}
.card{background:var(--surface);border:1px solid var(--border);border-radius:var(--radius);padding:22px;box-shadow:0 8px 30px rgba(0,0,0,.35)}
.card .label{font-size:13px;color:var(--dim);margin-bottom:6px}.card .value{font-size:28px;font-weight:700}
ul,ol{padding-left:22px;color:var(--dim);margin-bottom:14px}li{margin:6px 0}
table{width:100%;border-collapse:collapse;margin:18px 0;background:var(--surface);border-radius:var(--radius);overflow:hidden;border:1px solid var(--border)}
th{background:var(--surface2);color:var(--text);font-weight:600;text-align:left;padding:12px 16px;font-size:14px}
td{padding:12px 16px;border-top:1px solid var(--border);color:var(--dim);font-size:14px}
code{background:var(--surface2);color:#f472b6;padding:2px 7px;border-radius:6px;font-family:Consolas,monospace;font-size:13px}
pre{background:#0a0e16;border:1px solid var(--border);color:#e2e8f0;padding:18px;border-radius:var(--radius);overflow-x:auto}
.badge{display:inline-block;font-size:12px;padding:3px 10px;border-radius:999px;background:rgba(34,197,94,.15);color:var(--ok);font-weight:600}
blockquote{border-left:3px solid var(--accent);background:var(--surface);padding:12px 18px;border-radius:0 var(--radius) var(--radius) 0;color:var(--dim);margin:16px 0}
```

## 结构骨架

```html
<body><div class="container">
  <header><span class="eyebrow">标签</span><h1>标题</h1><p class="subtitle">副标题</p></header>
  <section><h2>概览</h2><div class="grid">
    <div class="card"><div class="label">指标</div><div class="value">128</div></div>
  </div></section>
  <section><h2>详情</h2><p>正文...</p></section>
  <section><h2>数据</h2><table><thead><tr><th>列</th></tr></thead><tbody><tr><td>值 <span class="badge">正常</span></td></tr></tbody></table></section>
</div></body>
```

## 元素选用
- 指标/数据 → `.grid` + `.card`
- 表格数据 → `table`
- 状态标记 → `.badge`
- 代码 → `pre`/`code`，引用 → `blockquote`
