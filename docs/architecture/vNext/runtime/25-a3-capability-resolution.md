# A3：编排去硬编码包名（按 capability 解析，已落地）

> 状态：去硬编码包名 已落地 + 编译 + 运行时验证通过。事件化（去轮询）经评估**不做**（见 §1）。
> 关联：`core/src/main.rs`（三个后台 tick 循环）、`app/registry.rs`（CapabilityRegistry）。

## 1. 范围决策：只去硬编码，不做事件化

A3 原描述含两半，价值/风险差异大：
- **去轮询 → 事件驱动**：cron 本质就是定时检查到期任务，轮询是合理实现，去不掉；orchestrator 的 3s dispatch 理论可事件化，但需要「包 → core 反向事件通道」，而当前 host_fn 是**同步单向**调用模型，没有反向通道。这是 L 级架构改动 + 高风险，**本轮不做**。
- **去硬编码包名**：main.rs 的后台循环硬编码了 `"cron"`/`"workflow-orchestrator"`/`"memory"`。改为按 capability 经 `CapabilityRegistry` 解析 provider。**低风险、可落地** — 本轮交付。

## 2. 实现

新增 `resolve_capability_provider(registry, capability) -> Option<String>`（main.rs）：从 `CapabilityRegistry` 取某 capability 的最高优先级 provider 包名。

三个后台 tick 循环改造（均先在短锁内 clone `state.capability_registry`，读出 provider 名后释放锁，再触碰 wasm handle）：
| 循环 | 原硬编码 | 改为按 capability 解析 |
|---|---|---|
| cron tick (10s) | `wh.call("cron", "tick")` | `scheduler.cron` → provider |
| memory cleanup (~5min) | `wh.call("memory", "cleanup_expired")` | `memory.store` → provider |
| orchestrator (3s) | `wh.call("workflow-orchestrator", "dispatch")` | `workflow.orchestration` → provider |

KV save 循环不涉及包名，未改。`grep 'has_package("' / '.call("'` 在 main.rs 现为**零字面量包名**。

## 3. 验证

- `cargo build` exit 0（无 error / unused 警告）。
- **运行时**：起 core，查 `/api/capabilities` 确认解析结果与已加载包匹配：
  - `scheduler.cron` → `cron` ✅（在已加载 wasm 列表）
  - `workflow.orchestration` → `workflow-orchestrator` ✅
  - `memory.store` → `memory-store`（见 §4）
- cron tick / orchestrator dispatch 运行无 error、无 "Package not loaded" —— provider 名解析正确、包已加载、tick 正常触发。

## 4. 顺带发现：memory cleanup 的 wasm 调用在 B2 后已失效（既有问题，非 A3 引入）

`memory.store` 现解析为 `memory-store`，但 B2 后它是 **service 包（Python）**，不在 wasm 加载列表 → `wh.has_package("memory-store")` = false → 内存清理静默跳过。

但**原硬编码 `"memory"` 同样不在 wasm 加载列表**（运行集只有 service 版 memory-store，无名为 `memory` 的 wasm 包；installed/memory 那个 wasm 已在 B2 归档）→ 原代码的 `wh.has_package("memory")` 也是 false。

**结论**：memory cleanup 的 wasm 调用路径在 memory 转 service 后就已失效，**与 A3 无关，行为无变化**。A3 的版本语义更诚实（指向真实 provider）。真正修复需让 cleanup 走 service 包的 HTTP 调用而非 wasm handle —— 属独立问题，记录待处理。

## 5. 验证边界 / 后续
- 事件化（去轮询）未做（§1，需反向事件通道，L 级）。
- memory cleanup 经 service HTTP 调用的修复未做（§4，独立问题）。
- 轮询间隔仍硬编码（10s/3s/300s）；可提取为 config，本轮未做。
