# WEFT vNext 蓝图：Binding 与 Resolve in the Weft Claw Model

## 一、为什么需要重写这份 Binding 文档

旧版 Binding 文档主要基于较粗粒度的 capability 集合，例如：

- `agent.capability`
- `tool.registry`
- `memory.backend`
- `execution.capability`
- `communication.bridge`

这套表达适合早期样板，但已经不足以支撑 `weft-claw` 这种由多层 package 装配出来的产品实例。

根据新的 Weft Claw 蓝图，Binding 不再只是“某个能力指向某个 provider”这么简单，而必须同时支持：

- 更细粒度的 capability 命名
- feature package 作为中间组合层
- product package default、feature-level、runtime override 三层 binding
- 更明确的 fixed、mutable、conditional mutable 规则
- resolve 过程中对 feature 组合结果的正式纳入

因此，这份文档必须以 `../products/weft-claw/09-product-blueprint.md` 与 `../foundations/13-product-package-instance-model.md` 为准，重新定义 Binding 与 Resolve。

## 二、新模型中的 capability 集合

在新的蓝图里，product package 与 feature package 面向的是更具体、但仍可控的一组 capability：

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

这里有三条硬规则：

1. product package 声明 capability，不硬写底层 package 名称
2. Feature package 依赖 capability 或稳定 feature 组合，不直接回收底层 provider 细节
3. Provider package 声明自己提供哪些 capability，Core 负责最后绑定

这意味着 Binding 的输入已经不再是“几个粗 capability 的平铺列表”，而是一个同时包含 capability、feature 依赖、profile 约束和 override 的装配问题。

## 三、Binding 的角色

Binding 表示：

**某个 product package instance 当前 generation 中，某个 capability 最终由哪个 provider package 提供。**

在新模型里，Binding 不是孤立映射，而是产品装配结果的一部分。它需要同时回答：

- 这个 capability 是 product package 直接需要，还是由 feature 组合引入
- 当前 provider 是 product package 默认、feature 默认，还是 runtime override 后的结果
- 这个 provider 是固定的、可替换的，还是受约束可替换的

例如，一个典型的 Weft Claw 默认绑定集合会是：

- `agent.runtime -> agent-runtime`
- `prompt.system -> prompt-system`
- `workflow.orchestration -> workflow-orchestrator`
- `tool.runtime -> tool-runtime-core`
- `tool.shell -> tool-shell`
- `memory.store -> memory-store`
- `channel.bridge -> channel-core`
- `ext.mcp -> mcp-client`
- `ext.skills -> skills-runtime`
- `ui.surface -> weft-claw-ui`

如果启用了相应渠道或工具能力，还会继续扩展为：

- `tool.files -> tool-files`
- `tool.web -> tool-web`
- `tool.git -> tool-git`
- `channel.telegram -> channel-telegram`
- `channel.discord -> channel-discord`
- `channel.slack -> channel-slack`

## 四、Binding 为什么必须升级为三层

Weft Claw 不是“一个 product package 加几个默认插件”，而是一个由 capability provider、foundation package、feature package、product package 壳共同装配出来的产品。只保留单层 binding，会立即失去以下能力：

- feature package 的稳定默认组合
- product package 对产品默认行为的控制
- 用户或 profile 在运行时对 provider 的安全覆盖

因此，新模型中的 Binding 必须明确分成三层。

### 4.1 Product Package Default Binding

这一层定义产品级默认装配结果，也就是 Weft Claw 作为一个稳定 product package，默认希望各 capability 指向哪个 provider。

例如：

- `agent.runtime -> agent-runtime`
- `prompt.system -> prompt-system`
- `workflow.orchestration -> workflow-orchestrator`
- `tool.runtime -> tool-runtime-core`
- `memory.store -> memory-store`
- `channel.bridge -> channel-core`
- `ext.mcp -> mcp-client`
- `ext.skills -> skills-runtime`
- `ui.surface -> weft-claw-ui`

这一层体现的是产品默认身份，而不是用户当前设备上的最终运行状态。

### 4.2 Feature-level Binding

feature package 也可以带有自己的默认依赖与默认 provider 选择。这一层不是重复 product package default，而是把某个产品能力块的内部组合固定下来，避免 product package 直接管理所有底层细节。

例如：

- `weft-claw-chat-feature` 默认组合 `agent.runtime + prompt.system + memory.store`
- `weft-claw-channel-feature` 默认组合 `channel.bridge + selected channel adapters`
- `weft-claw-automation-feature` 默认组合 `workflow.orchestration + tool.runtime + selected tool providers`
- `weft-claw-extensions-feature` 默认组合 `ext.mcp + ext.skills`

Feature-level binding 的核心价值是：

- 让 feature package 成为稳定产品能力块
- 避免 product package 直接管理过多细粒度 provider
- 让 resolve 可以先合成 feature，再收敛到底层 capability 绑定

### 4.3 Runtime Override

最后一层是用户、profile 或运行环境在 instance `config.toml` 中施加的覆盖规则。

典型场景包括：

- 把 `memory.store` 切换到另一个 memory provider
- 替换 `tool.shell` 的实现
- 禁用某个 channel adapter
- 关闭 `ext.mcp` 或 `ext.skills`
- 在允许范围内替换 `workflow.orchestration`

这一层的重点不是“任何东西都能改”，而是：

- override 必须落在 declaration 允许的边界内
- override 仍然要经过 resolve、verify、activate
- override 不得绕开 fixed binding

## 五、Fixed、Mutable、Conditional Mutable 的新规则

旧版文档里只把 binding 分成 fixed 与 mutable，两者过于粗糙。根据 Weft Claw 新蓝图，binding 规则必须扩展为三类。

### 5.1 Fixed Binding

Fixed binding 指默认不允许被 runtime override 或自动 rebinding 改写的部分。

在 Weft Claw 中，通常应固定：

- `ui.surface`
- 与产品身份强绑定的核心 feature package
- Weft Claw 默认产品入口

它们之所以固定，不是因为技术上不能替换，而是因为一旦漂移，用户看到的可能就不再是同一个产品。

### 5.2 Mutable Binding

Mutable binding 指允许在声明约束和 policy 约束内替换 provider 的部分。

通常允许 mutable：

- `memory.store`
- `tool.shell`
- `tool.files`
- `tool.web`
- `tool.git`
- `channel.telegram`
- `channel.discord`
- `channel.slack`
- `ext.mcp`
- `ext.skills`

这些能力天然具有 provider 或 adapter 属性，允许替换才符合 vNext 的可组合目标。

### 5.3 Conditional Mutable Binding

还有一类 capability 不能简单归为 fixed 或普通 mutable，它们更接近产品行为中枢。

Weft Claw 蓝图明确这类能力应视为条件可替换：

- `prompt.system`
- `agent.runtime`
- `workflow.orchestration`

条件可替换的意思是：

- 不永久写死
- 但必须满足更严格的 allowed provider、profile、policy 与 verification 条件
- 不能像普通工具 provider 一样随意漂移

换句话说，fixed、mutable、conditional mutable 的区别，不只是“能不能换”，更是“替换的验证门槛和系统风险级别不同”。

## 六、Feature package 在 Binding 中的地位

Weft Claw 模型里，feature package 不是装饰层，而是正式参与 resolve 的装配层。

它们的作用不是直接替代 capability binding，而是：

1. 把一组底层 capability 组合成稳定产品能力块
2. 提供 feature 默认依赖与默认 provider 选择
3. 把 product package 级配置压力从零散 provider 细节上移开

因此，Binding 过程必须承认两类依赖来源：

- product package 直接声明的 required capabilities
- 启用的 feature package 所组合出来的 additional capabilities

例如，当 `weft-claw-automation-feature` 被启用时，resolve 不能只看 `tool.runtime`，还必须把该 feature 要求的具体 tool provider 能力一起纳入候选图中，例如：

- `tool.shell`
- `tool.files`
- `tool.web`
- `tool.git`

同理，当 `weft-claw-channel-feature` 被启用时，resolve 不仅要绑定 `channel.bridge`，还必须根据 feature 选择和 config 状态决定是否引入：

- `channel.telegram`
- `channel.discord`
- `channel.slack`

这就是“feature composition 必须进入 resolve 语义”的含义。

## 七、Resolve 的职责

Resolve 是 Core 的内部过程，用于把：

- product package declaration
- instance config
- enabled feature packages
- provider package 集合
- profile / policy
- runtime overrides

解析成一个 candidate runtime graph 与 candidate lock。

在新模型里，Resolve 不再只是“给 capability 找 provider”，而是“在 feature composition 之后，收敛出可验证的产品装配结果”。

## 八、Resolve 的标准流程

为与 Weft Claw 蓝图保持一致，Resolve 至少要执行以下步骤。

### 8.1 读取 product package declaration

Core 先读取 product package 的产品意图，包括：

- required capabilities
- product package default bindings
- fixed、mutable、conditional mutable 规则
- 默认启用的 feature packages
- allowed provider 范围
- validation requirements

这一步回答的是：“这个 product package 作为产品，默认想装成什么样。”

### 8.2 读取 instance config 与 runtime state

Core 再读取运行时配置，包括：

- 当前 profile
- workspace
- feature 开关
- runtime override 请求
- 渠道、扩展、工具等启用状态

这一步回答的是：“当前设备和当前用户，想在什么边界内运行它。”

### 8.3 展开 feature composition

这是旧版文档缺失、但新模型必须补上的步骤。

Core 需要把 product package 默认启用并由 instance 覆盖后的 feature package 展开成实际依赖图，明确：

- 每个 feature 需要哪些 capability
- 哪些 capability 是 feature 内部默认引入的
- 哪些 adapter 或 provider 是 feature 可选项
- 哪些 feature 冲突、互斥或依赖其它 feature

这一步之后，Core 才能得到完整的 candidate capability set，而不是只拿到 product package 顶层写出来的粗列表。

### 8.4 收集 provider packages

Core 收集所有可用 provider package，并读取它们声明的：

- provides 哪些 capability
- requires 哪些 capability
- 自身运行形态与兼容信息
- source、version、digest 与验证元数据

这一步的目标不是立刻决定最终 provider，而是建立候选池。

### 8.5 应用三层 binding 规则

Core 按固定优先级叠加三层 binding：

1. feature-level defaults
2. product package default bindings
3. runtime overrides

实现上，优先级应理解为“后层覆盖前层，但不得越过上层约束”：

- feature-level binding 先给出 feature 内部默认组合
- product package default binding 作为产品级默认，覆盖或收敛 feature 内部默认
- runtime override 只在 declaration 与 policy 允许范围内覆盖前两层

因此最终 binding 不是某一层单独决定的，而是三层合成后的结果。

### 8.6 应用 fixed、mutable、conditional mutable 规则

Core 必须检查当前 binding 是否违反 declaration 里的变更边界：

- fixed binding 不得被 override
- mutable binding 只能落在 allowed provider 集内
- conditional mutable binding 需要额外满足 profile、policy 或验证门槛

如果某个 override 想把 `ui.surface` 换掉，resolve 必须直接拒绝。

如果某个 override 想替换 `workflow.orchestration`，resolve 不能像替换 `tool.web` 一样直接接受，而必须检查它是否满足更严格条件。

### 8.7 应用 profile / policy

profile 与 policy 继续对最终 candidate graph 施加约束，例如：

- 某些 profile 禁止 shell provider
- 某些 profile 禁止某类 channel adapter
- trusted 模式允许更多 override，自由度更高
- safe 模式要求部分 provider 退回默认值或被禁用

这一步说明一个关键事实：

**binding 合法，不等于运行策略合法。**

### 8.8 解析 legacy capability aliases

为兼容旧蓝图与历史文档，resolve 可以接受旧 capability 名称作为输入别名，但输出语义必须收敛到新命名。

例如可以接受以下 legacy alias：

- `agent.capability` -> `agent.runtime`
- `tool.registry` -> `tool.runtime`
- `memory.backend` -> `memory.store`
- `execution.capability` -> `workflow.orchestration` 或 `tool.shell`，具体必须按上下文判定，禁止无条件混用
- `communication.bridge` -> `channel.bridge`

这里必须强调两点：

1. legacy alias 只应用于兼容输入、迁移文档或旧 declaration
2. 新 generation 的输出、lock、binding 摘要与文档命名必须统一使用新 capability 名称

也就是说，系统可以识别旧名字，但不能继续把旧名字当正式输出模型。

### 8.9 生成 candidate bindings 与 candidate lock

在所有约束收敛之后，Core 生成：

- capability -> provider 的最终 candidate bindings
- feature package 与底层 package 的最终解析结果
- package source、version、digest、signature 等 lock 所需元数据
- 当前 generation 的装配快照

这一步生成的不是“用户偏好”，而是候选事实。

### 8.10 交给 verify

Resolve 的终点不是直接激活，而是把 candidate graph 与 candidate lock 交给 verify。

只有验证通过后，candidate generation 才能进入 activate。

## 九、Binding 与 Lock 的关系

在 Weft Claw 模型里，Binding 不是只存在于 declaration 中的抽象规则，也不是只存在于 config 中的用户偏好。真正运行的 binding 必须被锁进 instance `lock.toml`。

Lock 至少应记录：

- 实际参与装配的 package 列表
- feature package 与底层 provider 的最终解析结果
- capability -> provider 的最终绑定快照
- 对应 package 的 digest、signature、source metadata

因此，Binding 在三层文件中的角色分别是：

- declaration 定义默认 binding 与可变边界
- config 定义 feature 开关与 runtime override
- lock 记录最终 resolve 结果

## 十、为什么 Core 仍然必须是最终裁决者

三层 binding、feature composition 与 provider 可替换性成立的前提，是 Core 必须是最终解析与裁决中心。

Core 负责：

- capability registry
- feature composition 展开
- binding 优先级合成
- fixed / mutable / conditional mutable 检查
- profile / policy 约束
- candidate lock 生成
- verify 前的最终收敛

因此 Binding 不是 package 之间自行协商，也不是 product package 直接硬编码 provider 细节，而是：

**Product Package 声明目标，feature 组织组合，provider 提供实现，Core 统一解析并裁决。**

## 十一、设计结论

在新的 Weft Claw 模型里，Binding 与 Resolve 必须满足以下结论：

- capability 命名统一升级到 `agent.runtime`、`prompt.system`、`workflow.orchestration`、`tool.runtime`、`tool.shell`、`memory.store`、`channel.bridge`、`ext.mcp`、`ext.skills` 等新集合
- Binding 明确分成 product package default、feature-level、runtime override 三层
- fixed、mutable、conditional mutable 三类规则必须分开定义
- resolve 必须正式包含 feature composition，而不是只解析顶层 capability
- legacy capability 名称只允许作为输入兼容别名，正式输出必须统一到新命名

一句话总结：

**在 Weft Claw 模型里，Binding 不再只是 capability 到 provider 的平面映射，而是 product package、feature、runtime 三层规则共同收敛出的产品装配结果。**
