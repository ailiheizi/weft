# 07 — 包清单与运行时装载现状

> 作用：把 `packages/official/` 的设计包清单、`packages/installed/` 的实际装载结果，以及 Core Runtime / Registry Server / Frontend 的关联关系统一对齐，方便排查“设计有、运行时没有”的差异。

---

## 一、总体结论

WEFT-plug 当前存在明显的双轨现象：

- **official/** 保存了较完整的包设计与实现源码
- **installed/** 仅装载了实际运行时需要的一小部分包

这意味着：

1. 文档上的“完整能力面”不等于运行时真的可用
2. 运行时行为必须以 `packages/installed/` 为准
3. 只看 official 会高估当前系统的可执行能力

---

## 二、packages/official/ 完整包清单

> 下表按包名列出官方包、能力、主要 Actions、依赖与状态。

| 包名 | Capability | 主要 Actions | 调用依赖 | 状态 |
|------|------------|-------------|---------|------|
| agent-core | `agent.runtime` | `create_session`, `send_session_message`, `get_session_messages`, `list_sessions`, `delete_session`, `get_session_context`, `get_session_skills`, `set_session_skills`, `update_session_title`, `route_delegate` | `memory-store`, `skills-runtime`, `mcp-client` | 完整 |
| skills | `skills-runtime` | `retrieve_applicable`, `crystallize_from_trajectory`, `record_skill_usage`, `execute_tool`, `get_tool_specs`, `list_evolved`, `review_evolved` | `tool-runtime-core`, `mcp-client`, `tool-web` | 完整 |
| tool-runtime-core | `tool.runtime` | `execute_tool`（`fs_read/write/list`, `shell_exec`, `git`, `web_fetch`） | `host_exec_value`（宿主） | 完整 |
| tool-web | `tool.web` | `web_fetch`, `web_search` | `js-extension-runtime` | 完整 |
| weft-claw | `weft_claw.turn` | `send_message`, `list_sessions`, `get_session_messages`, `list_events`, `get_session_events`, `delete_session_events`, `reset_session`, `cancel_turn`, `debug_*` | `agent.runtime`, `session.events` | 完整 |
| mcp-client | `mcp.client` | `get_tools`, `call_tool`, `register_server`, `list_servers` | 外部 MCP servers | 部分 |
| channel-core | `channels` | `send`, `get_inbox`, `register_channel`, `list_channels` | — | 部分 |
| memory-store | `memory.store` | `store`, `recall`, `forget`, `list`, `cleanup` | SQLite（宿主） | 部分 |
| team-runtime | `team.runtime`, `team.role.catalog`, `team.context.shared` | `create_team_session`, `route_delegate`, `get_shared_context` | `agent.runtime` | 部分 |
| cron | `cron.scheduler` | — | — | 存根 |
| prompt-system | `prompt.system` | `render` | — | 存根 |
| workflow-orchestrator | `workflow.orchestration` | `plan` | — | 存根 |
| tool-git | `tool.git` | `run` | host bridge | 存根 |
| tool-shell | `tool.shell` | `run` | host bridge | 存根 |
| tool-files | `tool.files` | `list`, `read`, `write` | host bridge | 存根 |

### 说明

- `完整`：实现与设计基本对齐，可直接提供能力
- `部分`：有实现，但接口、依赖或能力覆盖仍不完整
- `存根`：仅保留壳或占位，尚未形成真实运行时能力

---

## 三、packages/installed/ 实际运行时加载包

当前运行时实际加载的是：

- `companion-core`
- `agent-core`
- `skills`
- `memory`
- `channels`
- `cron`
- `memory-runtime`
- `weft-claw(UI)`
- `weft-code-runtime`
- `native-echo`

### 明确的缺失项

以下包在 `packages/official/` 中存在实现，但 **未被 `packages/installed/` 加载**：

- `mcp-client`
- `prompt-system`
- `workflow-orchestrator`
- `tool-runtime-core`
- `tool-web`
- `tool-git`
- `tool-shell`
- `tool-files`

---

## 四、official / installed 分裂问题

这是当前最重要的包层问题。

### 4.1 现状

- **official** 更像“能力目录 + 源码库”
- **installed** 更像“实际运行时清单”

### 4.2 风险

1. **能力幻觉**：文档/源码存在，不代表运行时可调用
2. **依赖断链**：`agent-core`、`skills` 依赖的关键包未加载时，调用链会退化
3. **调试困难**：同名能力在不同目录状态不一致，排障成本高
4. **版本漂移**：official 更新了，installed 未同步，导致行为与预期不一致

### 4.3 当前缺口

- 外部 MCP 调用链缺少 `mcp-client`
- prompt 渲染链缺少 `prompt-system`
- 通用工具能力链缺少 `tool-runtime-core` / `tool-web`
- shell / git / files 相关能力仍停留在 official 侧
- 运行时加载清单没有覆盖设计上的完整工具栈

---

## 五、Core Runtime 关键模块

### `main.rs`

职责：

- 启动编排
- 发现产品根目录
- 构建 capability registry
- 启动 HTTP server

### `api/generations.rs`

职责：

- Generation 生命周期管理
- 提供 `propose / verify / activate / rollback`

### `api/capabilities.rs`

职责：

- Capability registry 查询
- 供外部查看当前可用能力

### `api/plans.rs`

职责：

- Activation plan 生成
- 支持能力装载与切换的计划化输出

### `app/`

职责：

- App config 解析
- package binding
- policy 执行

### `vkeys.rs`

职责：

- Virtual key store
- capability 访问控制

---

## 六、Registry Server（server/）

Registry Server 使用 **Go + Gin + PostgreSQL**，默认端口 **3007**。

### 6.1 认证

- `register`
- `login`
- `refresh`
- 认证方式：JWT

### 6.2 用户

- `/api/me`
- `change-password`

### 6.3 Credits

- `balance`
- `transactions`
- `checkin`
- 规则：**10 积分/天，连续 7 天有奖励**

### 6.4 Admin

- 用户管理
- `credits grant/deduct`

### 6.5 内部服务

- `/internal/credits/consume`
- `/internal/users/:id/verify`

---

## 七、Frontend（clients/frontend/）

前端是 **Electron + Vue 3 + TypeScript**。

### 7.1 主要组件

- `WeftCoreDashboard`
- `ChatPanel`
- `SettingsPanel`
- `SkillsPanel`
- `CronPanel`
- `AgentActivityTimeline`

### 7.2 主进程能力

- `weft-core-api-client`
- `weft-core-sse-client`
- `companion-runtime-controller`

### 7.3 Pinia stores

- `useChatStore`
- `useWeftCoreStore`

---

## 八、与运行时能力相关的判断

从包清单看，当前系统的真实可用能力主要集中在：

- agent 会话管理
- skills 调度
- memory 存取
- channels 通道
- weft-claw 基础 UI / turn 入口
- weft-code runtime 的最小实现

但以下能力仍明显不足：

- 外部 MCP 工具联通
- prompt 渲染链
- workflow 编排
- shell / git / files 完整工具链
- team runtime 完整协作面

---

## 九、结论

当前包体系不是“设计即运行”，而是“设计与装载分离”。

因此在排查功能缺失时，应优先确认：

1. 该能力是否只存在于 `packages/official/`
2. 它是否真的进入 `packages/installed/`
3. 依赖包是否同时满足
4. Core Runtime 是否把该包纳入 capability registry

如果这四步里任一步断开，功能就会在运行时表现为“看得到但用不了”。
