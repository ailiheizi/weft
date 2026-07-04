# WEFT vNext 蓝图：Weft Claw 作为首个 Product Package 的详细方案

## 一、为什么要重写 Weft Claw 蓝图

`weft-claw` 不只是当前仓库里的一个示例产品，它其实承载了整个 vNext 架构最难的一组问题：

- 它既是一个产品，又不是单一功能模块
- 它天然需要 agent、prompt、workflow、tools、memory、channels、UI、扩展系统共同工作
- 它历史上混合了 suite、plugin、chat provider、UI 状态、系统集成等多种角色
- 它非常容易把大量产品逻辑重新写回 Core，导致特判复活

因此，`weft-claw` 的目标不应只是“成为首个源码层产品对象”，而应成为：

**首个由多种 capability packages、少量 feature packages 和一个 product package 装配出来的产品型实例。**

如果这件事做成，WEFT vNext 才真正证明了三件事：

1. product package + instance 确实可以成为用户视角的稳定产品单位
2. Package 确实可以成为内部能力实现与替换单位
3. Core 确实可以只做 capability 平台，而不是重新吞回产品逻辑

## 二、Weft Claw 的重做目标

新的 `weft-claw` 蓝图必须同时满足以下目标：

1. `weft-claw` 正式成为 product package，而不是旧式 suite / plugin 特判集合
2. `weft-claw` 内部能力被拆成一组可以替换、升级、组合、禁用的 packages
3. `weft-claw` 作为 product package 不再承载大量底层实现，只保留产品装配与默认策略
4. 参考 `reference/weft-claw` 中的重要产品能力面，能够在新模型中找到清晰归属
5. 未来不止 `weft-claw`，别的产品也能复用这批 packages 重新装配出新的 product package

换句话说，`weft-claw` 在 vNext 中不是“大产品壳带几个默认插件”，而是一个产品装配结果。

## 三、用户视角与系统视角必须分离

### 3.1 用户视角

用户看到的应始终是：

- 产品名称：Weft Claw
- 产品说明
- 当前版本
- 当前 profile / 权限模式
- 是否健康
- 是否可更新
- 默认有哪些产品能力可用

用户不应被迫理解：

- 底层由多少 package 组成
- 哪些能力由 WASM、Service 或 Native 提供
- 哪个 channel、tool、memory provider 正在工作
- 具体 binding / provider 切换细节

### 3.2 系统视角

系统内部必须明确知道：

- `weft-claw` 需要哪些 capability
- 当前由哪些 package 提供这些 capability
- 哪些 binding 固定，哪些 binding 可替换
- 哪些能力属于基础 provider，哪些属于 product feature
- 哪些变化需要经过 propose / resolve / verify / activate

因此，用户视角与系统视角必须同时成立：

- 用户面对稳定产品
- 系统面对可组合、可验证、可替换的内部结构

## 四、Weft Claw 的总体结构定位

在新的蓝图中，`weft-claw` 应理解为四层结构：

1. **Capability Provider Packages**
2. **Foundation Packages**
3. **Weft Claw Feature Packages**
4. **Weft Claw Product Package**

这是一个明确的装配层次，而不是抽象建议。

### 4.1 Capability Provider Packages

最底层的 provider packages 负责提供原子能力实现，例如：

- `tool-shell`
- `tool-files`
- `tool-web`
- `tool-git`
- `channel-telegram`
- `channel-discord`
- `channel-slack`
- `mcp-client`
- `skills-runtime`
- `memory-store`

这类 package 的目标是：

- 提供清晰、有限、可替换的 capability 实现
- 不承担完整产品身份
- 不承担“整个 weft-claw 应该怎么工作”的产品责任

### 4.2 Foundation Packages

Foundation packages 负责把一类 provider 组织成稳定能力面，例如：

- `agent-runtime`
- `prompt-system`
- `workflow-orchestrator`
- `tool-runtime-core`
- `channel-core`

它们的职责不是成为最终产品，而是成为较稳定、较可复用的中间层：

- 统一协议
- 统一状态模型
- 统一调用面
- 统一组合方式

### 4.3 Weft Claw Feature Packages

Feature packages 用来承载 Weft Claw 的产品语义，把多个底层 capability 组合成用户能感知的功能块，例如：

- `weft-claw-chat-feature`
- `weft-claw-channel-feature`
- `weft-claw-automation-feature`
- `weft-claw-extensions-feature`

这一层很关键，因为它避免两种坏结果：

- 坏结果 A：全部逻辑直接堆回 `weft-claw` product package
- 坏结果 B：所有产品语义散落到底层 packages 中

### 4.4 Weft Claw Product Package

最上层 `weft-claw` product package 是产品默认装配层，而不是大单体。它负责：

- 默认启用哪些 feature packages
- 默认 bindings 指向哪些 providers
- 选择默认 workflow / prompt 组合
- 暴露 UI 与产品级入口声明
- 保持产品身份稳定

## 五、拆分原则：什么该拆，什么不该拆

### 5.1 默认原则

默认优先拆分，但必须满足以下条件之一：

- 具有清晰 capability 边界
- 存在真实复用价值
- 需要单独替换、升级、禁用或测试
- 天然具备 provider / adapter 关系

### 5.2 不允许为了“看起来模块化”而过度拆分

如果某部分：

- 只是局部实现细节
- 没有独立复用意义
- 拆完后只会增加 binding 与配置噪音

则不应强行拆成 package。

### 5.3 MCP 与 Skills 的特殊规则

`mcp` 与 `skills` 需要保持可插拔，但不做过度分层。

它们在第一阶段直接定义为两个独立 package：

- `mcp-client`
- `skills-runtime`

不再继续拆成 `core + adapter + orchestrator` 多层结构。原因很简单：

- 它们确实有独立边界
- 它们确实需要可替换与可禁用
- 但目前没有足够证据证明再细拆会带来真实收益

## 六、Weft Claw 第一阶段建议拆出的 Package 体系

这一节是蓝图的核心之一。以下不是“可能会有”，而是推荐的第一阶段详细结构。

### 6.1 Agent / Prompt / Workflow 组

#### `agent-runtime`

职责：

- agent turn loop
- provider interaction
- tool dispatch 主框架
- 回合状态推进
- 工具调用与模型回复间的主控流程

明确不负责：

- 具体 system prompt 内容
- 具体 persona 选择
- 产品级 workflow 策略
- channel 适配

#### `prompt-system`

职责：

- system prompt 组装
- persona 定义
- mode / role prompt 模板
- policy prompt 片段组合
- prompt assembly 规则

明确不负责：

- agent turn loop
- 工具执行
- 产品级 feature 开关

这部分必须独立成 package，因为它既需要复用，也需要灵活替换；你已经明确要求“系统写入”本身必须能拆出来。

#### `workflow-orchestrator`

职责：

- 多阶段任务流编排
- 不同模式下的执行策略切换
- 面向产品行为的 orchestration
- 自动化流程和任务编排规则

明确不负责：

- 具体模型 provider 通信细节
- 具体 prompt 文本本体
- 底层工具执行实现

这一层存在的目的，是避免把“产品怎么运转”重新压进 `agent-runtime` 或 `weft-claw` product package。

### 6.2 Tool Runtime 组

#### `tool-runtime-core`

职责：

- 统一工具协议
- 工具注册模型
- 工具调用入口
- 工具结果统一表示
- 工具权限/可见性/能力分类模型

#### `tool-shell`

职责：

- shell / command execution 能力
- 命令输入输出模型
- 退出码、stderr/stdout、timeout 语义

#### `tool-files`

职责：

- 文件读写、编辑、目录访问等能力

#### `tool-web`

职责：

- web fetch / search / remote content retrieval 能力

#### `tool-git`

职责：

- git status / diff / log / branch / commit 等能力

必要时后续可扩展：

- `tool-browser`
- `tool-jira`
- `tool-notion`
- `tool-docs`

这里采用明确的两层结构：

- 一层统一工具运行面
- 多个具体工具 provider packages

这样 `weft-claw` 才能按 profile 或 feature 选择装哪些工具，而不是被迫加载整套工具系统。

### 6.3 Memory / Session 组

#### `memory-store`

职责：

- 记忆写入
- 记忆检索
- 长短期记忆组织
- 摘要与归档模型
- 后端切换边界

#### `session-state`

这是一个**可选独立 package**。原则如下：

- 如果会话状态在实现里表现为稳定、明确、可复用的边界，则可独立
- 如果第一阶段仍与 feature / product package 绑定较深，则暂不强拆

这部分允许在 implementation plan 阶段再做一次收敛，但蓝图必须明确它是一个独立审视对象，而不是隐形塞进 product package。

### 6.4 Channel 组

#### `channel-core`

职责：

- 统一消息桥协议
- channel session / routing 抽象
- inbound / outbound 消息模型
- channel 与 agent / workflow 之间的桥接

#### `channel-telegram`
#### `channel-discord`
#### `channel-slack`

职责：

- 各自渠道协议适配
- 鉴权与接入细节
- 渠道专属消息格式处理

必要时后续可扩展：

- `channel-whatsapp`
- `channel-matrix`
- `channel-signal`

这里必须明确采用：

- `channel-core` + `channel-adapters`

而不是一个大而全的 `channels` 包。原因是：

- 多渠道不是一个统一实现细节，而是一组天然 adapter
- 某些 product instances 只需要一个渠道，某些 product instances 需要多个渠道并存
- 不拆会导致后续再次形成“一个大包里塞满所有外部接入”的问题

### 6.5 Extension 组

#### `mcp-client`

职责：

- MCP server 配置
- MCP 连接与调用
- 把 MCP 暴露为 agent / workflow 可消费能力

#### `skills-runtime`

职责：

- skills 文件读取
- skill 元数据解析
- skill 装载与能力暴露

这两者都必须是独立 package，但都不做更多层拆分。

## 七、Weft Claw Feature Packages 的详细定位

底层 package 足够多之后，如果没有 feature 层，产品语义会再次丢失。因此蓝图明确要求引入少量 feature packages。

### 7.1 `weft-claw-chat-feature`

建议组合：

- `agent-runtime`
- `prompt-system`
- `memory-store`

职责：

- 形成基本对话代理产品能力
- 提供默认会话模式
- 承接产品层聊天入口所需的能力组合

### 7.2 `weft-claw-channel-feature`

建议组合：

- `channel-core`
- 一个或多个 channel adapter

职责：

- 把 Weft Claw 变成多渠道可接入产品
- 统一渠道接入策略
- 避免 product package 直接面向多个 adapter 细节

### 7.3 `weft-claw-automation-feature`

建议组合：

- `workflow-orchestrator`
- `tool-runtime-core`
- 若干工具 provider packages

职责：

- 承接自动化、任务执行、程序监管、连续操作这类产品能力

### 7.4 `weft-claw-extensions-feature`

建议组合：

- `mcp-client`
- `skills-runtime`

职责：

- 把外部扩展系统统一接入产品面
- 给 agent / workflow 暴露“可加载扩展能力”的产品语义

## 八、Weft Claw Product Package 的职责与边界

`weft-claw` product package 在新的蓝图里必须明确是“中等厚度产品默认装配层”。

### 8.1 Product Package 保留的内容

`weft-claw` product package 只保留以下内容：

1. 默认启用哪些 feature packages
2. 默认 bindings 指向哪些 providers
3. 默认 workflow 组合的选择权
4. 默认 prompt 组合的选择权
5. profile、workspace、feature 开关等运行时配置
6. UI 入口与产品级 route
7. health / run / update / rollback 等产品对外入口

### 8.2 Product Package 不应再承载的内容

以下内容必须尽量从 product package 中移出：

- agent loop 实现
- prompt 具体文本和 persona 内部结构
- tool 执行实现细节
- channel adapter 实现
- MCP 协议细节
- skills 装载细节
- memory backend 具体实现

### 8.3 为什么不能把 product package 做得太薄

如果 product package 只剩 capability 列表和最基础 binding，它就会失去产品身份，产品默认行为会散到各个 package 中，不利于：

- 稳定产品体验
- 后续给用户暴露清晰配置
- 形成清晰的产品默认 workflow

### 8.4 为什么也不能把 product package 做得太厚

如果 product package 再次吸收大量底层逻辑，就会重回旧问题：

- package 体系失去意义
- Core 外围重新出现一组产品特判
- 别的产品包难以复用这批能力

因此，`weft-claw` product package 必须明确停在“中等厚度产品默认装配层”这一位置。

## 九、Capability 模型需要怎样扩展

当前蓝图里 `weft-claw` 主要通过这类较粗 capability 表达自身：

- `agent.capability`
- `tool.registry`
- `memory.backend`
- `execution.capability`
- `ui.surface`
- `communication.bridge`

这对第一阶段样板足够，但不足以支持“多个 package 变成一个产品实例”的详细设计。

### 9.1 新蓝图建议的 capability 粒度

建议扩展为更细粒度但仍可控的 capability 集合，例如：

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

### 9.2 这些 capability 的使用原则

- Product Package 面向 capability，不面向 package 名称
- Feature package 依赖 capability，而不是硬写实现层细节
- Provider package 负责声明“自己提供哪些 capability”
- Core 在 resolve 时根据 binding 与 policy 选择最终 provider

### 9.3 哪些 capability 可以保持较粗

并不是所有东西都必须无限细拆。以下能力允许先保持相对集中：

- `ext.mcp`
- `ext.skills`
- `ui.surface`

原因是：

- 它们的边界已经足够稳定
- 当前过细拆分收益不明显

## 十、Binding 模型必须升级为三层

为了让“几乎每个部分都有灵活性”真正落到系统结构上，新的 Weft Claw 蓝图必须明确三层 binding。

### 10.1 Product Package Default Binding

这是 Weft Claw 产品默认使用的 binding，例如：

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

### 10.2 Feature-level Binding

Feature package 自己也可以声明默认依赖关系，例如：

- `weft-claw-chat-feature` 默认依赖 `agent-runtime + prompt-system + memory-store`
- `weft-claw-channel-feature` 默认依赖 `channel-core + selected adapters`

这一层 binding 的意义是：

- 让 feature package 成为真正稳定的产品能力块
- 避免 product package 直接管理过多细碎底层依赖

### 10.3 User / Runtime Override

用户或 profile 可以在 instance `config.toml` 中覆盖某些 provider，例如：

- 切换 memory provider
- 替换 shell provider
- 关闭或替换 channel adapter
- 关闭 `mcp-client` 或 `skills-runtime`

这就是“灵活性”的最终落点：

- 灵活的是 provider 选择、feature 组合与 profile 开关
- 不是让 product package 结构变得模糊或不可验证

## 十一、Fixed / Mutable 的规则要重新定义

当前蓝图里只有较粗的 fixed / mutable 思路，新的 Weft Claw 蓝图需要更明确。

### 11.1 建议固定的部分

通常应固定：

- `ui.surface`
- 与产品身份强绑定的核心 feature package
- Weft Claw 默认产品入口

### 11.2 建议可替换的部分

通常允许 mutable：

- `memory.store`
- `tool.shell`
- `tool.files`
- `tool.web`
- `tool.git`
- `channel adapters`
- `ext.mcp`
- `ext.skills`
- 某些 workflow provider

### 11.3 条件可替换的部分

以下内容允许在受约束条件下替换：

- `prompt.system`
- `agent.runtime`
- `workflow.orchestration`

它们通常更接近产品身份与行为中枢，因此不能像普通工具 provider 一样随意漂移，但也不应被永久写死。

## 十二、WASM、Service、Native 在 Weft Claw 中各自扮演什么角色

### 12.1 WASM / 普通 Package 的角色

WASM 仍然是默认主路线，尤其适合：

- 产品编排
- workflow
- prompt 组织
- 可移植逻辑
- AI 后续自动修改和替换

### 12.2 Service / Managed Runtime 的角色

某些 channel、tool、扩展能力可能更适合 Service runtime，例如：

- 常驻连接型 adapter
- 外部服务桥
- 需要进程生命周期管理的 provider

### 12.3 Native 的角色

Native 仍然只是补充路线，不是默认主结构。它适用于：

- 深本地进程控制
- 平台专属 API
- 高性能宿主集成
- WASM / Service 难以覆盖的系统能力

因此 Weft Claw 的运行策略仍然是：

- 先用 Package / WASM / Service 解决
- 仅在必要时引入 Native

## 十三、Weft Claw 在声明、配置、锁文件中的表达需要怎样变化

新的 09 不是只讲概念，还要指导后续 declaration/config/lock 的演进。

### 13.1 `package.toml`

应更明确表达：

- Product Package 需要哪些 capability
- 默认启用哪些 feature packages
- 哪些 binding 属于固定，哪些允许 mutable

### 13.2 `config.toml`

应更明确表达：

- 当前启用哪些 feature 开关
- 当前 profile / workspace
- 当前是否启用 channels / mcp / skills
- 对哪些 provider 进行 override

### 13.3 `lock.toml`

应更明确表达：

- 实际参与装配的 packages
- feature package 与底层 package 的最终解析结果
- 各 package 的 digest / signature / source metadata
- 当前 generation 的 capability -> provider 最终绑定快照

换句话说，lock 不应只锁“几个默认包”，而应锁住整次产品装配结果。

## 十四、AI 在新的 Weft Claw 蓝图中如何工作

AI 在 `weft-claw` 里最重要的工作，不再只是“改一个包”，而是：

- 增加新的 capability provider package
- 替换 mutable binding
- 调整 feature package 组合
- 调整 product package 默认配置
- 在需要时引入或移除某类 adapter

但所有这些变化都仍必须经过：

- propose
- resolve
- verify
- activate

这意味着新的 Weft Claw 蓝图更适合 AI 参与，因为可变结构被正式写成系统模型，而不是靠隐含约定。

## 十五、这份蓝图具体要替换掉旧版 09 的哪些模糊点

旧版 09 的问题主要有三类：

1. **Package 结构过粗**
   - 只描述了少数默认 binding，无法支撑真实产品拆分
2. **没有 feature package 层**
   - 容易导致产品语义散落到底层 package 或重新回到 product package
3. **没有写清 product package 边界**
   - 后续很容易又把 agent、workflow、channel、MCP、skills 写回 product package 或 Core

新的 09 必须把这些点全部写死，避免后续实现时再次模糊。

## 十六、第一阶段蓝图更新后的设计结论

Weft Claw 在 WEFT vNext 中应被重定义为：

- 一个由多种 capability packages 提供底层能力
- 由少量 feature packages 组合出产品语义
- 再由一个中等厚度 product package 完成默认装配

它的核心不是“某几个固定 package 为它服务”，而是：

**它本身就是一个由 package 组合出来的产品系统。**

## 十七、对后续实现的直接约束

这份蓝图更新后，后续任何与 Weft Claw 相关的实现都应遵守以下约束：

1. 新能力优先判断是否应作为 package / feature package 引入，而不是直接塞进 product package 或 Core
2. `prompt/system` 默认按独立 package 设计，不再隐含塞进 agent
3. `channel` 默认按 `core + adapters` 设计，不再用单一大包兜底
4. `tool` 默认按 `runtime core + providers` 设计，不再把所有工具实现混成一个 registry 包
5. `mcp-client` 与 `skills-runtime` 必须保持独立 package，但不再过度拆层
6. `weft-claw` product package 只保留产品装配、默认策略与对外入口声明，不回收底层实现细节

## 十八、一句话总结

Weft Claw 不应再被描述为“首个产品对象加几种默认 capability”，而应被定义为：

**WEFT vNext 中首个完整展示“多个可替换 packages 如何装配成一个产品实例”的详细样板。**
