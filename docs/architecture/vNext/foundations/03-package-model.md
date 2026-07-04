# WEFT vNext 蓝图：Package 模型与实现形态

## 一、为什么保留 Package

虽然普通用户不应直接理解全部 Package 图，但源码层必须统一收敛到 Package。原因是：

- 产品默认装配需要由多个内部实现组合而成
- AI 需要能够替换、升级和比较内部实现
- Core 需要精确锁定和验证这些实现
- 同一类能力可能有多种不同实现形态

因此，Package 不再只是内部实现单位，而是统一的源码发布单位。

## 二、Package 的定义

Package 是一个可解析、可验证、可绑定的能力实现单元。

它首先回答三件事：

- 它提供什么 capability
- 它依赖哪些 capability
- 它是否适合作为默认实现、基础组合层、产品 feature 组合层，或产品级默认装配层

所以在 vNext 里，Package 的核心不再是“它属于哪种抽象运行层”，而是：

- 它提供什么能力
- 它如何被 Core 装载
- 它在装配层次里扮演什么角色

## 三、Package 不再只有“一个扁平集合”

旧的理解方式容易把所有 package 都看成同一层，只是 capability 名称不同。这在最小闭环里够用，但在 `weft-claw` 这种多 package 装配型产品中会马上变得含糊。

例如下面三种包，如果都只叫“package”，系统就很难表达它们的差异：

- `tool-shell`，它是具体 provider
- `tool-runtime-core`，它是统一工具能力面的 foundation
- `weft-claw-automation-feature`，它是 weft-claw 的产品功能组合块

它们都属于 package，但不是同一层角色。因此 vNext 需要明确 taxonomy，而不是继续维持单层心智模型。

## 四、vNext 的四类 Package Taxonomy

在收敛后的蓝图里，Package 明确分成四类：

1. **Provider Packages**
2. **Foundation Packages**
3. **Feature Packages**
4. **Product Packages**

这四类都是 package，只是职责不同。

### 4.1 Provider Packages

Provider package 负责提供原子能力实现。它们通常直接声明一个或少数几个明确 capability，例如：

- `tool-shell`
- `tool-files`
- `tool-web`
- `tool-git`
- `channel-telegram`
- `channel-discord`
- `channel-slack`
- `memory-store`
- `mcp-client`
- `skills-runtime`

这类 package 的目标是：

- 提供清晰、有限、可替换的 capability 实现
- 不承担完整产品身份
- 不决定整个 product instance 应该怎样运转

一个典型例子是：

- `tool-shell` 提供 `tool.shell`
- `channel-telegram` 提供 `channel.telegram`
- `memory-store` 提供 `memory.store`

### 4.2 Foundation Packages

Foundation package 负责把一类 provider 组织成稳定能力面。它们不是最终产品，但会形成中间层的统一协议、统一状态模型和统一调用面，例如：

- `agent-runtime`
- `prompt-system`
- `workflow-orchestrator`
- `tool-runtime-core`
- `channel-core`

这类 package 的目标是：

- 提供较稳定、较可复用的中间能力面
- 统一一组相关 provider 的调用方式
- 避免把产品行为直接压进 product package 或 Core

典型例子是：

- `tool-runtime-core` 提供 `tool.runtime`
- `channel-core` 提供 `channel.bridge`
- `prompt-system` 提供 `prompt.system`

### 4.3 Feature Packages

Feature package 负责把多个底层 capability 组合成用户能感知的产品能力块。它们仍是 package，但已经带有明确产品语义，例如：

- `weft-claw-chat-feature`
- `weft-claw-channel-feature`
- `weft-claw-automation-feature`
- `weft-claw-extensions-feature`

这类 package 的目标是：

- 承载产品语义，而不是原子能力实现
- 把多个 foundation/provider 组合成稳定功能块
- 避免产品逻辑重新堆回 product package

例如：

- `weft-claw-chat-feature` 可以组合 `agent-runtime`、`prompt-system`、`memory-store`
- `weft-claw-automation-feature` 可以组合 `workflow-orchestrator`、`tool-runtime-core` 和若干工具 provider

### 4.4 Product Packages

Product package 用来取代旧模型里的源码层 App。

它的职责是：

- 表达产品身份
- 声明默认启用哪些 feature packages
- 声明默认 binding 策略
- 定义 validation / upgrade policy
- 保持产品默认装配稳定

它不负责：

- 当前用户配置
- 当前 workspace / secret / profile
- 当前 lock 与 generation 状态

例如：

- `weft-claw` 作为 product package，默认启用 `weft-claw-chat-feature`、`weft-claw-extensions-feature`、`weft-claw-channel-feature`

## 五、四类 Package 的关系

三类 package 不是彼此替代，而是形成明确装配层次：

- Provider package 提供原子能力实现
- Foundation package 提供中间统一能力面
- Feature package 提供产品语义组合块
- Product package 再选择默认启用哪些 feature，并决定默认 binding

以 `weft-claw` 为例，可以形成下面这种具体关系：

1. `tool-shell`、`tool-files`、`tool-web`、`tool-git` 先提供原子工具能力
2. `tool-runtime-core` 统一工具注册、调用协议和结果表示
3. `weft-claw-automation-feature` 组合 `tool-runtime-core` 与若干工具 provider
4. `weft-claw` product package 决定默认启用 `weft-claw-automation-feature`

这样 Product Package 面向 capability 与 feature，而不是直接管理所有底层实现细节。

## 六、MCP 与 Skills 的特殊规则

`mcp` 与 `skills` 需要保持可插拔，但不做过度分层。

在扩展后的蓝图里，它们直接作为两个独立 provider package 存在：

- `mcp-client`
- `skills-runtime`

它们保持单层 package 的原因是：

- 它们确实有独立边界
- 它们确实需要可替换、可禁用、可验证
- 但目前没有足够证据表明继续拆成 `core + adapter + orchestrator` 会带来真实收益

因此，vNext 对它们的约束非常明确：

- 保留独立 package 身份
- 保留独立 capability 边界
- 不在第一阶段继续细拆多层

这意味着：

- `mcp-client` 可以直接提供 `ext.mcp`
- `skills-runtime` 可以直接提供 `ext.skills`

而不需要再人为引入更多中间包。

## 七、Package 的实现路线仍是双轨，但不再等同于 taxonomy

Package 的 taxonomy 说的是职责层次，WASM / Native / Service 说的是实现形态，这两件事不能混为一谈。

也就是说：

- Provider、Foundation、Feature、Product 都可以是 WASM package
- 某些 provider 也可能更适合 Service runtime
- 只有少数必须深宿主集成的部分才应采用 Native

### 7.1 WASM Package

WASM 仍是默认主路线，尤其适合：

- 默认扩展
- 跨平台能力
- AI 自动生成和修改
- 编排逻辑
- 监管逻辑
- 保活与恢复逻辑

WASM package 的关键不是“直接控制所有宿主能力”，而是：

- 通过 Core 提供的 capability 调用接口访问能力
- 通过 binding 组合其他能力
- 作为大多数 provider、foundation、feature 的默认实现形态

### 7.2 Service Package / Managed Runtime

某些 package 更适合通过服务运行时落地，例如：

- 常驻连接型 channel adapter
- 需要进程生命周期管理的外部桥接能力
- 长连接或高状态性的 provider

Service 不是新的 taxonomy，而是 package 的另一种实现方式。

### 7.3 Native Package / Native Plugin

Native 是辅助路线，不是默认路线。它适合：

- 深系统集成
- 平台专属 API
- 高性能本地能力
- 硬件与特殊工具链对接
- WASM 或 Service 难以覆盖的宿主能力

Native 的职责是补足 WASM 与 Service，而不是取代它们。

## 八、为什么不是“全走 Native”

虽然 Native 能力更强，但如果让它成为默认主路线，会马上带来更高风险：

- 平台差异更重
- 调试更复杂
- AI 自动生成难度更高
- 整体架构更容易失控

因此 vNext 的原则是：

- 先问能否用 WASM 或 Service 实现
- 不够时再引入 Native

## 九、为什么不是“全走 WASM”

同样，如果完全依赖 WASM，也会出现问题：

- 某些深系统能力无法覆盖
- 宿主集成迟早会不够用
- 最终会不断逼迫 Core 直接扩大量能力接口

所以 vNext 采用的是“taxonomy 分层 + 实现形态双轨”的组合模型：

- 按 Provider / Foundation / Feature 分清职责
- 用 WASM 解决大多数扩展
- 用 Service 或 Native 补足少数场景

## 十、Package 的基本信息

一个 Package 至少应包含：

- name
- version
- source
- digest / content hash
- description
- package kind，明确它是 provider、foundation、feature 还是 product
- 提供的 capability
- 声明需要的 capability
- compatibility 信息
- validation hooks

这些信息足以支撑：

- Lock
- Binding
- Resolve
- Verify
- Rollback

其中 `package kind` 的意义不是增加花哨元数据，而是让声明、解析、锁定和迁移时不再含糊。

## 十一、Package 与 Product Package 的关系

Product Package 不直接关心每个 package 的内部实现细节，但必须关心它装配的是哪一层 package。

Product Package 主要关心：

- 哪些 feature package 默认启用
- 这些 feature 依赖哪些 foundation / provider capability
- 哪个 capability 当前绑定到哪个 provider
- 哪些 binding 可替换，哪些必须固定
- 改动后是否能通过验证

也就是说：

- Product Package 面向 capability 与 feature
- Package 面向实现与组合
- Core 负责把两者绑定起来

## 十二、Package 与 Core 的关系

Core 装载 Package，并通过 capability registry 维护它们的可用性与绑定关系。

在新的 taxonomy 里，Core 需要同时理解三件事：

- 哪些 package 是 provider，负责真正提供 capability
- 哪些 package 是 foundation，负责形成统一能力面
- 哪些 package 是 feature，负责形成产品级组合块

Core 不直接承担产品语义，但必须能正确 resolve 这些层次关系。

## 十三、从当前单层 package index 迁移到新模型

这是本次蓝图更新必须写清楚的过渡点。

当前实现里，`packages/index.toml` 仍然更接近单层 package source 索引。它能表达“有哪些 package 可以装”，但还不足以明确表达“这些 package 在装配层次里扮演什么角色”。

迁移目标不是一次性推翻现有 index，而是在保持兼容的前提下，把它逐步升级为能表达 taxonomy 的索引模型。

### 13.1 当前模型的特点

当前单层模型大致表达的是：

- 一个 package 对应一个 source entry
- package 可声明自己提供哪些 capability
- resolve 时直接从 package 集合里选择 provider

这个模型在简单场景下可工作，但遇到以下情况就会含糊：

- `tool-shell` 和 `tool-runtime-core` 是否同层
- `weft-claw-chat-feature` 是不是也应和普通 provider 一样被看待
- `mcp-client`、`skills-runtime` 是否需要再塞进别的统一包里

### 13.2 迁移后的表达原则

迁移后，index 至少应能表达：

- package kind，是 provider、foundation 还是 feature
- package 提供哪些 capability
- package 依赖哪些 capability
- package 是否是某个 product package 默认装配的一部分，或只作为可选实现存在

这里的关键不是立刻改变所有文件格式，而是先让蓝图把语义定死：

- 单层索引仍可存在
- 但索引里的 entry 语义不能再是“所有 package 都同层”

### 13.3 建议的过渡方式

第一阶段建议采用兼容式迁移：

1. 保留当前 `packages/index.toml` 作为统一入口
2. 为每个 package entry 增加可选 taxonomy 元数据，例如 `kind = "provider" | "foundation" | "feature"`
3. 继续允许旧 entry 没有 `kind`，默认按 provider 解释，以保证兼容
4. 在新的 package / instance / lock 语义里优先使用显式 `kind`
5. 后续再逐步让 feature package 和 foundation package 的声明更清晰

这样迁移不会阻断当前最小闭环，但可以让系统从“单层索引”平滑过渡到“分层 package 模型”。

### 13.4 一个具体迁移例子

在旧心智里，下面这些包容易被并排看待：

- `skills`
- `memory`
- `channels`
- `command-tools`
- `agent-core`

在新心智里，应逐步重写为更明确的结构，例如：

- `agent-runtime`，`kind = "foundation"`
- `prompt-system`，`kind = "foundation"`
- `workflow-orchestrator`，`kind = "foundation"`
- `tool-runtime-core`，`kind = "foundation"`
- `tool-shell` / `tool-files` / `tool-web` / `tool-git`，`kind = "provider"`
- `channel-core`，`kind = "foundation"`
- `channel-telegram` / `channel-discord` / `channel-slack`，`kind = "provider"`
- `memory-store`，`kind = "provider"`
- `mcp-client`，`kind = "provider"`
- `skills-runtime`，`kind = "provider"`
- `weft-claw-chat-feature` / `weft-claw-automation-feature` 等，`kind = "feature"`

这个例子不是要求当前仓库立刻全部改名实现，而是要求 blueprint 与后续声明模型已经明确知道自己在往哪里迁移。

## 十四、Package 的成功标准

一个好的 Package 体系应满足：

- Core 能稳定装载它
- Product Package 能清晰知道它属于 provider、foundation、feature 还是 product
- Product Package 能明确绑定它或依赖它提供的 capability
- AI 能安全替换和升级它
- 用户不需要理解它的内部存在

如果某个新能力必须总是直接改 Core，或者总是只能作为“一个不知属于哪层的大包”出现，说明 Package 体系还不够成熟。

## 十五、设计结论

Package 仍然是 WEFT vNext 的内部实现单位，但它现在不应再被理解为单层集合，而应被理解为一个明确的分层模型：

- Provider package 负责原子能力实现
- Foundation package 负责稳定能力面
- Feature package 负责产品语义组合
- WASM 为主，Service / Native 为辅

这几层共同服务于一个目标：

**让用户始终只面对稳定产品实例，让系统内部能持续增长能力，并且知道每个 Package 到底处于哪一层。**

## 十六、当前实现状态

当前代码中，Package 模型已经落地到这些机制：

- `packages/index.toml` 作为 package source 索引
- generation activate 时会把 generation bindings 写入 `lock.packages[]`
- `packages[].sha512` 基于 package source 真实计算生成
- package index 与 lock 中记录 `signature` 元数据
- verify 对已加载与可临时加载的 wasm package 做 live probe
- native package 已有 loader 骨架，并新增 `native-echo` 作为最小示例声明

当前仍未完全落地的是：

- package index 对 provider / foundation / feature 的显式 taxonomy 表达
- feature package 与 foundation package 的更清晰声明语义
- 仓内真实 native 动态库示例产物
- 对缺失 wasm 构建产物的自动 build -> probe 闭环
