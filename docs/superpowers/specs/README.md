# Specs — 设计稿索引

> 父文档：[`../README.md`](../README.md)
> 用途：功能 / 模块的设计与契约

## 状态约定

- ✅ active：当前主线仍在用 / 仍是参考
- 🔄 partial：部分仍有效，部分被新文档覆盖
- 📦 archived：已落地或被替代，保留作历史
- ❌ superseded：被新文档完全替代

## 索引（按日期倒序）

| 日期 | 文件 | 主题 | 状态 |
|---|---|---|---|
| 2026-05-09 | [`agent-runtime-refactor-design`](./2026-05-09-agent-runtime-refactor-design.md) | Agent runtime 重构设计 | ✅ active |
| 2026-04-26 | [`openclaw-web-search-provider-design`](./2026-04-26-openclaw-web-search-provider-design.md) | OpenClaw Web 搜索 provider 设计 | ✅ active |
| 2026-04-24 | [`ai-local-scout-game-provider-depth-design`](./2026-04-24-ai-local-scout-game-provider-depth-design.md) | 本地游戏 scout 深度（Steam/Epic/GOG/EA 等） | 🔄 partial |
| 2026-04-24 | [`ai-local-scout-meaningful-breadth-design`](./2026-04-24-ai-local-scout-meaningful-breadth-design.md) | 本地 scout 广度采集设计 | 🔄 partial |
| 2026-04-15 | [`weft-code-design`](./2026-04-15-weft-code-design.md) | WEFT Code 产品设计 | ✅ active |
| 2026-04-15 | [`weft-workspace-session-design`](./2026-04-15-weft-workspace-session-design.md) | Workspace / Session 模型 | ✅ active |
| 2026-04-11 | [`memory-runtime-mempalace-bridge-design`](./2026-04-11-memory-runtime-mempalace-bridge-design.md) | Memory Runtime ↔ Mempalace 桥接 | ✅ active |
| 2026-04-11 | [`memory-runtime-preference-card-benchmark-adapter-design`](./2026-04-11-memory-runtime-preference-card-benchmark-adapter-design.md) | 偏好卡 benchmark adapter | ✅ active |
| 2026-04-10 | [`weft-claw-package-blueprint-design`](./2026-04-10-weft-claw-package-blueprint-design.md) | WEFT Claw 产品包蓝图 | ✅ active |
| 2026-04-06 | [`memory-runtime-precise-long-term-memory-design`](./2026-04-06-memory-runtime-precise-long-term-memory-design.md) | 精确长期记忆设计 | ✅ active |

## 推荐阅读顺序

### 想理解 WEFT Claw 产品
1. `2026-04-10-weft-claw-package-blueprint-design.md`

### 想理解 WEFT Code 产品
1. `2026-04-15-weft-code-design.md`
2. `2026-04-15-weft-workspace-session-design.md`

### 想理解 Memory Runtime
1. `2026-04-06-memory-runtime-precise-long-term-memory-design.md`
2. `2026-04-11-memory-runtime-mempalace-bridge-design.md`
3. `2026-04-11-memory-runtime-preference-card-benchmark-adapter-design.md`

### 想理解 Agent Runtime
1. `2026-05-09-agent-runtime-refactor-design.md`

### 想理解 Web Search
1. `2026-04-26-openclaw-web-search-provider-design.md`
