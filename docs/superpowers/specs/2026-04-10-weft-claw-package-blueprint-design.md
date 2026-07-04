# WEFT vNext 设计稿：Weft Claw 包化重构与 Product Package 装配蓝图

## 1. 背景与目标

当前 `WEFT-plug` 已经把 `weft-claw` 作为首个 Product Package 跑通了最小闭环，但现有蓝图里的早期描述仍偏“单一产品壳 + 少数默认 package”的描述，不足以承接参考产品目录里那种更完整的产品能力面。

本设计的目标不是立刻实现全部迁移，而是先把 vNext 蓝图更新到一个更稳定、可扩展、可装配的模型：

- 先把参考产品目录里的核心能力重新理解为一组可复用 package
- 让这些 package 通过 capability / binding / feature package 组织起来
- 最终由 `weft-claw` product package 作为中等厚度产品壳完成装配
- 为后续把更多 package 组合成更多 product package 留出清晰路径

本设计明确采用“能拆就拆，但只在边界明确、复用价值真实时才拆”的原则。`mcp` 与 `skills` 需要可插拔，但不做过度分层；其余如 agent、workflow、channel、tool、memory 等则尽量拆清。

## 2. 设计结论总览

新的 `weft-claw` 蓝图采用四层结构：

1. **Capability Provider Packages**：最底层原子能力实现包
2. **Foundation Packages**：把一类原子能力组织成稳定能力面
3. **Weft Claw Feature Packages**：Weft Claw 产品特性组合层
4. **Weft Claw Product Package**：产品壳与默认装配层

这一模型的核心原则是：

- Product Package 面向 capability，不面向具体实现细节
- Package 只负责提供实现，不承载最终产品身份
- Feature package 负责把多个 package 组合成产品功能块
- `weft-claw` product package 只保留默认产品装配、默认策略与对外入口

## 3. 拆分原则

### 3.1 默认原则

默认优先拆分，但必须满足以下条件之一：

- 该部分具有明确的 capability 边界
- 该部分未来可能被其他产品复用
- 该部分需要被单独替换、升级或禁用
- 该部分与其他能力存在天然 provider / adapter 关系

如果某一能力只是内部实现细节、缺乏独立复用价值、或者拆分只会增加复杂度，则不强制拆分。

### 3.2 保持简单的部分

以下两类能力保持“单独 package，但不过度多层化”：

- `mcp-client`
- `skills-runtime`

它们都保持可插拔与可替换，但不再进一步拆成 `core + adapter + orchestrator` 多层结构。

## 4. 新的 Weft Claw 包体系

### 4.1 Agent / Prompt / Workflow 组

- `agent-runtime`
  - 负责 agent turn loop、provider interaction、tool dispatch 框架
  - 不直接持有具体 prompt 内容与产品策略
- `prompt-system`
  - 负责 system prompt 组装、persona、mode、policy prompt 组合
  - 作为独立 package 暴露，允许被替换或按 product package 覆盖默认实现
- `workflow-orchestrator`
  - 负责任务流、模式切换、多阶段执行与产品行为编排
  - 与 `agent-runtime` 解耦，避免把所有产品行为都塞进 agent loop

### 4.2 Tool Runtime 组

- `tool-runtime-core`
  - 提供统一工具能力面、工具注册模型、工具调用协议
- `tool-shell`
- `tool-files`
- `tool-web`
- `tool-git`
- 未来可扩展：`tool-browser`、`tool-jira`、`tool-notion` 等

这里采用两层设计：底层统一协议 + 各具体工具 provider package。这样 Weft Claw 可按 profile / feature 决定装哪些工具包，而不是把全部工具固定打进一个大包里。

### 4.3 Memory / Session 组

- `memory-store`
  - 提供记忆读写、检索、摘要、后端切换等能力
- `session-state`
  - 可选；如果会话状态管理边界足够稳定，则独立成 package
  - 若第一阶段边界不稳，可先保留在 feature/product package 层，不强制独立

### 4.4 Channel 组

- `channel-core`
  - 负责统一消息桥、接入抽象、路由与会话映射
- `channel-telegram`
- `channel-discord`
- `channel-slack`
- 后续可扩展：`channel-whatsapp`、`channel-matrix`、`channel-signal` 等

这里采用两层结构：一个统一桥接核心，加若干渠道 adapter package。这样可以只启用某一个渠道，也可以多渠道并存。

### 4.5 Extension 组

- `mcp-client`
  - 负责配置、连接、调用外部 MCP 服务，并把能力暴露给 agent/workflow
- `skills-runtime`
  - 负责读取 skill 文件、解析元数据、装载 skill 能力并对外暴露

这两者都属于扩展能力，但保持单层 package，避免不必要的复杂化。

## 5. Weft Claw Feature Packages

在底层 package 之上，增加少量 Weft Claw 专属 feature packages，用来承载产品语义，而不是把所有产品逻辑直接塞回 product package。

建议至少存在以下 feature 层：

- `weft-claw-chat-feature`
  - 组合 `agent-runtime`、`prompt-system`、`memory-store`
- `weft-claw-channel-feature`
  - 组合 `channel-core` 与具体 channel adapter
- `weft-claw-automation-feature`
  - 组合 `workflow-orchestrator` 与工具运行面
- `weft-claw-extensions-feature`
  - 组合 `mcp-client` 与 `skills-runtime`

feature package 的职责是：

- 提供更接近产品功能的 capability 组合
- 把一组底层 package 封装成“用户能感知的能力块”
- 为 product package 提供更稳定的装配单位

## 6. Weft Claw Product Package 的职责

`weft-claw` product package 被定义为“中等厚度产品壳”，只保留以下内容：

1. 默认启用哪些 feature package
2. 默认 bindings 指向哪些 provider
3. 默认 workflow / prompt 组合的选择权
4. profile、workspace、feature 开关等运行时配置
5. UI 入口、health/run/update/rollback 等 product 级接口

以下内容必须尽量从 product package 中移出：

- agent loop 实现细节
- prompt 具体内容与 persona 内部结构
- shell / file / web / git 等工具执行实现
- channel adapter 细节
- MCP 协议处理逻辑
- skills 读取与装载逻辑

最终的 `weft-claw` product package 更像一份“产品装配说明”，而不是一个大系统本体。

## 7. Capability 与 Binding 模型更新

### 7.1 能力面原则

每个 package 应首先回答：

- 它提供什么 capability
- 它依赖哪些 capability
- 如果替换掉它，product package 是否仍可装配

建议将现有较粗的能力面扩展成更细粒度但仍可控的 capability 集合，例如：

- `agent.runtime`
- `prompt.system`
- `workflow.orchestration`
- `tool.runtime`
- `tool.shell`
- `tool.files`
- `tool.web`
- `tool.git`
- `memory.store`
- `channel.bridge`
- `channel.telegram`
- `channel.discord`
- `channel.slack`
- `ext.mcp`
- `ext.skills`
- `ui.surface`

其中：

- Foundation package 可暴露统一 capability 面
- Adapter package 暴露更具体 provider capability
- Feature package 可以依赖多个底层 capability，但不必一定新增过多“伪 capability 名称”

### 7.2 三层 Binding

新的 binding 分为三层：

1. **Product Default Binding**
   - Weft Claw 产品默认选用的 provider
2. **Feature-level Binding**
   - Feature package 自己声明所需能力与默认依赖
3. **User / Runtime Override**
   - 用户或 profile 在 instance `config.toml` 中覆盖 provider

这一设计确保“几乎每个部分都有灵活性”，但灵活的是 provider 选择与 feature 组合，而不是让 product package 结构本身失控。

## 8. 与当前蓝图的主要差异

当前 `09-weft-claw-product-blueprint.md` 已经承接新的 product package 语义，旧描述里曾把该产品近似表述为“少数默认 package + 产品壳”的模型，例如：

- `agent.capability -> agent-core`
- `tool.registry -> skills`
- `memory.backend -> memory`
- `execution.capability -> command-tools`
- `ui.surface -> weft-claw-ui`
- `communication.bridge -> channels`

新的蓝图需要把它提升为：

- 不再只强调少数默认 package
- 明确区分 provider packages、foundation packages、feature packages、product package 壳
- 明确 `prompt/system` 为独立 package
- 明确 `channel` 采用 `core + adapters` 模型
- 明确 `tool` 采用 `runtime core + adapters` 模型
- 明确 `mcp-client` 与 `skills-runtime` 为独立扩展 package
- 明确 product package 保留默认 workflow / prompt 组合与 feature 开关，而不是承载所有实现逻辑

## 9. 需要更新的蓝图文档

本设计对应的文档更新范围应至少包括：

- `docs/architecture/vNext/09-weft-claw-product-blueprint.md`
  - 从早期产品壳样板升级到“weft-claw 是首个多 package 装配型产品 Product Package 样板”
- `docs/architecture/vNext/03-package-model.md`
  - 增加 provider package / foundation package / feature package 的层次说明
- `docs/architecture/vNext/05-binding-and-resolution.md`
  - 增加三层 binding 与更细粒度 capability 映射原则
- 必要时补充到：
  - `02-app-model.md`
  - `04-declaration-config-lock.md`

## 10. 不在本设计中立即解决的内容

以下内容属于后续实现与迭代范围，本设计不强行在第一版蓝图中全部定死：

- 每个 capability 的最终命名细节
- `session-state` 是否独立成 package 的最终结论
- 每个渠道 adapter 的配置格式
- MCP transport 的全部协议细节
- skills 文件布局的最终规范
- feature package 是否需要独立 manifest 形态，还是先用 package + metadata 近似表达

这些都可以在后续 implementation plan 里进一步收敛，但当前设计已经明确了结构方向与责任边界。

## 11. 验收标准

本次蓝图更新完成后，应满足以下标准：

1. `weft-claw` 不再被描述为少数固定 package 的简单组合，而是多 package 装配型产品 Product Package
2. `agent / prompt-system / workflow / tool / channel / memory / mcp / skills` 的边界被清晰定义
3. `mcp` 与 `skills` 保持可插拔，但不过度分层
4. `weft-claw` product package 被明确限制为中等厚度产品壳
5. blueprint 文档能直接指导后续 implementation plan，把“多个 package 变成产品”作为主线而不是补充说明

## 12. 一句话结论

Weft Claw 在 WEFT vNext 中不应再被视为“一个较大的产品壳配几个默认插件”，而应被重定义为：

**一个由可替换 capability packages、少量 feature packages 和中等厚度 product package 壳共同装配出来的产品系统。**
