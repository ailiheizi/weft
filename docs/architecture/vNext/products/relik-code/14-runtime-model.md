# WEFT vNext 蓝图：WEFT-Code Runtime + Clients 模型

本文定义 WEFT-Code 如何在不破坏 Product Package + Instance 模型的前提下，引入统一 runtime 与多客户端结构。

## 一、为什么需要这篇蓝图

`../../foundations/13-product-package-instance-model.md` 已经把 vNext 的顶层模型收敛成了：

- 源码层只有 Package
- 运行层由 Instance 承担

这个收敛解决了源码发布对象重复的问题，但还没有单独回答一个更具体的产品问题：

**当某个 product package 真正变成一个可直接使用的 code product 时，它的运行时接口应该如何组织？**

`WEFT-Code` 正好是这个问题的第一块试金石。

它不是一个泛平台样板，而是一个明确的 coding product：

- 需要长期会话
- 需要任务对象
- 需要 mode 切换
- 需要 team 协作
- 需要 CLI / TUI / 未来 GUI 共享同一内核

因此，vNext 需要一篇单独蓝图，来定义：

- Core 如何被承载成一个产品 runtime
- runtime 如何暴露正式资源与事件流
- clients 如何在不直接调用 core packages 的前提下控制产品

这一层不是回到旧的 App 源码层，而是：

**在 Product Package + Instance 模型之上，定义 code product 的运行态架构。**

## 二、本文要解决的问题

本文专门回答以下问题：

1. `WEFT-Code` 如何从一个 product package 变成真正运行中的 code product
2. `core / runtime / clients` 的职责边界是什么
3. 为什么 CLI / TUI / GUI 不应该直接调用 core packages
4. `project / session / task / mode / team / approval / event stream` 为什么必须成为正式资源
5. team mode 为什么必须是正式执行模型，而不是“多 agent 聊天增强”

本文不直接规定具体 UI 样式，也不替代实现计划；它的职责是把产品运行时结构固定下来，避免以后实现时反复改模型。

## 三、WEFT-Code 的产品定位

`WEFT-Code` 是 WEFT 生态中的独立 code 产品。它和 `weft-claw` 的关系是：

- 两者共享底层 package 体系
- `weft-claw` 更偏产品装配样板与广义 AI 产品能力面
- `WEFT-Code` 更聚焦 coding workflow 与代码任务闭环

因此，`WEFT-Code` 不应被视为 `weft-claw` 的一个 mode 或二级产品，而应被视为：

**第一个明确以 coding workflow 为中心的 WEFT product package。**

它的默认工作法不是自由聊天，而是 workflow-first：

- analyze
- plan
- execute
- verify

它的核心差异也不在 UI 外观，而在于：

- 统一 runtime
- 正式 task model
- 正式 mode 语义
- 正式 team execution model
- 可配置控制面

## 四、三层结构：Core、Runtime、Clients

- Core 提供能力与领域真相
- Runtime 承载并暴露 WEFT-Code 的运行中接口
- CLI / TUI / GUI 作为 clients 连接 runtime

### 4.1 Core

Core 仍然是能力与领域真相的中心。

在 WEFT-Code 的场景里，Core 至少负责：

- task model
- session state
- workflow orchestration
- tool dispatch
- team runtime
- memory
- skills
- MCP
- approval / policy

Core 回答的问题是：

**“系统如何工作？”**

Core 不应回答这些问题：

- 当前应该显示什么 UI 布局
- 当前使用的是 CLI 还是 TUI
- 当前的 overlay/command palette 怎么呈现
- 某个 client 如何缓存自己的草稿状态

### 4.2 Runtime

Runtime 是 WEFT-Code 的运行时接口层。它承载 core，并把产品的运行中状态通过统一协议暴露出来。

Runtime 负责：

- 装载 WEFT-Code product package 默认行为
- 结合 instance config 形成当前运行态
- 承载 core 的运行中实例
- 暴露正式资源
- 暴露事件流
- 接受 clients 的控制请求

Runtime 回答的问题是：

**“运行中的 WEFT-Code 如何被控制与观察？”**

对外文档与产品语言里，可以直接把 runtime 视为“WEFT-Code 本身的本地运行接口”。没有必要强迫用户理解 `host`、`daemon`、`server` 这些术语。

### 4.3 Clients

CLI、TUI、未来 GUI 都是 clients。

Clients 负责：

- 发消息
- 发命令
- 切 mode
- 展示 task / session / team / approval 状态
- 订阅 event stream

Clients 不应拥有：

- task 真相
- workflow 真相
- team 真相
- approval 真相
- tool 执行真相

Clients 回答的问题是：

**“用户如何操作 WEFT-Code？”**

### 4.4 为什么不能让 Clients 直接调用 Core

如果 CLI / TUI / GUI 直接调用 core packages，会立刻出现三个问题：

1. 每个 client 都会长出一套自己的业务真相
2. GUI/IDE/remote client 后续接入时必须重新复制逻辑
3. session、task、team、approval 的语义无法统一

因此：

**clients 必须只通过 runtime 访问系统，而不是直接触碰 core packages。**

## 五、部署与运行模型

第一阶段的 WEFT-Code 采用“本地 runtime + 多 client”的模型。

### 5.1 第一阶段目标

- 用户运行 CLI 或打开 TUI
- client 自动拉起本地 WEFT-Code runtime（若尚未存在）
- client 通过统一协议连接 runtime
- runtime 持有 session/task/team 等运行时真相

用户感知上，它仍然像一个单体产品；架构上，它已经是多 client 共用一个 runtime。

### 5.2 为什么不直接做成纯单体 CLI

如果一开始把全部逻辑都塞进 CLI/TUI，本地体验可能更快跑通，但后续会立刻遇到问题：

- GUI 无法复用
- IDE 无法复用
- 同一会话无法被多个入口共享
- event stream 只能做成 UI 内部私有结构

因此，第一阶段虽然仍允许“无感本地启动”，但模型上必须是 runtime-first，而不是 CLI-first。

### 5.3 未来演进方向

这一模型天然支持未来演进到：

- GUI 连接已有 runtime
- IDE extension 连接已有 runtime
- remote client 连接本地运行中的 runtime
- 多入口共享同一 session/task 状态

这不是第一阶段必须完成的功能，但必须在蓝图里预留出来。

## 六、协议形态：HTTP + Event Stream

WEFT-Code 不是普通 CRUD 系统，因此不适合做成纯 RESTful 资源服务；但它也不需要一开始就完全押注到复杂 RPC。

本文明确采用混合协议形态：

- **资源操作**：HTTP 风格
- **流式状态更新**：Event Stream（如 SSE）

### 6.1 资源操作适合承载的对象

- project
- session
- task
- mode transition
- team creation / listing
- approval decision
- config / rules / command metadata

### 6.2 Event Stream 适合承载的对象

- message delta
- tool lifecycle
- task lifecycle
- mode changed
- team board update
- verification progress
- approval requested / resolved
- session summary / compaction update

### 6.3 为什么不是纯 REST

因为 WEFT-Code 是：

- 强会话
- 强流式
- 强状态演化
- 强协作

如果只做 REST，clients 会被迫自己轮询和拼状态，这会直接破坏 `runtime` 作为系统真相中心的意义。

## 七、正式资源模型

WEFT-Code runtime 暴露的对象，必须围绕正式资源设计，而不是围绕某个前端页面结构设计。

### 7.1 Project

`project` 是最上层工作容器，用于定义：

- 当前仓库/工作目录
- 项目级规则
- 项目级命令
- workflow/profile/tool 作用范围

它不应只是一个隐式 cwd。否则将来会话恢复、team 运行、规则加载都会失去清晰边界。

### 7.2 Session

`session` 是连续工作上下文，而不是普通聊天记录。

它至少应包含这些语义：

- 当前 mode
- 当前 profile
- 当前消息历史
- 当前工作摘要与 memory 状态
- 当前 lineage（例如 plan → execute → team）

Session 的存在，是为了让 WEFT-Code 成为一个持续工作的开发系统，而不是一次性问答器。

### 7.3 Task

`task` 是运行时的执行单位。建议至少存在：

- `coding_task`
- `plan_task`
- `workflow_task`
- `teammate_task`
- `verification_task`
- `background_memory_task`

Task 的引入有两个关键价值：

1. 把“系统正在做什么”从消息流里抽出来
2. 为 team mode、board、review、verification 提供稳定执行单位

### 7.4 Mode

mode 是 session 级正式状态，而不是 UI 小开关。

第一版至少承认：

- `coding`
- `plan`
- `team`

mode 会影响：

- 默认 workflow 行为
- 默认 agent/profile 选择
- 工具使用策略
- 输出组织方式
- task 生成方式

### 7.5 Team

`team` 不是 agent 列表，而是正式协作上下文。

它应包含：

- 角色与成员
- 共享上下文策略
- 任务板
- handoff 记录
- workflow phase
- review / integrate 状态

### 7.6 Approval

approval 必须是正式资源。

它不只是前端弹窗，而是 runtime 层的正式对象，影响：

- shell
- write
- git
- network
- MCP
- team delegate

并且必须明确区分：

- approval：产品治理与交互控制
- sandbox：更底层的执行隔离

### 7.7 Event Stream

event stream 是 clients 共享 runtime 的关键，不是附属功能。

它至少需要统一暴露：

- message delta
- task lifecycle
- tool lifecycle
- mode changed
- team board update
- approval event
- verification progress
- session summary update

## 八、默认工作流与 Mode 语义

WEFT-Code 的统一主链固定为：

`analyze → plan → execute → verify`

这不是建议，而是产品默认工作法。

### 8.1 Coding Mode

`coding mode` 是默认模式：

- 用于日常、局部、低协调成本任务
- 仍然会先 analyze
- plan 允许压缩为轻量形式
- execute 与 verify 可更快进入

关键点在于：

它不是跳过流程，而是轻量使用统一主链。

### 8.2 Plan Mode

`plan mode` 是正式规划模式：

- 强化 analyze 与 plan
- 默认冻结 execute
- 适合复杂任务、风险任务、架构任务、team 前置规划

Plan mode 不应只是一个 prompt 别名，而必须是 runtime 内部有正式状态语义的 mode。

### 8.3 Team Mode

`team mode` 是统一主链的协作执行版本：

- planner 强化 analyze / plan
- implementer 承担 execute
- reviewer 强化 verify
- integrator 承担收束与整合

所以 mode 和 workflow 的关系应被正式定义为：

- `coding`：轻量执行统一主链
- `plan`：强化前两段，冻结后两段
- `team`：把统一主链拆成多角色协作执行

## 九、Team Mode 作为正式执行模型

Team mode 必须被视为正式执行模型，而不是单 agent 的增强插件。

## 初始 runtime 对外接口

第一阶段先固定 runtime 作为唯一 client-facing control plane。当前约束的首批资源与路由为：

- projects
  - `GET /api/weft-code/projects`
- sessions
  - `GET /api/weft-code/sessions`
  - `GET /api/weft-code/session/{session_id}`
- session mode transition
  - `POST /api/weft-code/session/{session_id}/mode`
- teams
  - `GET /api/weft-code/teams`
  - `POST /api/weft-code/session/{session_id}/team`
  - `GET /api/weft-code/team/{team_id}/tasks`
- events
  - `GET /api/weft-code/events`

当前 `events` 路由是 read-only timeline snapshot contract：它通过 `weft-core` 的 `/api/weft-code/events` compatibility route 调用 installed WASM package `weft_code.runtime` 的 `list_events` action，并返回 deterministic array。第一版事件只描述 runtime bootstrap、current policy、当前 sessions 与 approvals 的快照，每条 event 带有稳定的 `id`、`kind`、`source`、`resource`、`summary` 与递增 `sequence`。

这个 endpoint 不是 SSE，不是 long-polling，也不是 live event stream；读取它不得创建 task、推进 approval、触发 execution，或修改 runtime state。真正的 streaming timeline、TUI timeline UI 与 workflow/event replay 仍属于后续阶段，必须在新的 scope 中单独定义。

这些接口的目标不是一次性做成完整产品协议，而是先把 runtime contract 固定下来，让 CLI / TUI / 未来 GUI 围绕同一控制面演进。

### P0 暂不覆盖的内容

以下内容在 P0 明确延后：

- full CLI implementation
- full TUI implementation
- persistent task replay
- GUI client
- remote runtime hosting

这些延后项不应反向污染当前 runtime 资源模型；P0 先固定 runtime contract，再逐步扩展具体 client 形态。

CLI / TUI / GUI 必须通过 WEFT-Code runtime 连接系统，不直接调用 core packages。

### 9.1 Team Session

进入 team mode 后，系统应创建正式 team session，而不是只在主 session 中临时加几个 agent。

team session 定义：

- 角色集合
- 自动 delegate 规则
- handoff 规则
- board 绑定关系
- workflow phase 绑定关系

### 9.2 Canonical Roles

第一版蓝图直接承认以下 canonical roles：

- planner
- implementer
- reviewer
- integrator

这不是限制未来扩展，而是为了让第一版协作模型先收敛。

### 9.3 Task Board

Task board 是 team mode 的事实源，必须承载：

- task
- review item
- blocked 状态
- handoff
- integration 状态
- activity log

任何 TUI/CLI/GUI 都应通过 runtime 读取 board，而不是自行从聊天记录推断协作状态。

### 9.4 Workflow Phase

team mode 不应悬浮在 workflow 之外。task 和 board 最好显式属于：

- analyze
- plan
- execute
- verify
- integrate

这样 team mode 就是 workflow 的协作版本，而不是另一套独立系统。

### 9.5 Team Mode 原则

必须坚持以下原则：

- delegate 必须落到正式 teammate task
- verify 与 integrate 不是可选动作
- team mode 结束时必须收束回主 session
- 多 agent 的价值来自职责、状态、交接和审查，而不是数量本身

## 十、CLI / TUI / GUI 的关系

### 10.1 CLI

CLI 是命令式控制入口，适合：

- 非交互调用
- 脚本
- CI / automation
- 快速命令式控制

### 10.2 TUI

TUI 是默认交互主壳，适合：

- 长会话
- mode 切换
- task/session/team 可视化
- approval 可视化
- 复杂 workflow 的持续跟踪

### 10.3 GUI

GUI 不是第一阶段必须交付，但它必须被视为 runtime 的自然复用目标，而不是未来重新做一套系统的理由。

### 10.4 共同原则

CLI / TUI / GUI：

- 共享同一 runtime
- 共享同一资源模型
- 共享同一 mode 语义
- 不共享各自私有业务真相

## 十一、与 Product Package + Instance 模型的关系

`WEFT-Code` 不是旧式 App 模型的复活。

它仍然严格建立在：

- `weft-code` 作为 product package
- instance 作为运行中的配置与 lock 载体

runtime 的引入，只是为了回答：

**“当一个 product package 真正运行成 code product 时，客户端如何控制它？”**

因此，runtime：

- 不是新的源码发布对象
- 不是新的产品身份层
- 不是替代 product package 的概念

它只是：

**product package 在本地运行时暴露出来的统一控制与观察面。**

## 十二、一句话结论

`WEFT-Code` 在 vNext 中应被定义为：

**一个建立在 Product Package + Instance 模型之上的独立 code product，其中 Core 提供能力真相，Runtime 暴露产品运行接口，CLI/TUI/GUI 作为 clients 共享同一会话、任务、协作与事件流模型。**
