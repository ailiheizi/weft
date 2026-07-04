# WEFT vNext 蓝图：Weft Claw Capability 与 Package 矩阵

## 一、文档目的

这份文档是 Weft Claw 迁移阶段的反歧义参考表。

它不重复展开 `../../foundations/03-package-model.md`、`../../runtime/05-binding-and-resolution.md`、`09-product-blueprint.md` 的完整论证，而是把三者已经批准的设计压缩成一份可直接查阅的矩阵，回答下面几个问题：

- 旧的粗粒度 capability 名称，在 Weft Claw vNext 里应落到哪些新名称
- 每个新 capability 对应的是 provider package、foundation package、feature package，还是 product package 职责
- 哪些 provider 是 Weft Claw 默认绑定
- 哪些绑定应固定，哪些允许 mutable，哪些只能条件替换
- 哪些能力留在 product package 边界内，不能重新滑回 Core 或底层 provider

因此，这份 10 号文档的角色是：

- 对 `03` 的 package 分层做 Weft Claw 场景落表
- 对 `05` 的 capability / binding / mutability 规则做 Weft Claw 场景落表
- 对 `09` 的 Weft Claw 分层方案做可执行的矩阵化整理

如果后续 03、05、09 更新，本文件应优先同步，因为它承担迁移期的统一术语表职责。

## 二、使用规则

阅读和使用这份矩阵时，必须坚持以下边界：

1. Product Package 面向 capability，不面向 package 实现名，详细原则见 `../../runtime/05-binding-and-resolution.md`
2. Package 是统一源码发布单位，WASM 为主、Native 为辅，详细原则见 `../../foundations/03-package-model.md`
3. Weft Claw 仍按 `provider -> foundation -> feature -> product package` 四层理解，详细结构见 `09-product-blueprint.md`
4. 本文不新增任何未经 03、05、09 批准的运行时行为
5. 本文中的 default provider 表示 Weft Claw 默认装配选择，不表示 Core 全局硬编码默认值

## 三、Weft Claw 装配层次总览

| 层级 | 角色 | 典型包 | 对应文档锚点 | 是否面向用户直接暴露 |
| --- | --- | --- | --- | --- |
| Capability Provider Packages | 提供原子 capability 实现 | `tool-shell`, `tool-files`, `tool-web`, `tool-git`, `memory-store`, `channel-telegram`, `mcp-client`, `skills-runtime` | `09` 第 4.1、6.2、6.3、6.4、6.5 节 | 否 |
| Foundation Packages | 统一协议、状态和调用面 | `agent-runtime`, `prompt-system`, `workflow-orchestrator`, `tool-runtime-core`, `channel-core` | `09` 第 4.2、6.1、6.2、6.4 节 | 否 |
| Weft Claw Feature Packages | 组合成用户可感知的产品能力块 | `weft-claw-chat-feature`, `weft-claw-channel-feature`, `weft-claw-automation-feature`, `weft-claw-extensions-feature` | `09` 第 4.3、7.x 节 | 间接 |
| Weft Claw Product Package | 提供产品身份、默认装配和产品级入口 | `weft-claw`, `weft-claw-ui` | `09` 第 4.4、8.x 节 | 是 |

这张表是全文的阅读前提。

- `03` 解释了为什么这些都仍然是 package 世界的一部分
- `05` 解释了为什么它们最终都要落到 capability binding
- `09` 解释了为什么 Weft Claw 不能跳过中间两层，直接变成“大产品包 + 一堆插件”

## 四、旧 capability 名称到新 capability 名称的映射

`05` 里仍保留了一组旧的粗粒度 capability 名称，用来说明 binding 原理。`09` 则给出了 Weft Claw 所需的更细粒度命名。迁移时应以下表为准。

| 旧 capability 名称 | 新 capability 名称 | 迁移解释 | 默认提供者 | mutability | 备注 |
| --- | --- | --- | --- | --- | --- |
| `agent.capability` | `agent.runtime` | 旧的“代理主能力”拆成更明确的运行时能力面 | `agent-runtime` | 条件替换 | 见 `09` 11.3 |
| `agent.capability` | `prompt.system` | prompt 组装从 agent 内部拆出独立 capability | `prompt-system` | 条件替换 | 见 `09` 6.1、11.3 |
| `agent.capability` | `workflow.orchestration` | 产品行为编排从 agent 主循环中分离 | `workflow-orchestrator` | 条件替换 | 见 `09` 6.1、11.3 |
| `tool.registry` | `tool.runtime` | 旧 registry 概念升级为统一工具运行面 | `tool-runtime-core` | 固定或高约束 mutable | 作为工具总入口，不建议随意漂移 |
| `tool.registry` | `tool.shell` | shell 执行 provider 独立化 | `tool-shell` | Mutable | 见 `09` 11.2 |
| `tool.registry` | `tool.files` | 文件访问 provider 独立化 | `tool-files` | Mutable | 见 `09` 11.2 |
| `tool.registry` | `tool.web` | web 检索/抓取 provider 独立化 | `tool-web` | Mutable | 见 `09` 11.2 |
| `tool.registry` | `tool.git` | git 操作 provider 独立化 | `tool-git` | Mutable | 见 `09` 11.2 |
| `memory.backend` | `memory.store` | 名称与职责更贴近 provider 实现边界 | `memory-store` | Mutable | `05` 中 mutable 的代表场景 |
| `communication.bridge` | `channel.bridge` | 旧通信桥更明确聚焦到多 channel 抽象层 | `channel-core` | Mutable | foundation 本身可受策略约束 |
| `communication.bridge` | `channel.telegram` | 渠道 adapter 单独声明 | `channel-telegram` | Mutable | adapter 默认按需启用 |
| `communication.bridge` | `channel.discord` | 渠道 adapter 单独声明 | `channel-discord` | Mutable | adapter 默认按需启用 |
| `communication.bridge` | `channel.slack` | 渠道 adapter 单独声明 | `channel-slack` | Mutable | adapter 默认按需启用 |
| `communication.bridge` | `ext.mcp` | 外部扩展接入不再混入 channel 或 tool 语义 | `mcp-client` | Mutable | `09` 9.3 允许保持较粗 |
| `communication.bridge` | `ext.skills` | skills 装载能力保持独立 capability | `skills-runtime` | Mutable | `09` 9.3 允许保持较粗 |
| `ui.surface` | `ui.surface` | 名称保持不变，但边界更明确 | `weft-claw-ui` | Fixed | 见 `09` 11.1 |
| `execution.capability` | `workflow.orchestration` + `tool.runtime` + 具体 `tool.*` | 旧执行能力不是单一 provider，应拆成编排层、工具运行层和具体执行 provider | `workflow-orchestrator` + `tool-runtime-core` + `tool-shell` 等 | 混合 | 迁移时禁止继续把它当成一个含糊大能力 |

### 迁移判断原则

| 判断问题 | 迁移结论 |
| --- | --- |
| 旧名字能否继续保留为 Weft Claw 内部 binding 键 | 不建议。迁移后应以新 capability 名称为 canonical key |
| 旧名字是否还能用于解释系统原理 | 可以，但仅用于和 `05` 旧示例对照 |
| `execution.capability` 是否应该保留为单独新能力 | 不应保留。它在 Weft Claw 蓝图中已经被拆成更真实的结构边界 |

## 五、Weft Claw capability 到 package 层的 canonical 矩阵

下表是本文件最核心的参考矩阵。后续 declaration、config、lock、binding 讨论，都应以此表为优先术语基线。

| Capability | Legacy 来源 | Provider Package | Foundation Package | Feature Package | Product Package 责任 | Weft Claw 默认 provider | Mutability | 说明 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `agent.runtime` | `agent.capability` | 无单独 provider，默认即 `agent-runtime` 实现本体 | `agent-runtime` | `weft-claw-chat-feature` | 选择默认 agent 行为组合 | `agent-runtime` | 条件替换 | 产品中枢之一，不应像普通工具 provider 一样随意替换 |
| `prompt.system` | `agent.capability` | 无单独 provider，默认即 `prompt-system` 实现本体 | `prompt-system` | `weft-claw-chat-feature` | 选择默认 persona / prompt 组合 | `prompt-system` | 条件替换 | 明确从 agent 内部拆出 |
| `workflow.orchestration` | `agent.capability`, `execution.capability` | 无单独 provider，默认即 `workflow-orchestrator` 实现本体 | `workflow-orchestrator` | `weft-claw-automation-feature` | 选择默认 workflow 模式 | `workflow-orchestrator` | 条件替换 | 负责产品级执行策略，不是底层工具实现 |
| `tool.runtime` | `tool.registry`, `execution.capability` | 无单独 provider，默认即 `tool-runtime-core` 实现本体 | `tool-runtime-core` | `weft-claw-automation-feature` | 控制默认启用哪些工具类能力 | `tool-runtime-core` | 高约束 mutable | 更像工具能力总线，不建议轻易切换 |
| `tool.shell` | `tool.registry`, `execution.capability` | `tool-shell` | `tool-runtime-core` | `weft-claw-automation-feature` | 控制是否启用 shell 能力 | `tool-shell` | Mutable | 具体执行 provider |
| `tool.files` | `tool.registry` | `tool-files` | `tool-runtime-core` | `weft-claw-automation-feature` | 控制是否启用文件能力 | `tool-files` | Mutable | 具体执行 provider |
| `tool.web` | `tool.registry` | `tool-web` | `tool-runtime-core` | `weft-claw-automation-feature` | 控制是否启用 web 能力 | `tool-web` | Mutable | 具体执行 provider |
| `tool.git` | `tool.registry` | `tool-git` | `tool-runtime-core` | `weft-claw-automation-feature` | 控制是否启用 git 能力 | `tool-git` | Mutable | 具体执行 provider |
| `memory.store` | `memory.backend` | `memory-store` | 无独立 foundation | `weft-claw-chat-feature` | 控制聊天默认是否带记忆 | `memory-store` | Mutable | `05` 中最典型的 mutable binding |
| `channel.bridge` | `communication.bridge` | 无单独 provider，默认即 `channel-core` 实现本体 | `channel-core` | `weft-claw-channel-feature` | 选择默认渠道接入策略 | `channel-core` | Mutable | 统一消息桥和路由抽象 |
| `channel.telegram` | `communication.bridge` | `channel-telegram` | `channel-core` | `weft-claw-channel-feature` | 控制 telegram 渠道是否启用 | 无强制默认 | Mutable | adapter 按配置启用 |
| `channel.discord` | `communication.bridge` | `channel-discord` | `channel-core` | `weft-claw-channel-feature` | 控制 discord 渠道是否启用 | 无强制默认 | Mutable | adapter 按配置启用 |
| `channel.slack` | `communication.bridge` | `channel-slack` | `channel-core` | `weft-claw-channel-feature` | 控制 slack 渠道是否启用 | 无强制默认 | Mutable | adapter 按配置启用 |
| `ext.mcp` | `communication.bridge` | `mcp-client` | 无独立 foundation | `weft-claw-extensions-feature` | 控制扩展系统是否开放 MCP | `mcp-client` | Mutable | `09` 明确保持独立 package，但不过度拆层 |
| `ext.skills` | `communication.bridge` | `skills-runtime` | 无独立 foundation | `weft-claw-extensions-feature` | 控制扩展系统是否开放 skills | `skills-runtime` | Mutable | `09` 明确保持独立 package，但不过度拆层 |
| `ui.surface` | `ui.surface` | `weft-claw-ui` | 无独立 foundation | 无独立 feature，直接被 product package 暴露 | 提供 UI 入口、产品级 route、产品身份呈现 | `weft-claw-ui` | Fixed | 属于产品身份边界，不能自由漂移 |

## 六、默认 provider 与 mutability 策略矩阵

这一节把 `05` 的 fixed / mutable 思路，与 `09` 的 Weft Claw 约束做更直接的对照。

| Capability | 默认 provider | 默认策略 | 可否被 user/profile override | override 约束 | 对应来源 |
| --- | --- | --- | --- | --- | --- |
| `agent.runtime` | `agent-runtime` | 条件固定 | 可以，但不应自由漂移 | 必须经过 propose / resolve / verify / activate | `05` 第 9 节，`09` 第 11.3 节 |
| `prompt.system` | `prompt-system` | 条件固定 | 可以，但受产品行为边界约束 | 替换后不能破坏 Weft Claw 产品默认语义 | `09` 第 11.3 节 |
| `workflow.orchestration` | `workflow-orchestrator` | 条件固定 | 可以，但受策略控制 | 应视为产品行为中枢之一 | `09` 第 11.3 节 |
| `tool.runtime` | `tool-runtime-core` | 高约束默认 | 理论上可以，实践上应谨慎 | 替换会影响全部工具 provider 接入面 | `05` 第 4 节延伸解释 |
| `tool.shell` | `tool-shell` | 默认启用 | 可以 | 受 profile / policy 限制 | `05` 第 4 节，`09` 第 11.2 节 |
| `tool.files` | `tool-files` | 默认启用 | 可以 | 受 profile / policy 限制 | 同上 |
| `tool.web` | `tool-web` | 默认启用 | 可以 | 受 profile / policy 限制 | 同上 |
| `tool.git` | `tool-git` | 默认启用 | 可以 | 受 profile / policy 限制 | 同上 |
| `memory.store` | `memory-store` | 默认启用 | 可以 | 可作为标准 mutable binding | `05` 第 4 节、9 节 |
| `channel.bridge` | `channel-core` | 默认可装配 | 可以 | 受是否启用 channel feature 影响 | `09` 第 10.3 节 |
| `channel.telegram` | 无强制默认 | 按需启用 | 可以 | adapter 可独立开关 | `09` 第 7.2 节、10.3 节 |
| `channel.discord` | 无强制默认 | 按需启用 | 可以 | adapter 可独立开关 | 同上 |
| `channel.slack` | 无强制默认 | 按需启用 | 可以 | adapter 可独立开关 | 同上 |
| `ext.mcp` | `mcp-client` | 默认可选 | 可以 | 可整体关闭或替换 | `09` 第 10.3、11.2 节 |
| `ext.skills` | `skills-runtime` | 默认可选 | 可以 | 可整体关闭或替换 | `09` 第 10.3、11.2 节 |
| `ui.surface` | `weft-claw-ui` | 固定 | 不应作为普通 override 项 | 它定义产品表面与入口 | `09` 第 8.1、11.1 节 |

## 七、feature package 视角的能力归属矩阵

这一节的目的是防止 feature、foundation、provider 三层重新混淆。

| Feature Package | 主要组合的 capability | 默认依赖的 package | 自己不应吸收的职责 | 对应来源 |
| --- | --- | --- | --- | --- |
| `weft-claw-chat-feature` | `agent.runtime`, `prompt.system`, `memory.store` | `agent-runtime`, `prompt-system`, `memory-store` | 不应吸收 agent loop 实现细节，不应吸收 memory backend 实现细节 | `09` 第 7.1 节 |
| `weft-claw-channel-feature` | `channel.bridge`, `channel.telegram`, `channel.discord`, `channel.slack` | `channel-core` + selected adapters | 不应把各 adapter 协议细节重新封回 product package | `09` 第 7.2 节 |
| `weft-claw-automation-feature` | `workflow.orchestration`, `tool.runtime`, `tool.shell`, `tool.files`, `tool.web`, `tool.git` | `workflow-orchestrator`, `tool-runtime-core`, provider packages | 不应把底层工具 provider 实现并入 workflow 层 | `09` 第 7.3 节 |
| `weft-claw-extensions-feature` | `ext.mcp`, `ext.skills` | `mcp-client`, `skills-runtime` | 不应把 MCP 或 skills 协议细节重新塞回 product package | `09` 第 7.4 节 |

## 八、Product Package 边界矩阵

这张表直接对应 `09` 第 8 节，目标是防止迁移时“哪些内容还属于 product package”再次变模糊。

| 领域 | 属于 product package | 不属于 product package | 原因 |
| --- | --- | --- | --- |
| 产品身份 | `Weft Claw` 名称、版本、说明、默认 profile、健康入口 | 底层 provider 内部实现 | 用户应感知稳定产品，而不是内部装配细节 |
| 默认装配 | 默认启用哪些 feature packages，默认 binding 指向哪些 providers | provider 的内部算法与协议细节 | product package 负责产品默认值，不负责底层实现 |
| UI | `weft-claw-ui`、产品级 route、产品入口面 | agent loop、tool provider、channel adapter 实现 | UI 属于产品表面，不能反向吸收底层运行逻辑 |
| 运行入口 | `health / run / update / rollback` 等产品对外入口 | Core capability registry、底层 package loader | product package 负责产品级对外入口声明，Core 负责平台裁决 |
| 配置 | profile、workspace、feature 开关、provider override 入口 | resolve / verify / activate 的平台执行本体 | product package 只暴露配置入口，不取代 Core 生命周期流程 |
| 产品策略 | 默认 workflow / prompt 组合选择权 | prompt 文本内部结构、workflow 引擎实现细节 | product package 负责产品级选择，不负责具体实现 |

## 九、迁移时的禁止歧义清单

下面这些说法在 Weft Claw vNext 迁移中都应视为不精确，必要时要改写成本文矩阵中的 canonical 说法。

| 不精确说法 | canonical 改写 |
| --- | --- |
| “Weft Claw 需要一个 agent capability” | `weft-claw` 需要 `agent.runtime`，通常还会同时绑定 `prompt.system` 与 `workflow.orchestration` |
| “tool registry 就是所有执行能力” | `tool.runtime` 是统一运行面，具体执行能力由 `tool.shell`、`tool.files`、`tool.web`、`tool.git` 等 provider 提供 |
| “communication bridge 负责 channels、MCP、skills” | `channel.bridge` 只对应 channel 抽象层，`ext.mcp` 与 `ext.skills` 是独立 capability |
| “execution capability 默认就是 command tools” | 在 Weft Claw 新蓝图中，不应再使用单一 `execution.capability` 描述这组结构 |
| “Weft Claw 产品包负责 agent / tools / channels 细节” | `weft-claw` product package 只负责产品装配、默认策略和对外入口声明 |
| “ui.surface 也是普通 mutable provider” | `ui.surface` 属于产品身份边界，默认 fixed |

## 十、对 declaration / config / lock 同步的直接指导

为了让后续与 `../../runtime/04-declaration-config-lock.md` 以及 `../../runtime/05-binding-and-resolution.md` 的同步更容易，Weft Claw 相关声明应优先遵循以下约束：

| 载体 | 建议记录内容 | 术语来源 |
| --- | --- | --- |
| `package.toml` | 使用本文的 canonical capability 名称，声明默认 feature package 和 binding 意图 | 以本文和 `09` 为主，结合 `05` |
| `config.toml` | 使用本文的 mutable / fixed 分类暴露 override 入口 | 以本文和 `05` 为主 |
| `lock.toml` | 锁定本文矩阵中的最终 capability -> provider 解析结果 | 以本文、`03`、`05` 为主 |

如果后续文档或实现中出现新的命名，必须先回答三个问题：

1. 它是本文某个 capability 的别名，还是一个真正新增的 capability
2. 它落在 provider、foundation、feature，还是 product package
3. 它的默认 mutability 是否已在 `05` 和 `09` 的约束范围内得到解释

在这三个问题没有明确之前，不应把新命名写入 Weft Claw 迁移主线文档或锁文件结构说明。

## 十一、一句话结论

Weft Claw 迁移时，真正稳定的不是旧的粗粒度 capability 叫法，也不是某几个历史 package 名，而是这套经过 `03`、`05`、`09` 共同约束后的 canonical 矩阵：

**Product Package 面向新的 capability 名称装配产品，Package 分层提供实现，Core 负责 binding 与裁决，product package 只保留产品身份与默认策略。**
