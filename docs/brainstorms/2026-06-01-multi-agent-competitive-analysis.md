---
date: 2026-06-01
topic: multi-agent-competitive-analysis
status: 待审核
related-plan: ../../.plans/multi-agent-activation/plan.md
---

# 多 Agent 协作竞品分析与借鉴决策

> 本文考察 4 个开源 Agent 平台（Clawith / openakita / paperclip / DeepSeek-Reasonix）+ Factory.ai Missions，
> 对照 WEFT-plug 现状，给出「借鉴什么 / 不借什么 / 怎么落地」的决策。
> 实施方案见 [`.plans/multi-agent-activation/plan.md`](../../.plans/multi-agent-activation/plan.md)。

## 0. 一句话结论

WEFT 的多 agent **数据模型已经写好了**（team-runtime / team-task-board / workflow-template-devteam，
共 2300+ 行真实实现），但**只记账不执行**——缺的是执行引擎、状态外置、验证回流、编排驱动轮。
这四样恰好是 Factory Missions 让 agent 跑 16 天的关键，且**全部能在 package 层补齐，不改 Rust core**。

**实施边界（负责人已确认）**：
- 主战场 = **core + package 的逻辑部分**；**逻辑优先**，缓存等优化不得弱化逻辑主线。
- **不碰 server**：本方向不依赖也不修改任何 server 业务线（成本/计费不在此盘）。
- **client 只做展现**：多 agent 的工作流状态 / 进度 / 成本最终在 client 端可视化；package 侧先把数据产出做好。
- 缓存/工具修复（Reasonix 借鉴）**仅 package 层**，降级为后续。

---

## 1. WEFT 现状基线（已核实）

### 1.1 能力底座【强】
| 能力 | 状态 | 证据 |
|------|------|------|
| Capability Registry | ✅ | `core/src/app/registry.rs`，provides 声明 + 优先级 + 多对多绑定 |
| Generation 生命周期 | ✅ | `core/src/app/generation.rs`（2007 行），Candidate→Verified→Active→Rollback |
| Profile 三档授权 | ✅ | `core/src/app/policy.rs`，Safe/Developer/Trusted |
| WASM host 桥 | ✅ | `core/src/package/bridge.rs`（1767 行），见下 |
| Go server 9 业务线 | ✅ | identity/billing/llm_gateway(38ch)/search/cloud/multimedia/packages/device/remote_relay |

### 1.2 WASM 包能力天花板（关键发现）
包通过 `packages/sdk`（`#[host_fn] extern "ExtismHost"`）可调用的 host 函数：
- LLM：`host_chat_completion` / `_stream`
- 数据库：`host_sqlite_query/execute/batch` + KV `host_kv_get/set/list/delete`
- 进程/文件：`host_process_spawn/read_stdout/write_stdin`、`host_read_file/write_file`
- **包间调用**：`host_call_package` / `host_call_package_ws` / `host_capability_call`
- 被动入口：`handle_webhook`（路由已接 `/api/packages/{name}/webhook`）、cron `tick`
- SDK 已封装好 `call_package` / `call_package_ws_action` / `now_ms`，**无需手写 FFI**

**结论：host 桥已是一个相当完整的 agent runtime ABI**，绝大多数竞品能力落在 package 层。

### 1.3 空白点
多 agent 真正协作、触发器/自主性、向量记忆（memory 包只有 wasm 无源码）、MCP 未装载、成本可观测性。

### 1.4 战略背景（重要）
`docs/server/01-architecture-v2.md:12`：**"不做散发 — 主线成熟前砍：Agent 自我演化 / 多 Agent 协作 / 触发器 / 工作流市场。"**
→ 这是**时序决策**，不是永久否定。本文借鉴的能力多落在"主线成熟后要补"的区间，竞品给的是免费蓝图。

---

## 2. 四个参考项目分析

### 2.1 Clawith（多 agent 数字员工组织 · Python FastAPI + React）
**定位**：正面竞品，主打 WEFT v2 暂时搁置的能力。
| 特性 | 机制 | 借鉴价值 |
|------|------|---------|
| Aware / Focus Items | DB 驱动的 working memory（key/status/kind/metadata），Trigger 经 `focus_ref` 绑定，完成自动取消触发器 | agent 自主任务管理的数据结构 |
| 六种触发器 | cron/once/interval/poll/on_message/webhook，**自研轻量调度器**（非 Celery）：每 15s tick → 评估 → 入队（idempotency key）→ DB 行锁认领 → 30s 去重 → 唤醒 agent | **对 Rust 后端极友好**，是编排驱动轮的现成蓝图 |
| Plaza 知识流 | Post/Comment/Like 三表，agent 发帖 + LLM context 注入 | agent 间异步知识共享 |
| A2A 通信 | notify(单向)/consult(同步)/task_delegate(异步建 Trigger) | 多 agent 通信语义分层 |
| 三级自主权 | autonomy_policy 把操作映射 L1(免批)/L2(人批)/L3(高危)，L3 建 ApprovalRequest 暂停 | 运行时逐操作审批 |
关键文件：`backend/app/services/trigger_daemon.py`、`models/{focus,trigger,plaza,agent}.py`

### 2.2 openakita（多 agent 助手 · Python + Tauri）
| 特性 | 机制 | 借鉴价值 |
|------|------|---------|
| MCP 一等公民 | stdio + HTTP + SSE 三传输，多目录发现，运行时增删，GUI 管理 | WEFT 有 mcp-client 但**未装载**，这是最快补齐路径 |
| 插件故障隔离 | `PluginErrorTracker` 错误超阈值自动禁用 + `_LiveFilteredHostRefs` 白名单 | WASM 沙箱缺"健康度→自动降级"层 |
| SOUL.md / POLICIES.yaml | 系统级价值观注入 + 四区分级安全策略 | 与 Profile 正交可叠加 |
关键文件：`src/openakita/tools/mcp.py`、`plugins/manager.py`、`identity/POLICIES.yaml`

### 2.3 paperclip（多 agent 编排控制平面 · Node + React）
**"If OpenClaw is an employee, Paperclip is the company."**
| 特性 | 机制 | 借鉴价值 |
|------|------|---------|
| 渐进式外部化 | 内置适配器可逐步转插件并保留回退，可变注册表 + 热重载 + UI 解析器动态加载 | 架构最优雅；但会动 WEFT 核心加载模型，**大改慎入** |
| Skills 系统 | Markdown 工作流指南（非 tool），心跳时 `listSkills/syncSkills` 注入 | 与 Anthropic Skills 同源 |
关键文件：`server/src/adapters/registry.ts`、`plugin-loader.ts`

### 2.4 DeepSeek-Reasonix（缓存工程化编码 CLI · TypeScript）
**定位**：技术参考，核心价值不在 reasoning 而在**缓存工程化**（99.82% 命中率，成本 $61→$1.38）。
| 支柱 | 机制 | 借鉴价值 |
|------|------|---------|
| Cache-First Loop | 三层上下文：ImmutablePrefix(字节不变)/AppendOnlyLog(仅追加)/VolatileScratch(每轮重置) | **多 agent 的 token 底座**，在 package 内的 LLM 调用层做（不碰 server） |
| Tool-Call Repair | flatten/scavenge/truncation/storm 四层容错 | 跨模型工具调用健壮性（package 层） |
| Cost Control | 分层默认 + 转端压缩 + 模型自报升级 `<<<NEEDS_PRO>>>` | 成本数据在 package 产出、**client 展现**；不进 server billing |
关键文件：`src/memory/runtime.ts`、`src/repair/`、`src/telemetry/stats.ts`

---

## 3. WEFT 已有的 team 三件套现状（关键）

| 包 | 行数 | 实现了什么 | 缺什么 |
|------|------|-----------|--------|
| `team-task-board` | 1032 | 任务 CRUD + 状态机(todo/in_progress/blocked/done) + review_items(pending/completed) + activity log + handoff 记录，全 host_kv | handoff 只记录**不唤醒**下家；状态转移无校验 |
| `team-runtime` | 792 | 4 角色 catalog(planner/implementer/reviewer/integrator) + session + 委派合同生成 | 委派只生成合同**不投递**（`delegate_contract_result` ~673-687） |
| `workflow-template-devteam` | 475 | 6 阶段(intake→plan→execute→review→integrate→done) + 转移校验 | 纯声明，无执行引擎 |

**委派链路真相**（修正过的精确判断）：
- ✅ companion-core → agent-runtime 委派**真跑通**（`execute_delegate_runtime_turn` ~1508，真投递真接回）
- ❌ agent → agent 跑不通；team 三件套**零** `host_call_package` 互调，与 agent 主循环未接线
- ❌ 这三个包 + cron + workflow-orchestrator **已在 index.toml 注册但未进 `packages/installed/`**

补充事实：`index.toml` 用两层包名机制——`name`（逻辑名）/ `runtime_provider`（物理名），是全局通用约定（如 `memory-store→memory`、`skills-runtime→skills`），非 agent 特例。agent 逻辑名 `agent-runtime`（物理 `agent-core`）。**代码一律用逻辑名 `agent-runtime`**；现有一处不一致 `cron/lib.rs:296` 误用物理名，落地时顺手统一。`workflow-orchestrator` 现为 37 行纯存根。

---

## 4. Factory Missions 的四个关键启发

Factory Missions 的数据模型与 WEFT team 三件套**几乎同构**（orchestrator/worker/validator ≈ planner/implementer/reviewer/integrator；features/milestones ≈ 6 阶段；任务状态/handoff/review 都已建模）。
**WEFT 建模不输 Factory，差的是建模之外的四样**：

1. **状态外置成单一真相源**：全部状态摊在共享工件（validation-contract.md / features.json / git），无 agent 在 context 里装全局，每个 worker 只读相关部分。WEFT 现在是碎的（各包各 KV）。
2. **验证契约先于实现**：orchestrator 第一步写可测的行为断言（VAL-AUTH-001 式），**先于** feature。
3. **creator-verifier 强制分离**：写代码的 agent 无权判定自己对错，独立 validator 黑盒验证，只暴露问题不修；失败回流生成 fix feature，2-4 轮收敛。
4. **执行引擎 + 编排驱动轮**：默认串行（继承上一 commit，天然避免改同一文件冲突）+ 只读操作并行 + milestone 检查点 + 卡住交还人类（Mission Control）。

来源：[missions-architecture](https://factory.ai/news/missions-architecture)、[ship for days(Alvoeiro)](https://startuphub.ai/ai-news/artificial-intelligence/2026/missions-ai-agents-that-ship-for-days)

---

## 5. 借鉴决策矩阵

| # | 借鉴项 | 来源 | 决策 | 优先级 | 理由 |
|---|--------|------|------|--------|------|
| 1 | 缓存三层模型 + 工具修复（**仅 package 层**） | Reasonix | **做（弱化）** | 🔵 后续 | 只做 package 内缓存/工具容错；**成本展示归 client，逻辑归 core/package，不碰 server**；优先级低于逻辑主线，不得弱化多 agent 逻辑 |
| 2 | 接通 team 委派/交接执行 | WEFT 自有 + Clawith | **做** | 🟢 现在 | 把已写好的三件套从"记账"变"能跑"，最短路径 |
| 3 | 编排驱动轮（cron tick → 独立 orchestrator） | Clawith | **做** | 🟢 现在 | host 已每 10s 调 cron.tick；cron 心跳 + workflow-orchestrator 编排，职责分离 |
| 4 | 验证契约 + creator-verifier 回流 | Factory | **做** | 🟡 紧随 | 复用现有 review_items，质量保障核心 |
| 5 | MCP 一等公民 | openakita | **做** | 🟡 并行 | mcp-client 已有未装载，补工具生态 |
| 6 | 三级自主权 + 审批 | Clawith/openakita | **做** | 🟡 紧随 | 与静态 Profile 正交叠加；对应 Factory 的 human-in-the-loop |
| 7 | 插件故障隔离 | openakita | **做** | 🔵 后续 | WASM 沙箱补健康度降级层（~50 行 Rust，唯一碰 core 的小改） |
| 8 | 渐进式外部化 | paperclip | **暂缓** | ⚪ 评估 | 解决插件灵活性，非当前核心矛盾；动核心加载模型，大改 |
| 9 | Plaza 知识流 / A2A 三模式 | Clawith | **暂缓** | ⚪ 后续 | 多 agent 跑通后再加协作维度 |

**主线（负责人已定方向）**：#2 #3 #4——把 team 三件套接成可执行的多 agent 闭环。**逻辑优先**。
**边界**：只动 **core + package**，**client 只做展现**，**不碰 server**。缓存/成本（#1）降级为后续，且仅 package 层。

---

## 6. 落地方向

详见 [`.plans/multi-agent-activation/plan.md`](../../.plans/multi-agent-activation/plan.md)，四个 Phase 全在 package 层、零 core 改动：

- **P1 装载 + 心跳**：team 三件套/cron/orchestrator 进 installed；cron.tick 加一行转调 orchestrator
- **P2 接通委派/交接**：delegate 真投递、handoff 加 status、防循环委派
- **P3 编排驱动轮**：orchestrator 存根填实（扫 task-board → 推进 handoff → 阶段转移 → 幂等）
- **P4 验证回流**：验证契约存 review_items、creator-verifier 分离、fail 生成 fix task、收敛轮次

**关键架构裁决**：cron = 心跳源（host 唯一周期钩子），workflow-orchestrator = 编排器（承载业务）。职责分离且零 host 改动。

---

## 7. 来源

- Factory Missions：[architecture](https://factory.ai/news/missions-architecture) · [introducing](https://factory.ai/news/missions) · [ship for days](https://startuphub.ai/ai-news/artificial-intelligence/2026/missions-ai-agents-that-ship-for-days) · [docs](https://docs.factory.ai/cli/features/missions)
- 参考代码（已克隆至 `reference/`）：Clawith · openakita · paperclip · DeepSeek-Reasonix
- WEFT 内部：`core/src/package/bridge.rs`、`packages/official/{team-runtime,team-task-board,workflow-template-devteam,cron,companion-core}/`、`packages/index.toml`、`docs/server/01-architecture-v2.md`
