---
name: lottie-animation
description: 需要动效/动画展示时使用。当用户要求"动画"、"动效"、"loading 动画"、"流程动画"、"动态展示"、"lottie"、"让它动起来"时触发。基于本技能用 lottie-web 在深色主题里播放矢量动画,而非静态图。
---

# Lottie 动画展示

## 角色
动画展示生成器。用 **lottie-web** 播放矢量动画(JSON 驱动),适合 loading 态、流程演示、庆祝动效、图标动画等。配深色主题。

## 触发场景
用户要"动起来"的效果:加载动画、成功/完成庆祝、流程步骤动效、品牌动画。

## 核心规则
1. 一步到位 `fs_write` 完整自包含 `.html`,内嵌 lottie-web(CDN)+ 深色样式。
2. **Lottie JSON 来源**(按可得性优先):
   - 用户提供了 .json → 直接内嵌或 path 加载。
   - 用 LottieFiles 公开动画 URL(`lottie.host` / `assets*.lottiefiles.com` 的 .json),用 `path:` 加载。
   - 简单几何动效可手写极简 Lottie JSON。
3. 不要停下说"请稍候"。配文字说明动画含义。

## 必用模板

```html
<!DOCTYPE html><html><head><meta charset="utf-8">
<script src="https://cdnjs.cloudflare.com/ajax/libs/lottie-web/5.12.2/lottie.min.js"></script>
<style>
:root{--bg:#0b0f17;--surface:#131925;--border:#283142;--text:#e6edf6;--dim:#9aa7b8;--accent:#3b82f6;--radius:14px}
*{box-sizing:border-box;margin:0;padding:0}
body{font-family:-apple-system,'Segoe UI','Microsoft YaHei','PingFang SC',sans-serif;background:radial-gradient(1200px 600px at 70% -10%,#1a2436,var(--bg) 55%);color:var(--text);min-height:100vh;display:flex;flex-direction:column;align-items:center;justify-content:center;padding:40px}
h1{font-size:30px;font-weight:700;background:linear-gradient(135deg,var(--text) 30%,var(--accent) 120%);-webkit-background-clip:text;background-clip:text;-webkit-text-fill-color:transparent;margin-bottom:8px;text-align:center}
.subtitle{font-size:16px;color:var(--dim);margin-bottom:28px;text-align:center}
.stage{width:420px;max-width:90vw;height:420px;background:var(--surface);border:1px solid var(--border);border-radius:var(--radius)}
</style></head>
<body>
  <h1>标题</h1><p class="subtitle">动画说明</p>
  <div id="anim" class="stage"></div>
<script>
  lottie.loadAnimation({
    container: document.getElementById('anim'),
    renderer: 'svg', loop: true, autoplay: true,
    // 方式A:公开 URL
    path: 'https://lottie.host/REPLACE_WITH_REAL.json'
    // 方式B:内嵌 JSON(用户提供时):  animationData: { ...lottie json... }
  });
</script>
</body></html>
```

## 提示
- 优先用 `path` 加载公开 Lottie URL(体积小、可靠)。
- 用户给了 JSON 就用 `animationData` 内嵌(去掉 `path`)。
- loop/autoplay 默认开;一次性动效设 `loop:false`。
