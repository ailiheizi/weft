# 设计文档 vs 实际实现 — 差距分析

> 分析时间: 2026-05-20
> 方法: 4 个 worker 并行阅读全部设计文档 + 实际代码

---

## 总体完成度

| 领域 | 设计文档数 | 完成度 |
|------|-----------|--------|
| Core Runtime (kernel/config/binding/generation) | 10 | ~79% |
| weft-claw 产品 | 6 | ~66% 忠实度, 81% 包存在 |
| weft-code 产品 | 4 | ~50% (TUI 较好，runtime/service 缺失多) |
| 包服务/注册/发布 | 7 | ~70% (server 完整，core 消费端缺) |

---

## 一、weft-claw 产品差距

### feature 壳 vs 实际实现

5 个 feature 包全是 `runtime="embedded"` + `entry="README.md"`，无代码：

| Feature | 提供的能力 | 实际由谁实现 |
|---------|-----------|-------------|
| weft-claw-chat-feature | agent.runtime, prompt.system, memory.store | agent-core(WASM) + prompt-system(WASM) + memory(WASM) |
| weft-claw-extensions-feature | ext.skills, ext.mcp | skills-runtime(WASM) + mcp-client(WASM) |
| weft-claw-channel-feature | channel.bridge | channel-core(WASM) |
| weft-claw-devteam-feature | team.*, workflow.*, 等 13 项 | team-runtime + team-task-board + workflow-template-devteam (WASM) |
| weft-claw-automation-feature | workflow.orchestration, tool.* | workflow-orchestrator + tool-* (WASM, optional) |

### 能力越界

| 设计规定 | 实际 |
|---------|------|
| `tool.runtime` 只由 tool-runtime-core 提供 | skills-runtime 也注册了 `tool.web` |
| `ext.skills` 只由 skills-runtime 提供 | tool-runtime-core 也提供了 tool 相关能力 |
| `memory.store` 只由 memory-store 提供 | memory-runtime 也提供（端口冲突 43129） |

### 缺失的插件

| 设计有 | 实际 |
|--------|------|
| 独立 channel-core 目录 | installed 里是 `channels`（老名字） |
| weft-claw-ui 独立实现 | installed/weft-claw 只是 `plugin.toml` + `ui/index.html` |

---

## 二、Core Runtime 差距

### 完成度高的模块 (≥83%)

- **Core Kernel** (01-core-kernel.md): 100% — HTTP server, WASM host, process manager, pipeline 全部实现
- **Declaration/Config/Lock** (04): 100% — package.toml, config.toml, lock.toml 三层模型完整
- **AI Change Workflow** (08): 83% — generation propose/verify/activate/rollback 完整

### 完成度中等的模块 (~57%)

- **Binding & Resolution** (05): 57%
  - ✅ slot_bindings 机制存在
  - ✅ product_graph 合并逻辑
  - ❌ 动态重绑定未实现
  - ❌ 设计中的 resolution 优先级链未完全落地（前端做了查表，core 侧无感知）

- **Execution & Files** (07): 57%
  - ✅ 内置工具 fs_read/fs_write/fs_list/shell_exec
  - ❌ 沙盒执行模型未实现
  - ❌ 文件系统 root 白名单未实现
  - ❌ 设计中的执行策略链（approve→sandbox→native）未实现

### 关键缺失

| 设计机制 | 状态 |
|---------|------|
| Product Package Instance Model (13) | ⚠️ `.weft/<app>/` 目录结构存在，但 instance config 字段不全 |
| Generation Manager (19) | ✅ propose/verify/activate/rollback/diff 全部实现 |
| Package Server Integration (20) | ❌ Core 的 PackageServiceClient 只有 fetch_package_index()，未消费 server 的 bind/install/update API |

---

## 三、weft-code 产品差距

### TUI (较好)

| 特性 | 状态 |
|------|------|
| 5-panel ratatui 布局 | ✅ |
| Route 导航 (Session/Teams/Tasks/Approvals/Timeline) | ✅ |
| Focus 切换 / 搜索 / 命令面板 | ✅ |
| Live SSE event stream | ❌ 只有静态快照 |
| 平滑滚动 / 会话切换 / 斜杠命令 / 模态叠加 | ❌ 未实现 |

### Runtime (差距大)

| 设计 | 实际 |
|------|------|
| 独立 HTTP runtime server (server/src/weft_code/) | WASM 单文件 (packages/installed/weft-code-runtime/) |
| 分离的 SessionService/TaskService/PolicyService | 全在一个 lib.rs |
| Project 资源 + projects API | 不存在 |
| SSE 事件流 | 只读快照 list_events |
| 6 种 session 状态 | 只有 3 种 (active, waiting_approval, blocked) |
| 6 种 task 类型 | 只有 coding_task |
| 7 种 task 状态 | 只有 4 种 |
| Session lineage (parent_session_id) | 未实现 |

### CLI (缺失)

| 设计 | 实际 |
|------|------|
| weft-code CLI (sessions, teams, tasks, mode, status) | 不存在。core/src/bin/weft.rs 是老 gen/scene CLI |
| CLI 通过 HTTP API 交互 | 无对应二进制 |

---

## 四、包服务/注册差距

### Server 侧 (较好)

| 模块 | 行数 | 状态 |
|------|------|------|
| draft_store.rs | 1800+ | ✅ |
| bindings.rs | 1500+ | ✅ |
| plans.rs | 1300+ | ✅ |
| source_authority (Phase 3) | — | ✅ /api/sources/* 全部实现 |
| review/publication | — | ✅ candidate→review→publish→yank 完整 |

### Core 消费侧 (缺失)

| 设计 | 实际 |
|------|------|
| PackageServiceClient 完整 API | 只有 fetch_package_index() |
| Core 远程解析 bind/install/update | Core 完全本地解析 |
| Review/publication workflow 集成 | Core 从不触发 |
| AppSourceIndex | 未实现 |
| 真实 artifact 存储 (Phase 6) | 未实现 |
| GitHub 接入 (Phase 10) | 未实现 |

---

## 五、installed vs official 分裂

这是最关键的架构差距：

```
packages/official/ — 29 个插件（设计 + 源码）
packages/installed/ — 10 个插件（实际运行时加载）

只在 official 有但 installed 没有的:
  mcp-client, prompt-system, workflow-orchestrator
  team-runtime, team-task-board, workflow-template-devteam
  tool-runtime-core, tool-shell, tool-files, tool-web, tool-git
  所有 feature 壳

installed 独有但 official 没有的:
  native-echo (测试用)
  weft-code-runtime
```

**稳定运行的 weft-claw 实际只加载了 10 个插件**：companion-core, agent-core, skills, memory, channels, cron, memory-runtime, weft-claw(UI), weft-code-runtime, native-echo。

---

## 六、优先级建议

### P0 — 阻塞性问题
1. **installed 缺失 mcp-client** — 无法调用外部 MCP 工具
2. **installed 缺失 prompt-system** — agent prompt 渲染可能退化
3. **installed 和 official 双轨** — 到底哪个是真实来源？

### P1 — 功能缺口
4. weft-code CLI 不存在
5. weft-code session/task 状态机不完整
6. weft-code SSE 事件流缺失
7. Core PackageServiceClient 只读不写

### P2 — 设计偏离
8. feature 壳全是空 README
9. 能力越界 (tool-runtime-core ↔ skills-runtime)
10. execution sandbox 未实现
11. 动态重绑定未实现

---

## 七、Reference 代码对比 — 竞品已实现但 weft-claw 缺失的功能

> 分析时间: 2026-05-21
> 来源: reference/zeroclaw, reference/codex-main, reference/opencode-dev 实际源码

### 7.1 Hook 系统（zeroclaw/src/hooks/）

zeroclaw 实现了完整的 hook 拦截框架：

| Hook 类型 | 触发点 | weft-claw 状态 |
|-----------|--------|----------------|
| before_llm_call | LLM 调用前，可修改 prompt | ❌ 无 |
| before_tool_call | 工具执行前，可 Cancel | ❌ 无 |
| after_tool_call | 工具执行后，fire-and-forget | ❌ 无 |
| before_prompt_build | system prompt 组装前 | ❌ 无 |
| llm_input/output | 模型 I/O 审计 | ❌ 无 |
| session_start/end | 会话生命周期 | ❌ 无 |
| on_message_received | 入站消息拦截 | ❌ 无 |

**特性**：modifying hooks 按优先级串行执行，返回 `Continue(data)` 或 `Cancel(reason)`；void hooks 并行 fire-and-forget。

**建议**：在 agent-core 的 `do_send_session_message` 前后插入 hook dispatch 点，capability 名 `hook.runtime`。

---

### 7.2 Approval 工作流（zeroclaw/src/approval/）

zeroclaw 实现了三级审批模式：

| 模式 | 行为 |
|------|------|
| Full | 所有工具调用自动批准 |
| Supervised | 危险工具需用户确认 |
| ReadOnly | 只允许读操作 |

**特性**：
- Session 级 allowlist（"Always" 决策在会话内持久）
- `auto_approve` / `always_ask` 配置级策略
- `ApprovalLogEntry` 审计日志（时间戳、工具名、参数摘要、渠道）
- 非交互模式（channel 驱动时自动拒绝需审批工具）

**weft-claw 状态**：❌ 无任何 approval 机制，所有工具调用直接执行。

**建议**：在 `tool-runtime-core` 的工具执行入口加 approval gate，通过 `session.events` 发出 `approval_request` 事件，前端响应后继续。

---

### 7.3 多 Agent 委托（zeroclaw/src/tools/ DelegateTool）

zeroclaw 的 `delegate` 工具支持：
- **sync**：等待子 agent 完成后返回结果
- **background**：后台运行，结果写入 workspace 文件
- **parallel**：并行启动多个子 agent

**weft-claw 状态**：❌ 无 delegate/sub-agent 机制，agent-core 只支持单 agent 单轮。

---

### 7.4 SOP 确定性执行（zeroclaw/src/sop/）

zeroclaw 的 SOP 引擎支持无 LLM 的确定性步骤管道：

| 步骤类型 | 行为 |
|---------|------|
| LLM | 调用模型 |
| Tool | 直接执行工具，跳过 LLM |
| Checkpoint | 暂停等待人工审批 |

**触发方式**：MQTT、Webhook、Cron。

**weft-claw 状态**：❌ 无 SOP/workflow 执行引擎（`workflow-orchestrator` 包存在但为空壳）。

---

### 7.5 Loop 检测与 History 剪枝（zeroclaw/src/agent/）

zeroclaw agent 内置：
- **Loop detector**：检测连续重复工具调用，自动中止
- **History pruner**：按 token budget 裁剪历史，防止 context 溢出
- **Thinking module**：结构化推理 + 成本追踪

**weft-claw 状态**：
- Loop 检测：❌ 无
- History 剪枝：⚠️ agent-core 有基础 history 管理，但无 token budget 控制
- Thinking：❌ 无

---

### 7.6 成本追踪（zeroclaw/src/cost/）

zeroclaw 对每次工具调用和 LLM 调用追踪：token 用量、定价、累计成本。

**weft-claw 状态**：❌ agent-core 返回 `model`/`provider` 字段，但无成本计算。

---

### 7.7 Sandbox 执行（codex-main）

codex 实现了 seatbelt sandbox：
- `CODEX_SANDBOX=seatbelt` 环境变量启用
- 网络隔离：`CODEX_SANDBOX_NETWORK_DISABLED=1`
- 进程级隔离

**weft-claw 状态**：❌ `tool-runtime-core` 直接调用 `host_exec_value`，无沙盒层。

---

### 7.8 Verifiable Intent（zeroclaw/src/verifiable_intent/）

zeroclaw 实现了基于 SD-JWT 的三层凭证链，用于支付/结账场景的约束验证。这是 zeroclaw 特有的硬件/IoT/支付场景能力，weft-claw 当前定位不需要此功能。

---

### 7.9 工具覆盖对比

| 工具类别 | zeroclaw | weft-claw |
|---------|---------|-----------|
| 文件操作 | file_read/write/edit | fs_read/write/list ✅ |
| Shell | shell | shell_exec ✅ |
| Web | web_search/fetch/browser | web_search/fetch ✅ (browser ❌) |
| Git | git_operations | git ✅ |
| MCP | mcp_client/deferred/transport | mcp-client ⚠️ (installed 缺失) |
| 多 agent | delegate (sync/bg/parallel) | ❌ |
| 用户交互 | ask_user, screenshot | ❌ |
| 文档 | pdf_read | ❌ |
| 图像 | image_gen, image_info | ❌ |
| 计算 | calculator | ❌ |
| 外部集成 | jira, notion, composio, linkedin | ❌ |
| 记忆工具 | memory_store/recall/forget | memory-store ⚠️ (间接) |
| Cron 工具 | cron_add/update/remove/list | cron 包 ⚠️ (非工具形式) |

---

### 7.10 优先级补充（基于 reference 对比）

| 优先级 | 功能 | 来源 |
|--------|------|------|
| P1 | Approval gate（Supervised 模式） | zeroclaw |
| P1 | Loop 检测 | zeroclaw |
| P1 | `ask_user` 工具（agent 主动请求用户输入） | zeroclaw |
| P2 | Hook 系统（before_tool_call / after_tool_call） | zeroclaw |
| P2 | Token budget history 剪枝 | zeroclaw |
| P2 | 成本追踪 | zeroclaw |
| P2 | Multi-agent delegate 工具 | zeroclaw |
| P3 | SOP 确定性执行引擎 | zeroclaw |
| P3 | Sandbox 执行层 | codex |
| P3 | pdf_read、screenshot、image_gen 工具 | zeroclaw |
