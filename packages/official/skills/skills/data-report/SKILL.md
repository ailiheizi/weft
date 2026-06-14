---
name: data-report
description: 用图表可视化数据时使用。当用户要求"图表"、"可视化"、"趋势图"、"统计图"、"数据分析报告"、"chart"、"plot"、"图形展示数据"时触发。基于本技能用 Plotly 在深色主题里画交互图表(折线/柱状/饼/散点),而非只列表格或裸数字。
---

# 数据可视化报告

## 角色
数据可视化生成器。把数据用 **Plotly 交互图表**呈现(折线、柱状、饼图、散点、热力图),配深色主题,而非只给表格或纯数字。

## 触发场景
用户要看数据趋势/对比/分布/占比;有一组数值要"画出来"而非"列出来"。

## 核心规则
1. 一步到位 `fs_write` 一个完整自包含 `.html`,内嵌 Plotly(CDN)+ 深色样式。
2. 用真实数据填 `data`/`layout`,删示例。不要停下说"请稍候"。
3. 选对图型:趋势→折线;对比→柱状;占比→饼图;相关→散点;矩阵→热力图。
4. 多个指标可放多个 `<div>` 图 + 顶部指标卡片概览。

## 必用模板(复制后填真实数据)

```html
<!DOCTYPE html><html><head><meta charset="utf-8">
<script src="https://cdn.plot.ly/plotly-2.35.2.min.js"></script>
<style>
:root{--bg:#0b0f17;--surface:#131925;--surface2:#1b2333;--border:#283142;--text:#e6edf6;--dim:#9aa7b8;--accent:#3b82f6;--radius:14px}
*{box-sizing:border-box;margin:0;padding:0}
body{font-family:-apple-system,'Segoe UI','Microsoft YaHei','PingFang SC',sans-serif;background:radial-gradient(1200px 600px at 70% -10%,#1a2436,var(--bg) 55%);color:var(--text);padding:40px 24px;min-height:100vh}
.container{max-width:980px;margin:0 auto}
.eyebrow{display:inline-block;font-size:13px;letter-spacing:.08em;text-transform:uppercase;color:var(--accent);background:rgba(59,130,246,.12);padding:4px 12px;border-radius:999px;margin-bottom:16px}
h1{font-size:34px;font-weight:700;background:linear-gradient(135deg,var(--text) 30%,var(--accent) 120%);-webkit-background-clip:text;background-clip:text;-webkit-text-fill-color:transparent;margin-bottom:8px}
.subtitle{font-size:17px;color:var(--dim);margin-bottom:24px}
.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:14px;margin-bottom:28px}
.card{background:var(--surface);border:1px solid var(--border);border-radius:var(--radius);padding:18px}
.card .label{font-size:13px;color:var(--dim);margin-bottom:6px}.card .value{font-size:26px;font-weight:700}
.chart{background:var(--surface);border:1px solid var(--border);border-radius:var(--radius);padding:14px;margin-bottom:20px}
</style></head>
<body><div class="container">
  <span class="eyebrow">数据报告</span>
  <h1>标题</h1><p class="subtitle">副标题/数据说明</p>
  <div class="grid">
    <div class="card"><div class="label">指标</div><div class="value">128</div></div>
  </div>
  <div class="chart"><div id="chart1" style="height:380px"></div></div>
</div>
<script>
// Plotly 深色主题统一布局
const DARK = {
  paper_bgcolor:'rgba(0,0,0,0)', plot_bgcolor:'rgba(0,0,0,0)',
  font:{color:'#9aa7b8'}, margin:{t:30,r:20,b:40,l:50},
  xaxis:{gridcolor:'#283142'}, yaxis:{gridcolor:'#283142'},
  colorway:['#3b82f6','#22c55e','#f59e0b','#ef4444','#a855f7']
};
// 用真实数据替换:
Plotly.newPlot('chart1',
  [{x:['一月','二月','三月'], y:[10,25,18], type:'scatter', mode:'lines+markers', name:'示例'}],
  {...DARK, title:'图表标题'},
  {responsive:true, displayModeBar:false});
</script>
</body></html>
```

## 图型速查
- 折线 `type:'scatter', mode:'lines'` — 时间趋势
- 柱状 `type:'bar'` — 分类对比
- 饼图 `type:'pie', values:[...], labels:[...]` — 占比
- 散点 `type:'scatter', mode:'markers'` — 相关性
- 热力图 `type:'heatmap', z:[[...]]` — 矩阵/密度
