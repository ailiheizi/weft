# WEFT vNext 蓝图：Weft Claw DevTeam Feature 的 Package-First 设计

## 一、文档目的

这份文档定义 `weft-claw devteam` 的 v1 设计，作为 `../09-product-blueprint.md` 与 `../10-capability-package-matrix.md` 的直接延伸。

它回答的问题不是“要不要做一个团队模式”，而是：

- 在不破坏 `weft-claw` 既有 product package 边界的前提下，如何把“开发团队式协作”建模为一组明确的 package 与 capability
- 如何让 devteam 成为 `weft-claw` 中一个正式的 feature 组合，而不是继续把团队逻辑塞回 core、agent runtime 或 product package
- 如何给后续实现提供可执行的 package 列表、能力面、对象模型、运行边界、绑定关系和实施顺序

## 二、问题陈述

Weft Claw 当前蓝图已经清晰覆盖了 chat、automation、extensions、channel 等产品能力，但还没有一个正式的、package-first 的“多角色开发团队协作”模型。

如果没有这层模型，后续实现很容易滑向三种错误方向：

1. 直接把团队编排、任务板、角色切换、上下文转发等逻辑塞进 `weft-claw` product package
2. 把团队协作误写进 `agent.runtime` 或 `workflow.orchestration`，让 foundation package 承担产品语义
3. 把“多 agent 协作”做成过重的 swarm 系统，在 v1 就引入不必要的复杂分布式协调

因此，`weft-claw devteam` 必须被定义为：

**一个由明确 capability packages、少量 foundation/feature 组合和 product package 装配出来的开发团队协作能力，而不是产品内置模式开关。**

## 三、设计目标

### 3.1 v1 目标

`weft-claw devteam` v1 必须满足以下目标：

1. 以 package-first 方式提供团队协作能力，不把团队逻辑写回 core 或 product package
2. 把开发团队最小闭环落到明确 package：
   - `team-runtime`
   - `team-task-board`
   - `workflow-template-devteam`
   - `weft-claw-devteam-feature`
3. 复用既有能力而不是重写：
   - `delegate`
   - `workflow.orchestration`
   - `memory.store`
   - `channel.bridge`
   - `ext.mcp`
   - `ext.skills`
   - `agent.runtime`
4. 支持单工作区内、单产品上下文下的多角色协作
5. 支持明确任务板、角色分工、交接与结果归档
6. 让 product package 只负责默认启用、默认 binding、入口暴露和 feature 开关
7. 为 v2 保留扩展空间，但 v1 不引入 swarm 级复杂自治系统

### 3.2 非目标

v1 明确不做以下事情：

1. 不做开放式 swarm intelligence
2. 不做去中心化 agent 网络
3. 不做自动自繁殖式角色生成
4. 不做跨多个 workspace 的分布式任务编排
5. 不做复杂组织管理模型，例如部门、层级审批、多人并发冲突解决
6. 不做独立于 `weft-claw` 之外的通用团队产品壳
7. 不把 task board 做成完整项目管理系统替代品

## 四、设计原则

### 4.1 Package First

devteam 的产品语义必须主要落在 feature package 和相关能力包中，不允许默认回收到 core 或 product package。

### 4.2 复用优先

凡是已经存在于 `agent.runtime`、`workflow.orchestration`、`memory.store`、`channel.bridge`、`ext.mcp`、`ext.skills`、`delegate` 的能力，devteam 只做组合与约束，不做平行重写。

### 4.3 团队语义高于 agent 细节

`agent.runtime` 负责 agent turn loop，`team-runtime` 负责团队成员生命周期、角色约束、上下文路由和协作协议。不能反过来把团队逻辑写进 agent foundation。

### 4.4 任务板是协作事实源

v1 中，团队协作的共享事实源应是 `team-task-board`，而不是散落在消息流、agent 内存或产品 UI 临时状态里。

### 4.5 工作流模板独立表达

开发团队执行流程应通过 `workflow-template-devteam` 表达，而不是把“计划、分派、执行、评审、收尾”的流程写死在 `team-runtime` 里。

### 4.6 Product Package 只保留产品装配权

`weft-claw` product package 只决定是否启用 `weft-claw-devteam-feature`、默认 role/profile、默认模板和对外入口，不承载团队执行实现。

### 4.7 v1 保持中控式协调

v1 采用受控协调模型，即一个明确 team session + task board + workflow template 的装配方式。它可以有多个角色，但不追求 swarm 式自由并发自治。

## 五、建议文档命名与仓库放置约定

建议在 `docs/architecture/vNext/products/weft-claw/devteam/` 下按顺序新增：

- `11-feature-design.md`

后续若需要更细矩阵，再增加：

- `12-capability-package-matrix.md`

## 六、分层定位

`weft-claw devteam` 应按以下四层理解：

1. Capability Provider / Specialized Packages
2. Foundation Reuse and Coordination Packages
3. Weft Claw DevTeam Feature Package
4. Weft Claw Product Package Integration

## 七、Canonical Package 列表

### 7.1 新增 package

#### `team-runtime`

职责：

- 团队会话生命周期管理
- 团队成员实例装配
- 角色约束与角色能力面校验
- 团队内 delegate 路由
- 团队共享上下文投影
- 团队工作循环协调

#### `team-task-board`

职责：

- 团队任务、状态、指派、依赖关系的 canonical 模型
- task / subtask / handoff / review item 管理
- 团队执行事实记录
- 与 workflow state 的对齐快照

#### `workflow-template-devteam`

职责：

- devteam 默认工作流模板定义
- 阶段模型，例如 intake、plan、execute、review、integrate、done
- 角色切换规则
- 任务状态推进规则

#### `weft-claw-devteam-feature`

职责：

- 把 team runtime、task board、workflow template 与既有 foundation 组合成产品能力
- 暴露 devteam 能力开关和默认依赖
- 提供 product package 可装配的稳定团队功能块

### 7.2 复用 package

- `agent-runtime`
- `workflow-orchestrator`
- `memory-store`
- `channel-core`
- `mcp-client`
- `skills-runtime`
- `delegate`

## 八、Canonical Capability 列表

建议新增 capability：

- `team.runtime`
- `team.taskboard`
- `workflow.template.devteam`
- `team.role.catalog`
- `team.delegate`
- `team.handoff`
- `team.context.shared`

### 8.1 capability 定义

#### `team.runtime`

默认 provider：`team-runtime`  
mutability：条件替换

#### `team.taskboard`

默认 provider：`team-task-board`  
mutability：Mutable

#### `workflow.template.devteam`

默认 provider：`workflow-template-devteam`  
mutability：条件替换

#### `team.role.catalog`

默认 provider：`team-runtime`  
mutability：条件替换

#### `team.delegate`

默认 provider：复用 `delegate`  
mutability：Mutable 或高约束 mutable

#### `team.handoff`

默认 provider：`team-task-board`  
mutability：Mutable

#### `team.context.shared`

默认 provider：`memory-store` + `team-runtime`  
mutability：Mutable

## 九、Capability 到 Package 的建议矩阵

| Capability | Provider Package | Foundation / Reuse | Feature Package | Product Package 责任 | 默认 provider | Mutability |
| --- | --- | --- | --- | --- | --- | --- |
| `team.runtime` | `team-runtime` | `agent-runtime`, `delegate` | `weft-claw-devteam-feature` | 选择默认团队模式与入口 | `team-runtime` | 条件替换 |
| `team.taskboard` | `team-task-board` | `memory.store` | `weft-claw-devteam-feature` | 控制是否暴露任务板 UI | `team-task-board` | Mutable |
| `workflow.template.devteam` | `workflow-template-devteam` | `workflow.orchestration` | `weft-claw-devteam-feature` | 选择默认 devteam workflow 模板 | `workflow-template-devteam` | 条件替换 |
| `team.role.catalog` | `team-runtime` | `agent-runtime`, `ext.skills` | `weft-claw-devteam-feature` | 选择默认角色集 | `team-runtime` | 条件替换 |
| `team.delegate` | `delegate` | `agent-runtime` | `weft-claw-devteam-feature` | 控制是否允许角色间委派 | `delegate` | Mutable |
| `team.handoff` | `team-task-board` | `memory.store` | `weft-claw-devteam-feature` | 控制交接记录可见性 | `team-task-board` | Mutable |
| `team.context.shared` | `memory-store` + `team-runtime` | `memory.store` | `weft-claw-devteam-feature` | 控制团队上下文是否启用 | `memory-store` + `team-runtime` | Mutable |

## 十、对象模型

### 10.1 `DevTeamSession`

核心字段建议：

- `session_id`
- `app_id`
- `workspace_id`
- `feature_id`
- `workflow_template_id`
- `status`
- `active_board_id`
- `shared_context_ref`

### 10.2 `DevTeamMember`

核心字段建议：

- `member_id`
- `role_id`
- `display_name`
- `agent_binding`
- `skill_set_refs`
- `state`
- `current_task_id`

### 10.3 `DevTeamRole`

v1 建议内置最小角色集：

- `planner`
- `implementer`
- `reviewer`
- `integrator`

### 10.4 `TeamTaskBoard`

核心字段建议：

- `board_id`
- `session_id`
- `tasks[]`
- `review_items[]`
- `handoffs[]`
- `activity_log[]`

### 10.5 `TeamTask`

核心字段建议：

- `task_id`
- `title`
- `description`
- `kind`
- `status`
- `owner_member_id`
- `depends_on[]`
- `artifact_refs[]`
- `review_state`

### 10.6 `TeamHandoff`

核心字段建议：

- `handoff_id`
- `from_member_id`
- `to_member_id`
- `task_id`
- `reason`
- `context_snapshot_ref`
- `expected_outcome`

### 10.7 `SharedTeamContext`

核心字段建议：

- `context_id`
- `session_id`
- `memory_refs[]`
- `decision_log[]`
- `active_constraints[]`
- `workspace_summary`
- `open_risks[]`

## 十一、运行时边界

### 11.1 Core 边界

Core 继续负责：

- capability registry
- package 装载
- binding / resolve / verify / activate
- policy / profile enforcement

Core 不负责：

- team session 状态机
- team role 语义
- team task board 数据模型

### 11.2 `agent-runtime` 边界

`agent-runtime` 负责单 agent 执行，不负责团队成员关系、任务板和团队级 handoff。

### 11.3 `workflow-orchestrator` 边界

`workflow-orchestrator` 负责流程编排引擎，不负责 devteam 专属阶段定义、团队角色权限模型和 task board 持久化。

### 11.4 `team-runtime` 边界

`team-runtime` 负责：

- 组装团队成员
- 协调 delegate 与 handoff
- 对接 task board 和 shared context
- 将团队执行请求映射到具体成员动作

### 11.5 `weft-claw` product package 边界

product package 负责：

- feature 启用
- 默认模板选择
- 默认角色集选择
- devteam 入口与页面路由

## 十二、Feature Binding 与 Product Package 集成

`weft-claw-devteam-feature` 默认应组合以下 capability：

- `team.runtime`
- `team.taskboard`
- `workflow.template.devteam`
- `team.role.catalog`
- `team.delegate`
- `team.handoff`
- `team.context.shared`

并复用既有 capability：

- `agent.runtime`
- `workflow.orchestration`
- `memory.store`
- `channel.bridge`
- `ext.mcp`
- `ext.skills`

## 十三、角色模型

v1 默认角色：

- `planner`
- `implementer`
- `reviewer`
- `integrator`

角色不是 prompt 文本别名，而是一个包含以下内容的正式模型：

- workflow stage 权限
- delegate 权限
- skills 暴露范围
- MCP 工具可见范围
- task board 操作权限
- 输出物期望

## 十四、通信与任务板模型

v1 不做完全自由的 agent 对话网络，而采用受控通信模型：

1. 成员动作先写入 `team-task-board`
2. 必要的团队共享结论写入 `team.context.shared`
3. 对外通知通过 `channel.bridge` 输出摘要事件
4. 成员间 handoff 必须以显式记录存在，而不是只靠自然语言消息

建议 `team-task-board` 支持最小事件集：

- `task_created`
- `task_assigned`
- `task_started`
- `task_blocked`
- `handoff_requested`
- `handoff_accepted`
- `review_requested`
- `review_completed`
- `integration_started`
- `task_completed`
- `session_closed`

## 十五、对既有 package / primitive 的复用策略

- 复用 `delegate`
- 复用 `workflow.orchestration`
- 复用 `memory.store`
- 复用 `channel.bridge`
- 复用 `ext.mcp`
- 复用 `ext.skills`
- 复用 `agent.runtime`

## 十六、v1 / v2 范围拆分

### 16.1 v1 范围

v1 必须包含：

- `team-runtime`
- `team-task-board`
- `workflow-template-devteam`
- `weft-claw-devteam-feature`
- 四个固定默认角色
- 单 session 团队运行
- 显式 task board
- 显式 handoff
- 受控 workflow stage

### 16.2 v2 范围

v2 可以考虑，但不纳入 v1：

- 可配置角色目录外部化
- 多板块或多泳道 task board
- 与外部项目管理系统双向同步
- 更丰富的 review policy
- 跨 session 复用团队模板
- 更复杂的并行调度
- swarm 式自治探索

## 十七、建议实现顺序

1. 先落 `team-task-board`
2. 再落 `workflow-template-devteam`
3. 再落 `team-runtime`
4. 再落 `weft-claw-devteam-feature`
5. 最后做 product package 集成

## 十八、风险与约束

### 18.1 风险一，团队逻辑回流到 product package

控制方式：

- 强制以 `weft-claw-devteam-feature` 和新增 capability 为实现边界

### 18.2 风险二，`team-runtime` 吞掉 workflow 语义

控制方式：

- devteam 流程语义必须优先写在 `workflow-template-devteam`

### 18.3 风险三，task board 退化成 UI 状态容器

控制方式：

- task board 必须有正式对象模型、事件模型和持久化语义

### 18.4 风险四，角色模型过早复杂化

控制方式：

- v1 固定四角色
- 不做 swarm

## 十九、验收标准

以下标准满足后，才能认为 `weft-claw devteam v1` 设计达成并可进入实现：

1. 团队逻辑没有直接塞进 core
2. 团队逻辑没有直接塞进 `weft-claw` product package
3. 新增 package 和 capability 命名已 canonical 化
4. `weft-claw-devteam-feature` 可作为独立 feature 启用
5. 对象模型、运行边界、实现顺序已清晰可执行

## 二十、一句话结论

`weft-claw devteam` 在 v1 中不应被实现为“product package 内的一个多 agent 模式”，而应被定义为：

**由 `team-runtime`、`team-task-board`、`workflow-template-devteam` 和 `weft-claw-devteam-feature` 组成，并复用 `delegate`、`workflow.orchestration`、`memory.store`、`channel.bridge`、`ext.mcp`、`ext.skills`、`agent.runtime` 的 package-first 开发团队协作能力。**
