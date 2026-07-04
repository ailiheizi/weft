# WEFT-Code 设计稿：统一 Runtime 驱动的 CLI/TUI Code Product 蓝图

## 1. 背景与目标

当前 `WEFT-plug` 已经围绕 WEFT vNext 逐步建立了 product package、workflow、team runtime、task board、AI change workflow 等一套偏平台化的能力底座，并且 `weft-claw` 已经承担了首个产品装配样板的角色。

但如果目标是做一个真正可直接用于日常开发的 code product，仅有 `weft-claw` 这类偏平台语义的产品装配体还不够。当前缺少的是一个更聚焦 coding workflow 的独立产品：它既要能复用现有 package 体系，又要在产品层清楚区分自己与 `weft-claw`、Claude Code、Codex、OpenCode 的边界。

本设计的目标，是定义一个名为 `WEFT-Code` 的独立 code 产品蓝图。它不是 `weft-claw` 的换皮，也不是简单模仿 Claude Code、Codex 或 OpenCode，而是一个：

- 以统一 runtime 为内核的本地 code product
- 以 CLI / TUI 为首批 clients，未来可扩展 GUI / IDE / remote client
- 以 `coding / plan / team` 为正式工作模式
- 以结构化 workflow、team runtime、可配置控制面为核心差异点
- 尽量复用现有 package，只有在产品边界必须独立时才新增包

本设计不要求一次性实现全部能力，但要求一次性把完整蓝图写清楚，避免后续继续重复补蓝图。

## 2. 产品定义总纲

`WEFT-Code` 是 WEFT 生态中的独立 code 产品：以统一 runtime 为内核，以 CLI/TUI/未来 GUI 为 clients，以 `coding / plan / team` 为正式工作模式，以 `project / session / task / team / approval / event stream` 为核心对象，复用现有 package 体系并通过结构化 workflow、team runtime 与可配置控制面形成区别于 Claude Code、Codex 与 OpenCode 的产品路径。

这个总定义可以拆成三个层次理解：

1. **它是一个真正可直接使用的 coding product**
   - 不是平台内部样板，也不是纯技术 demo
   - 用户应能直接用它完成真实仓库里的开发任务

2. **它的默认工作法是 workflow-first**
   - analyze → plan → execute → verify 不是提示词习惯，而是产品默认路径
   - 不同 mode 只是这条统一主链的不同执行方式

3. **它内生支持 team mode**
   - 不是后期外挂多 agent 功能
   - 而是基于正式 team runtime、task board、handoff、workflow phase 的协作执行模型

## 3. 与参考产品的关系

### 3.1 与 `weft-claw` 的关系

`weft-claw` 更偏向 WEFT 平台里的广义产品装配体，强调 feature/package/platform 语义；`WEFT-Code` 则是一个专门面向 coding workflow 的终端产品。

两者关系应定义为：

- `weft-claw` 继续承担平台产品装配与更广能力面的样板角色
- `WEFT-Code` 作为 WEFT 生态中的独立 code 产品，对外拥有自己的清晰身份
- 二者共享底层 package，不共享产品身份

因此，`WEFT-Code` 不应被定义成 `weft-claw` 的 mode、子形态或 branding 分支，而应视为平级产品。

### 3.2 对 Claude Code、Codex、OpenCode 的参考方式

本设计明确借鉴三者，但不照搬：

- **Claude Code**：主要借 runtime / task model / plan lineage / teammate / hooks / skills / memory 的内层骨架
- **OpenCode**：主要借统一 TUI 产品壳、agent 切换、项目规则入口、monorepo 生态组织与 client/server 演进思路
- **Codex**：主要借 config/schema、sandbox/approval/exec policy、slash commands、TUI 工程化设计与 app-server / SDK 分层

因此，`WEFT-Code` 的整体方向可以概括为：

- 外层是 OpenCode 式产品壳
- 内层有 Claude Code 式 runtime/task/mode 骨架
- 控制面吸收 Codex 的 config/approval/TUI discipline
- 真正差异来自 WEFT 自己的 workflow/team/package-first 内核

## 4. 设计结论总览

`WEFT-Code` 采用以下总体结构：

1. **Core Packages**
   - 提供 task、session、workflow、team、tool、memory、skills、MCP、approval 等核心能力

2. **Runtime Layer**
   - 统一承载 core，并对外暴露产品运行中的接口与事件流
   - 对外可直接以产品名 `WEFT-Code` 表达，不强制使用 `host/server` 技术术语

3. **Clients**
   - 首批为 CLI 与 TUI
   - 未来可扩展 GUI、IDE、remote client

4. **Workflow / Profile / Product Assembly**
   - 通过结构化配置与少量模板补充定义默认行为
   - 不允许 package 直接任意篡改 core 的原始 instructions

5. **Team Mode Execution Model**
   - 把协作对象、task board、handoff、review、integrate 提升为正式产品模型

## 5. 系统分层与运行时模型

### 5.1 Core 与 Runtime 分离

`WEFT-Code` 不应被实现成一个“大而全 CLI 程序”。

必须有明确的两层边界：

- **Core**
  - 负责 runtime 语义、task model、session、workflow orchestration、team runtime、tool dispatch、memory、skills、MCP、approval/policy
  - 它回答“系统如何工作”

- **Runtime**
  - 负责装载 core、维护运行中实例、暴露接口、提供事件流
  - 它回答“产品运行时如何被控制与观察”

Runtime 是产品的一部分，但不是额外的独立业务产品。对外可以直接视为 `WEFT-Code` 本身的运行接口层。

### 5.1.1 Runtime 内部建议模块

为了避免 runtime 在实现阶段重新长成另一个大单体，蓝图中应先承认以下内部模块边界：

1. **Runtime Bootstrap**
   - 负责 product package 与 instance config 装载
   - 负责默认 profile、默认 workflow/profile、默认 policy 的初始化

2. **Session Service**
   - 管理 session 生命周期
   - 管理 session lineage
   - 管理 session summary / compaction / resume

3. **Task Service**
   - 管理 task 创建、状态推进、归属关系与 task lineage

4. **Workflow Gateway**
   - 把 `coding / plan / team` 模式语义映射到具体 runtime 行为
   - 负责决定何时创建 plan task、teammate task、verification task

5. **Approval / Policy Service**
   - 统一管理 approval 请求、决策、策略命中与状态投影

6. **Event Publisher**
   - 把 session/task/tool/team/approval 变化统一发布到 event stream

这些模块不是为了制造额外抽象，而是为了保证：

- session 真相只有一个维护面
- task 真相只有一个维护面
- approval 不会散在 UI 和工具层之间
- event stream 不会由多个临时调用点各自拼装

### 5.1.2 Runtime 与 Core 的边界约束

Runtime 不应重新定义 core 的领域语义。它只负责：

- 装配
- 生命周期管理
- transport-facing resource projection
- event publishing

它不应负责：

- 重新发明 task 执行模型
- 重新发明 workflow 语义
- 重新发明 team runtime 真相
- 内联工具实现细节

因此，runtime 是产品化运行面，而不是第二个 core。

### 5.2 Clients 作为控制入口

CLI、TUI、未来 GUI 都应被视为 clients，而不是 core 的不同实现。

它们的职责是：

- 发送命令与消息
- 切换 mode
- 展示 task/session/team/approval 状态
- 订阅 event stream

它们不应拥有：

- session 真相
- team 真相
- workflow 真相
- approval 真相
- tool 执行真相

因此，第一阶段的 client 边界必须明确固定为：CLI / TUI / GUI 必须通过 WEFT-Code runtime 连接系统，不直接调用 core packages。

对于 P0，先约束并固定 runtime contract，而不直接实现完整 client：

- 支持的首批 runtime 资源：projects / sessions / session mode transition / teams / events
- 明确延后的 client 侧内容：full CLI、full TUI、persistent task replay、GUI client、remote runtime hosting

### 5.3 第一阶段部署模型

第一阶段采用折中模型：

- 对用户仍保持单产品体验
- 但架构上按 `core + runtime + clients` 设计
- CLI/TUI 默认可以无感拉起本地 runtime
- 后续可自然演进到 GUI / IDE / remote client 连接已有 runtime

这保证了：

- 现在不被分布式复杂度拖累
- 未来不需要重拆一遍系统

## 6. 包分层与命名策略

### 6.1 产品命名

本设计正式采用产品名：`WEFT-Code`。

命名原则：

- 产品层统一使用 `WEFT-Code`
- 新建且与产品强绑定的包可使用 `weft-code-*` 风格命名
- 现有通用底层包尽量保留已有命名，避免为品牌整齐进行无意义迁移

### 6.2 包分层

建议把 package 划分为以下几类：

1. **Core / Runtime Packages**
   - 现有 `agent-runtime`、`workflow-orchestrator`、`tool-runtime-core`、`memory-store`、`team-runtime` 等都属于这一层

2. **Feature Packages**
   - 组织 coding 与 team 相关能力块
   - 建议至少存在：
     - `weft-code-core-feature`
     - `weft-code-team-feature`

3. **Workflow / Profile Packages**
   - 承载默认 workflow、默认 profile、默认规则与少量模板
   - 用于声明默认行为，而不是直接改写 core

4. **Product Package**
   - `weft-code` 作为正式产品对象
   - 负责默认 feature、默认 workflow/profile、默认 policy 的装配

### 6.3 默认行为装配模型

默认行为不应通过“package 直接修改 core 指令文本”来实现。

正确方式是：

- 结构化配置负责定义主行为
- 模板只补充少量风格与流程引导
- 最终由 product / workflow / profile 装配出运行中的默认上下文

正式装配链定义为：

`core/runtime` 提供能力  
→ `feature/workflow/profile` 组织能力与默认行为  
→ `product` 选择正式产品装配  
→ `runtime` 运行并暴露接口  
→ `clients` 呈现与控制

## 7. 协议与资源模型

WEFT-Code 的协议设计必须围绕正式资源，而不是围绕某个单一前端页面。

### 7.1 Project

`project` 是最上层工作上下文，定义：

- 当前仓库/工作目录
- 项目级规则
- 项目级命令
- tool / workflow / profile 的作用范围

它不应只是一个隐式 cwd。

### 7.2 Session

`session` 是连续工作上下文，承载：

- 消息历史
- 当前 mode
- 当前 profile
- memory/summary/compaction 状态
- 从 plan 到 execute 到 team 的 lineage

session 不只是聊天记录，而是正式运行上下文。

建议蓝图里进一步明确 session 生命周期至少包括：

- `created`
- `active`
- `compacting`
- `paused`
- `resumed`
- `closed`

同时，session 应支持 lineage 关系，例如：

- coding session 进入 plan mode
- plan session 进入 team mode
- team session 收束回主 session

这些 lineage 即使第一阶段不全部持久化，也必须先作为正式模型存在。

### 7.3 Task

`task` 是运行时的执行单位。建议至少支持：

- `coding_task`
- `plan_task`
- `workflow_task`
- `teammate_task`
- `verification_task`
- `background_memory_task`

建议 task 至少具备以下最小状态机：

- `queued`
- `running`
- `blocked`
- `waiting_approval`
- `completed`
- `failed`
- `cancelled`

同时应明确三类关系：

- task 属于哪个 session
- task 是否属于某个 team
- task 是否由另一个 task 或 phase 派生

### 7.4 Mode

mode 是 session 级正式状态，至少包括：

- `coding`
- `plan`
- `team`

后续可扩展 `review` / `remote`。

蓝图里建议明确 mode 的典型切换路径：

- `coding -> plan`
  - 用户显式要求先规划
  - 或系统判断任务复杂度超出直接执行边界

- `plan -> coding`
  - 用户确认计划进入执行

- `coding -> team`
  - 用户显式要求协作
  - 或任务已明显需要分角色执行

- `plan -> team`
  - 规划阶段已得出明确角色拆分与阶段划分

- `team -> coding`
  - 协作结束并收束回主线

mode 切换必须产生正式 runtime 事件，供所有 clients 观察。

### 7.5 Team

`team` 是正式协作上下文，不是 agent 列表。它应包含：

- 角色与成员
- 共享上下文策略
- task board
- handoff 记录
- workflow phase
- review / integrate 状态

### 7.6 Approval

approval 与 permission 必须是正式资源，而不是 UI 附件。

它们会影响：

- shell
- file write
- git
- network
- MCP
- team delegate

### 7.7 Event Stream

事件流应统一暴露：

- message delta
- task lifecycle
- tool call lifecycle
- approval request / resolution
- mode change
- team board update
- verification progress
- session compaction / summary update

## 8. 默认工作流与模式语义

WEFT-Code 的统一主链固定为：

`analyze → plan → execute → verify`

这不是建议，而是产品默认工作法。

### 8.1 Coding Mode

`coding mode` 是默认模式：

- 面向日常、局部、低协调成本任务
- 仍然会 analyze
- plan 可以轻量化
- execute 与 verify 可以更快速进入

其核心不是“跳过流程”，而是“轻量使用统一主链”。

### 8.2 Plan Mode

`plan mode` 是正式规划模式：

- 强化 analyze 与 plan
- 默认冻结 execute
- 适合复杂任务、高风险任务、架构调整与 team 前置规划

它必须是正式 mode，而不是简单命令别名。

### 8.3 Team Mode

`team mode` 是统一主链的协作执行版本：

- planner 强化 analyze / plan
- implementer 负责 execute
- reviewer 强化 verify
- integrator 负责结果收束

因此：

- `coding mode`：轻量使用统一主链
- `plan mode`：强化前两段，冻结后两段
- `team mode`：把统一主链拆成多角色协作执行

## 9. Team Mode 作为正式执行模型

### 9.1 Team Session

team mode 应基于正式 `team session`，而不是临时多 agent 群聊。

它定义：

- 角色集
- 共享上下文策略
- 是否允许自动 delegate
- handoff 规则
- task board 与 workflow 绑定关系

### 9.2 Canonical Roles

完整蓝图里应直接承认 canonical roles：

- planner
- implementer
- reviewer
- integrator

后续可以扩展，但第一版不宜过散。

### 9.3 Task Board 作为事实源

board 必须是 team mode 的事实源，记录：

- task
- review item
- blocked 状态
- handoff
- integration 状态
- activity log

### 9.4 Workflow Phase

team mode 不应悬浮在 workflow 之外。board/task 最好明确属于：

- analyze
- plan
- execute
- verify
- integrate

### 9.4.1 Delegate、Handoff、Review 的关系

为了避免 team mode 退化成“多个 agent 各做各的”，必须提前把三种正式关系写清楚：

- **delegate**
  - 表示主执行面把某个正式工作项分派给特定 role 的 teammate task

- **handoff**
  - 表示某个 teammate task 的输出被转移到下一个 role 或 phase

- **review**
  - 表示某项输出进入正式审查，而不是聊天里一句“我看过了”

其中：

- delegate 产生新的 teammate task
- handoff 连接两个 task / phase
- review 产生 review item，并影响 integrate 是否可继续

### 9.4.2 Team Board 的最小字段建议

为了让 CLI/TUI/GUI 后续可以一致展示，board item 最小建议包含：

- `id`
- `parent_session_id`
- `team_id`
- `role`
- `phase`
- `status`
- `title`
- `input_summary`
- `output_summary`
- `blocked_reason`
- `next_handoff_target`
- `created_at` / `updated_at`

### 9.5 Team Mode 原则

- 默认由主 session 发起 team
- delegate 必须落到正式 teammate task
- 每个 teammate task 都有 owner role、input context、output artifact、next handoff target
- verify 与 integrate 不是可选，而是正式阶段
- team mode 结束时必须收束回主 session

## 10. 统一 TUI 与 CLI 控制层

### 10.1 角色定位

CLI 与 TUI 都是 `WEFT-Code` 的 clients：

- CLI：命令式入口、脚本模式、自动化友好
- TUI：默认交互主壳、长会话、mode 切换、task/session/team 可视化

它们共享同一 runtime 与同一套 mode 语义。

### 10.2 TUI 顶层信息架构

建议 TUI 至少包含五块区域：

1. 顶栏：project / session / mode / model / approval / policy
2. 主消息区：消息、plan、tool、verify、handoff 摘要
3. 侧栏或面板：session 列表、task board、文件变更、diagnostics
4. 输入区：自然语言输入、slash commands、命令补全、模式入口
5. overlay：permissions、palette、session switch、team setup、review details

进一步建议 TUI 至少支持三种可切换观察焦点：

- **conversation focus**：消息、工具输出、模型流
- **task focus**：当前 task、verification、diagnostics
- **team focus**：board、handoff、review、integrate 状态

这三者是同一 runtime 状态的不同投影视图，不应被设计成三套互不相干的产品。

### 10.3 模式切换原则

mode 切换必须是可见状态，而不是隐藏 prompt 魔法。

用户应明确知道自己当前在：

- `coding`
- `plan`
- `team`

切换后的状态应在顶栏、输入区、任务区都可见。

## 11. 配置、规则、命令与权限模型

### 11.1 配置层级

至少存在三层配置：

1. 全局配置
   - 默认模型/provider、approval 策略、默认 profile、MCP servers、日志等
2. 项目配置
   - 项目级规则、默认 workflow、命令集合、工具范围、团队模板
3. 会话级覆盖
   - 当前 session 的临时 mode、approval override、team setup、额外目录、memory 行为

### 11.2 项目级规则

WEFT-Code 应支持正式项目规则入口，用于承载：

- 项目结构说明
- 开发命令
- 验证要求
- 禁止事项
- 默认 workflow
- 何时进入 plan / team mode
- 参考目录 / 生成物 / 只读区说明

这些内容应由 runtime 正式加载，而不是散落在 README 与个人经验中。

建议规则优先级明确为：

1. product default
2. global user config
3. project rules
4. session override

其中 project rules 优先影响：

- 默认 workflow
- 推荐 mode 切换
- 验证入口
- 可用命令
- 受保护目录与只读目录

### 11.3 命令模型

建议同时支持三类命令：

- 内建命令：如 init / plan / team / review / compact / share / session / config
- 项目命令：由项目配置或项目命令目录声明
- slash / palette commands：供 TUI 与 CLI 交互入口使用

### 11.4 权限与 Approval

approval 必须是正式运行时对象，不是 UI 小组件。

需特别明确：

- approval 不等于 sandbox
- approval 是交互与治理层
- sandbox 是更底层的执行隔离能力

两者可以同时存在，但不能混为一体。

建议 approval policy 在蓝图里至少承认以下形态：

- `always_allow`
- `on_request`
- `on_sensitive_actions`
- `read_only_mode`

是否进一步细分到 shell/write/git/network/MCP 的独立策略，可以在实现阶段收敛，但蓝图里应先承认 policy 会影响不同动作类型。

## 12. 复用策略

第一优先原则是：**能复用就复用。**

建议优先复用的现有 package 包括：

- `agent-runtime`
- `prompt-system`
- `workflow-orchestrator`
- `tool-runtime-core`
- `tool-shell`
- `tool-files`
- `tool-git`
- `memory-store`
- `skills-runtime`
- `mcp-client`
- `team-runtime`
- `team-task-board`
- `workflow-template-devteam`

原则是：

- 底层实现尽量不重写
- 产品身份、feature 组合、workflow/profile 装配重新定义
- 只有在现有包无法承载产品边界时，才新增包

## 13. 分期路线图

蓝图一次写完整，但实现按阶段推进。

### 13.1 P0：架构骨架成立

目标：把 `WEFT-Code` 的产品骨架坐实。

至少应成立：

- core 独立
- runtime 独立
- CLI/TUI 连接 runtime
- `coding / plan / team` 三个 mode 成立
- `project / session / task / approval / event stream` 对象成立

重点是系统骨架，不是交互美化。

### 13.2 P1：单人 Coding 体验做强

目标：让 `WEFT-Code` 在单人开发上足够可比现有产品。

重点增强：

- TUI 信息架构
- slash commands / palette
- session 恢复与切换
- plan mode 完整体验
- verification / diagnostics / git / file change 可视化
- 项目级 rules / commands / profiles

P1 的验收重点是：

- 单人 coding 体验已足够与现有产品同台比较
- plan mode 不再只是能力，而是稳定正式体验
- runtime 协议已经足以支撑一个真实 TUI client

### 13.3 P2：Team Mode 与多 Client 生态做强

目标：把差异化优势做出来。

重点增强：

- team session
- task board
- handoff
- review / integrate
- teammate task
- GUI / IDE / remote client 初步接入

P2 的验收重点是：

- team mode 成为正式主打能力，而非附属增强
- board / handoff / review / integrate 能稳定驱动协作
- 同一 runtime 被多个 clients 复用的路径已经成立

## 14. 明确非目标

以下内容进入蓝图，但不要求早期立刻实现：

- 完整 GUI
- 更复杂的 remote / bridge / mobile 控制
- 长期跨项目记忆
- 更细粒度角色系统
- 更广的 plugin / ecosystem / marketplace
- 企业级多用户部署与网络化运行

这类能力应在蓝图中占位，但不提前拖慢前期产品收敛。

## 15. 风险与设计约束

### 15.1 不允许 package 任意改写 core

如果让 package 直接修改 core 指令与调用参数，最终一定失控。必须坚持：

- core 保持稳定
- 默认行为来自结构化装配
- 模板只做补充
- 行为变化必须可追踪到具体 product / feature / profile / session override

### 15.2 不把 TUI/CLI 逻辑塞回 core

如果前端控制逻辑反向侵入 core，后续 GUI/IDE/remote client 接入会再次分裂。

### 15.3 不让 Team Mode 退化为多 Agent 聊天

team mode 必须坚持 session、board、handoff、phase、review/integrate 的正式结构。

## 16. 验收标准

本设计成立后，应满足以下标准：

1. `WEFT-Code` 被定义为 WEFT 生态中的独立 code 产品，而非 `weft-claw` 的子模式
2. `core / runtime / clients` 的边界被清晰定义
3. `coding / plan / team` 作为正式模式被清晰定义
4. `project / session / task / team / approval / event stream` 成为正式对象模型
5. 现有 package 的复用原则被明确，新增包只围绕产品边界发生
6. `team mode` 被定义为正式执行模型，而不是附加功能
7. 蓝图足以支撑后续 implementation plan，不需要再回头重做产品级设计

## 17. 一句话结论

`WEFT-Code` 不应被实现为另一个“会聊天的 coding CLI”，而应被设计成：

**一个以统一 runtime 为核心、以 workflow-first 为默认工作法、以内生 team mode 为正式执行模型、以 CLI/TUI/未来 GUI 共享同一内核的独立 code product。**
