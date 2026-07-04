# Plans — 实施计划索引

> 父文档：[`../README.md`](../README.md)
> 用途：分阶段、checkbox 风格的执行清单

## 状态约定

- ✅ active：当前仍在执行 / 仍是有效路径
- 🔄 partial：部分已落地，部分仍 pending
- 📦 done：已落地，保留作历史
- 📜 historical：早期阶段计划，已被新阶段超越
- ❌ superseded：被新计划完全替代

## 索引（按日期倒序）

| 日期 | 文件 | 主题 | 状态 |
|---|---|---|---|
| 2026-05-09 | [`agent-runtime-refactor`](./2026-05-09-agent-runtime-refactor.md) | Agent runtime 重构实施 | ✅ active |
| 2026-05-01 | [`generation-manager-closure-evidence`](./2026-05-01-generation-manager-closure-evidence.md) | Generation Manager closure 证据收集 | ✅ active（从 docs/plans/ 合并）|
| 2026-04-29 | [`weft-tool-plugin-wiring`](./2026-04-29-weft-tool-plugin-wiring.md) | Weft 工具插件接线 | 🔄 partial |
| 2026-04-26 | [`openclaw-web-search-provider`](./2026-04-26-openclaw-web-search-provider.md) | Web 搜索 provider 实施 | 🔄 partial |
| 2026-04-24 | [`ai-local-scout-steam-epic-depth`](./2026-04-24-ai-local-scout-steam-epic-depth.md) | Steam/Epic 深度采集 | 🔄 partial |
| 2026-04-24 | [`ai-local-scout-gog-battlenet-ea-depth`](./2026-04-24-ai-local-scout-gog-battlenet-ea-depth.md) | GOG/Battle.net/EA 深度采集 | 🔄 partial |
| 2026-04-24 | [`ai-local-scout-meaningful-breadth`](./2026-04-24-ai-local-scout-meaningful-breadth.md) | 本地 scout 广度实施 | 🔄 partial |
| 2026-04-18 | [`memory-runtime-wiki-authoring-model`](./2026-04-18-memory-runtime-wiki-authoring-model.md) | Memory Runtime wiki 写作模型 | 🔄 partial |
| 2026-04-16 | [`weft-claw-handoff`](./2026-04-16-weft-claw-handoff.md) | WEFT Claw 交接说明 | 📜 historical |
| 2026-04-15 | [`weft-code-implementation`](./2026-04-15-weft-code-implementation.md) | WEFT Code 实施 | 🔄 partial |
| 2026-04-15 | [`weft-code-p1-tui-shell-scaffold`](./2026-04-15-weft-code-p1-tui-shell-scaffold.md) | WEFT Code P1 TUI shell 骨架 | 🔄 partial |
| 2026-04-15 | [`weft-workspace-session`](./2026-04-15-weft-workspace-session.md) | Workspace/Session 实施 | ✅ active |
| 2026-04-13 | [`vnext-product-package-instance-implementation`](./2026-04-13-vnext-product-package-instance-implementation.md) | vNext 产品-包-实例 模型实施 | 🔄 partial |
| 2026-04-11 | [`memory-runtime-mempalace-bridge`](./2026-04-11-memory-runtime-mempalace-bridge.md) | Memory Runtime ↔ Mempalace 实施 | 🔄 partial |
| 2026-04-11 | [`memory-runtime-preference-card-benchmark-adapter`](./2026-04-11-memory-runtime-preference-card-benchmark-adapter.md) | 偏好卡 benchmark adapter 实施 | 🔄 partial |
| 2026-04-10 | [`weft-claw-package-blueprint-implementation`](./2026-04-10-weft-claw-package-blueprint-implementation.md) | WEFT Claw 包蓝图实施 | 🔄 partial |
| 2026-04-09 | [`vnext-phase-5-weft-claw-productization`](./2026-04-09-vnext-phase-5-weft-claw-productization.md) | vNext Phase 5 WEFT Claw 产品化 | 📜 historical |
| 2026-04-09 | [`vnext-phase-4-runtime-hardening`](./2026-04-09-vnext-phase-4-runtime-hardening.md) | vNext Phase 4 Runtime 硬化 | 📜 historical |
| 2026-04-09 | [`vnext-phase-3-server-source-authority`](./2026-04-09-vnext-phase-3-server-source-authority.md) | vNext Phase 3 Server 来源权威 | 📜 historical |
| 2026-04-07 | [`vnext-phase-4-profiles-policy`](./2026-04-07-vnext-phase-4-profiles-policy.md) | vNext Phase 4 Profiles/Policy | 📜 historical |
| 2026-04-06 | [`vnext-phase-3a-wasm-capability-dispatch`](./2026-04-06-vnext-phase-3a-wasm-capability-dispatch.md) | vNext Phase 3a WASM capability dispatch | 📜 historical |
| 2026-04-06 | [`vnext-phase-2a-2b-app-state-config-lock`](./2026-04-06-vnext-phase-2a-2b-app-state-config-lock.md) | vNext Phase 2a/2b 状态/配置/Lock | 📜 historical（文件自标） |
| 2026-04-06 | [`vnext-phase-1-app-resolver`](./2026-04-06-vnext-phase-1-app-resolver.md) | vNext Phase 1 App Resolver | 📜 historical（文件自标） |
| 2026-04-06 | [`memory-runtime-precise-long-term-memory`](./2026-04-06-memory-runtime-precise-long-term-memory.md) | 精确长期记忆实施 | 🔄 partial |
| 2026-04-03 | [`companion-runtime-extraction`](./2026-04-03-companion-runtime-extraction.md) | Companion Runtime 抽取 | 🔄 partial |

## 关键脉络

### vNext 阶段历史（按 phase 演进）
1. Phase 1：`2026-04-06-vnext-phase-1-app-resolver`
2. Phase 2a/2b：`2026-04-06-vnext-phase-2a-2b-app-state-config-lock`
3. Phase 3a：`2026-04-06-vnext-phase-3a-wasm-capability-dispatch`
4. Phase 3：`2026-04-09-vnext-phase-3-server-source-authority`
5. Phase 4：`2026-04-07-vnext-phase-4-profiles-policy` + `2026-04-09-vnext-phase-4-runtime-hardening`
6. Phase 5：`2026-04-09-vnext-phase-5-weft-claw-productization`
7. 后续：`2026-04-13-vnext-product-package-instance-implementation`

### WEFT Claw 产品化
1. `2026-04-09-vnext-phase-5-weft-claw-productization`（阶段计划）
2. `2026-04-10-weft-claw-package-blueprint-implementation`（蓝图实施）
3. `2026-04-16-weft-claw-handoff`（交接）

### WEFT Code 产品化
1. `2026-04-15-weft-code-implementation`
2. `2026-04-15-weft-code-p1-tui-shell-scaffold`

### Memory Runtime
1. `2026-04-03-companion-runtime-extraction`
2. `2026-04-06-memory-runtime-precise-long-term-memory`
3. `2026-04-11-memory-runtime-mempalace-bridge`
4. `2026-04-11-memory-runtime-preference-card-benchmark-adapter`
5. `2026-04-18-memory-runtime-wiki-authoring-model`

### AI Local Scout 系列
- `2026-04-24-ai-local-scout-meaningful-breadth`
- `2026-04-24-ai-local-scout-steam-epic-depth`
- `2026-04-24-ai-local-scout-gog-battlenet-ea-depth`

### Generation Manager 证据
- `2026-05-01-generation-manager-closure-evidence`（从 docs/plans/ 合并）

## 注

- 状态标记基于命名 + 文件自我描述。**实际状态以仓库代码 git log 为准**
- 如需准确判断某 plan 的当前进度，请去对应代码模块查 commit history
