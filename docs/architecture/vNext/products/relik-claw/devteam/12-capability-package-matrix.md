# WEFT vNext 蓝图：Weft Claw DevTeam Capability 与 Package 矩阵

## 一、文档目的

这份文档是 `11-feature-design.md` 的配套矩阵，作用与 `../10-capability-package-matrix.md` 类似：把 devteam 相关 capability、package、feature 和 product package 责任压缩成一份可直接查阅的 canonical 表。

它回答的问题是：

- devteam v1 应新增哪些 capability
- 每个 capability 对应哪个 provider package、foundation package、feature package，还是 product package 责任
- 哪些能力复用现有 package，哪些必须新增 package
- 哪些 binding 应固定，哪些允许 mutable，哪些只能条件替换

## 二、使用规则

1. 本文遵循 `../09-product-blueprint.md` 和 `11-feature-design.md`
2. devteam 仍按 `provider -> foundation -> feature -> product package` 理解
3. 本文不把团队逻辑重新塞回 core 或 product package
4. 默认 provider 指 Weft Claw 默认装配选择，不表示 Core 全局硬编码

## 三、DevTeam 装配层次总览

| 层级 | 角色 | 典型包 | 是否面向用户直接暴露 |
| --- | --- | --- | --- |
| Capability Provider Packages | 提供专门能力实现 | `workflow-template-devteam` | 否 |
| Foundation Packages | 提供团队运行与任务板 | `team-runtime`, `team-task-board` | 否 |
| Reused Foundation / Providers | 复用现有执行与扩展能力 | `agent-runtime`, `workflow-orchestrator`, `memory-store`, `channel-core`, `mcp-client`, `skills-runtime`, `delegate` | 否 |
| Weft Claw Feature Package | 承载 devteam 产品语义 | `weft-claw-devteam-feature` | 间接 |
| Weft Claw Product Package | 提供 devteam 默认装配和入口 | `weft-claw`, `weft-claw-ui` | 是 |

## 四、DevTeam capability 到 package 层的 canonical 矩阵

| Capability | Provider Package | Foundation Package | Feature Package | Product Package 责任 | Weft Claw 默认 provider | Mutability | 说明 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `team.runtime` | `team-runtime` | `team-runtime` | `weft-claw-devteam-feature` | 启用 team session 入口与默认 team mode | `team-runtime` | 条件替换 | 团队运行中枢 |
| `team.taskboard` | `team-task-board` | `team-task-board` | `weft-claw-devteam-feature` | 暴露 task board UI / route | `team-task-board` | Mutable | 团队协作事实源 |
| `workflow.template.devteam` | `workflow-template-devteam` | `workflow-orchestrator` | `weft-claw-devteam-feature` | 选择默认 team workflow template | `workflow-template-devteam` | 条件替换 | 团队阶段模板 |
| `team.role.catalog` | `team-runtime` | `team-runtime` | `weft-claw-devteam-feature` | 选择默认 role set | `team-runtime` | 条件替换 | 角色目录 |
| `team.delegate` | `delegate` | `agent-runtime` | `weft-claw-devteam-feature` | 控制是否允许成员间委派 | `delegate` | Mutable | 复用既有 delegate 原语 |
| `team.handoff` | `team-task-board` | `team-task-board` | `weft-claw-devteam-feature` | 控制交接记录可见性 | `team-task-board` | Mutable | 交接状态与记录 |
| `team.context.shared` | `memory-store` + `team-runtime` | `memory-store` | `weft-claw-devteam-feature` | 控制共享上下文是否启用 | `memory-store` + `team-runtime` | Mutable | 团队共享上下文投影 |
| `agent.runtime` | `agent-runtime` | `agent-runtime` | `weft-claw-devteam-feature` | 控制默认成员执行器 | `agent-runtime` | 条件替换 | 团队成员执行基础 |
| `workflow.orchestration` | `workflow-orchestrator` | `workflow-orchestrator` | `weft-claw-devteam-feature` | 选择 devteam 的默认阶段执行器 | `workflow-orchestrator` | 条件替换 | 团队工作流引擎 |
| `memory.store` | `memory-store` | 无独立 foundation | `weft-claw-devteam-feature` | 控制是否启用共享记忆 | `memory-store` | Mutable | 共享摘要、决策、handoff notes |
| `channel.bridge` | `channel-core` | `channel-core` | `weft-claw-devteam-feature` | 控制是否暴露外部通知桥 | `channel-core` | Mutable | 可选 team 通知桥 |
| `ext.mcp` | `mcp-client` | 无独立 foundation | `weft-claw-devteam-feature` | 控制成员的 MCP 接入面 | `mcp-client` | Mutable | member profile 的外部工具面 |
| `ext.skills` | `skills-runtime` | 无独立 foundation | `weft-claw-devteam-feature` | 控制成员的 skills 暴露 | `skills-runtime` | Mutable | member profile 的技能面 |

## 五、Feature Package 视角的能力归属矩阵

| Feature Package | 主要组合的 capability | 默认依赖的 package | 自己不应吸收的职责 |
| --- | --- | --- | --- |
| `weft-claw-devteam-feature` | `team.runtime`, `team.taskboard`, `workflow.template.devteam`, `team.role.catalog`, `team.delegate`, `team.handoff`, `team.context.shared`, `agent.runtime`, `workflow.orchestration`, `memory.store`, `ext.mcp`, `ext.skills` | `team-runtime`, `team-task-board`, `workflow-template-devteam`, `agent-runtime`, `workflow-orchestrator`, `memory-store`, `delegate`, `mcp-client`, `skills-runtime` | 不应把 task board、workflow engine、member runtime、memory backend、MCP/skills 实现细节回收进 feature 包 |

## 六、默认 provider 与 mutability 策略矩阵

| Capability | 默认 provider | 默认策略 | 可否被 user/profile override | override 约束 |
| --- | --- | --- | --- | --- |
| `team.runtime` | `team-runtime` | 条件固定 | 可以，但不应自由漂移 | 必须经过 propose / resolve / verify / activate |
| `team.taskboard` | `team-task-board` | 默认启用 | 可以 | 可作为标准 mutable binding |
| `workflow.template.devteam` | `workflow-template-devteam` | 条件固定 | 可以，但受团队语义约束 | 不应破坏 devteam 默认阶段模型 |
| `team.role.catalog` | `team-runtime` | 条件固定 | 可以，但受角色模型约束 | 不应破坏默认角色边界 |
| `team.delegate` | `delegate` | 默认启用 | 可以 | 应受 profile / policy 约束 |
| `team.handoff` | `team-task-board` | 默认启用 | 可以 | 应与 task board 模型兼容 |
| `team.context.shared` | `memory-store` + `team-runtime` | 默认启用 | 可以 | 不应退化为无限制全局 memory |
| `agent.runtime` | `agent-runtime` | 条件固定 | 可以 | 保持 team 成员执行基础稳定 |
| `workflow.orchestration` | `workflow-orchestrator` | 条件固定 | 可以 | 受模板与团队流程约束 |
| `memory.store` | `memory-store` | 默认启用 | 可以 | 受 profile / policy 约束 |
| `channel.bridge` | `channel-core` | 默认可选 | 可以 | 仅作为可选外部桥 |
| `ext.mcp` | `mcp-client` | 默认可选 | 可以 | 可按角色关闭或替换 |
| `ext.skills` | `skills-runtime` | 默认可选 | 可以 | 可按角色关闭或替换 |

## 七、Product Package 边界矩阵

| 领域 | 属于 product package | 不属于 product package | 原因 |
| --- | --- | --- | --- |
| 产品身份 | devteam feature 名称、默认模式、入口说明 | team runtime 状态机 | product package 负责产品默认值，不负责实现 |
| 默认装配 | 默认启用 `weft-claw-devteam-feature`、默认 template、默认 role set | task board 数据模型、member lifecycle | product package 只负责装配 |
| UI | devteam 页面入口、任务板 route、状态展示 | 任务推进逻辑、成员执行逻辑 | UI 是表面，不是执行引擎 |
| 配置 | 是否启用 devteam、默认 profile、默认成员数量等入口 | `delegate` 执行本体、workflow engine 本体 | product package 不应吞掉 foundation 责任 |

## 八、建议的 declaration / config / lock 同步指导

### 8.1 `package.toml`

建议增加：

- `team.runtime`
- `team.taskboard`
- `workflow.template.devteam`
- `team.role.catalog`
- `team.delegate`
- `team.handoff`
- `team.context.shared`

并增加 feature：

- `weft-claw-devteam-feature`

### 8.2 `config.toml`

建议增加：

- devteam feature 开关
- 默认 workflow template 选择
- 默认 role set 选择
- team 共享上下文策略开关
- 成员的 skills / MCP 可见性开关

### 8.3 `lock.toml`

应锁定：

- `team.runtime -> team-runtime`
- `team.taskboard -> team-task-board`
- `workflow.template.devteam -> workflow-template-devteam`
- 以及 devteam feature 最终实际装配结果

## 九、一句话结论

`weft-claw devteam` 的 canonical 矩阵应明确表达：

**新增 `team-runtime`、`team-task-board`、`workflow-template-devteam`、`weft-claw-devteam-feature`，并把团队协作能力以 package-first 方式挂接到既有 `agent.runtime`、`workflow.orchestration`、`memory.store`、`channel.bridge`、`ext.mcp`、`ext.skills` 上，而不是回收到 Core 或 product package。**
