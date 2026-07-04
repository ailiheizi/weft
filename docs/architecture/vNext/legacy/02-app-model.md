# WEFT vNext 蓝图：旧 App 模型与收敛说明

## 一、为什么保留这份文档

这份文档保留的目的，不再是继续把 App 作为 vNext 的最终源码模型，而是说明旧 App 模型曾经解决了什么问题，以及这些职责如何收敛进 Product Package + Instance 模型。

旧 App 模型曾经试图解决两个问题：

1. 用户心智和系统心智不一致
2. 像 `weft-claw` 这样的产品会在 Package / Suite / 插件实现之间摇摆，最终在 Core 中出现特判

在当前收敛方案中，这两个问题不再通过独立源码层 App 解决，而是通过：

- `product package` 表达产品默认意图
- `instance` 表达当前环境中的运行事实

因此，本文件应视为旧模型说明，新的主模型以 `../foundations/13-product-package-instance-model.md` 为准。

## 二、旧 App 模型的核心价值

旧 App 模型真正有价值的部分包括：

- 产品身份需要和内部实现边界分离
- 产品默认装配需要有独立声明层
- 用户配置不应与解析结果混在一起
- generation / verify / activate / rollback 需要稳定锚点

这些价值在新模型中分别收敛为：

- product package：产品默认装配与默认策略
- instance：用户配置、lock、generation、rollback

## 三、这些职责如何收敛进新模型

### 1. 对用户简单

用户仍然看到的是：

- 产品名称
- 功能说明
- 版本
- 模式（安全 / 开发者 / 完全信任）
- 更新信息
- 当前健康状态

用户仍然不需要理解：

- package 图
- 能力绑定细节
- provider 替换逻辑
- 内部实现细节

在新模型里，用户面对的不是源码层 App，而是某个 product package 的 instance。

### 2. 对系统可组合

旧 App 模型里真正成立的，是“产品默认装配应有明确层次”，而不是“必须存在独立 App 源码层”。

在新模型中，一个成熟产品默认仍然落到以下四层：

1. provider packages
2. foundation packages
3. feature packages
4. product package

其中：

- provider packages 提供原子 capability 实现
- foundation packages 把同类 provider 组织成稳定能力面
- feature packages 把多个 capability 组合成用户能感知的功能块
- product package 负责默认装配、默认策略与产品级入口声明

Core 负责解析这些能力声明、binding 与 instance override，把它们装配成一个可运行产品实例。

### 3. 对 AI 可维护

AI 仍然可以：

- 提议升级某个 Package
- 替换某个能力的实现方
- 调整默认绑定策略
- 在验证通过后生成新 generation

但 AI 改的是 product package 的默认装配与 instance 的运行配置，不是直接漂移用户感知的稳定产品身份。

## 四、旧 App 模型中的组成，现改写为 Product Package + Instance

原本归属于 App 的内容，现分流如下：

### 1. Product Package Identity

用于表达产品身份：

- name
- version
- display name
- description
- author / publisher
- icon / branding

### 2. Product Package Required Capabilities

product package 不直接依赖所有底层 Package，而是先声明自己需要哪些 capability。

原则仍然是：

- product package 面向 capability，不面向具体 package 名称
- capability 要足够表达装配结构，但不追求无限细拆
- `ext.mcp`、`ext.skills`、`ui.surface` 这类边界较稳定的能力，可先保持相对集中

### 3. Product Package Default Bindings

product package 需要声明每种 capability 默认绑定到哪个 Package。

例如：

- `agent.runtime -> agent-runtime`
- `prompt.system -> prompt-system`
- `workflow.orchestration -> workflow-orchestrator`
- `tool.runtime -> tool-runtime-core`
- `tool.shell -> tool-shell`
- `tool.files -> tool-files`
- `tool.web -> tool-web`
- `tool.git -> tool-git`
- `memory.store -> memory-store`
- `channel.bridge -> channel-core`
- `ext.mcp -> mcp-client`
- `ext.skills -> skills-runtime`
- `ui.surface -> weft-claw-ui`

### 4. Binding Rules

product package 需要声明 binding 规则，而 instance 负责在允许边界内做运行态 override。

在收敛模型里，binding 继续分成三层：

1. product package default binding
2. feature-level binding
3. instance runtime override

### 5. Validation Rules

product package 需要定义自己的验证标准，例如：

- 能否启动
- 能否完成一次基本对话
- 能否执行一次基础工具调用
- 能否完成一次基础记忆写入与读取

### 6. Upgrade Policy

product package 需要定义：

- 允许自动升级什么
- 哪些能力升级后必须重新验证
- 是否允许自动 rebinding

## 五、运行生命周期在新模型中的归属

在新模型中，下面这些状态不再属于源码层 App，而属于 instance 生命周期：

1. declared — 已有 product package 声明
2. resolved — 依赖与绑定已解析
3. locked — 精确结果已锁定
4. candidate — 候选 generation 已生成
5. verified — 验证通过
6. active — 已激活
7. rollback-available — 可回滚

用户不需要理解这些内部状态，但 Core 仍需要用这些状态保证升级安全。

## 六、产品内部层次仍然成立，但顶层对象改为 Product Package

不是所有产品都要一次性长成复杂系统，但 vNext 仍然必须允许明确的内部层次。

对于一个成熟的产品型产品，推荐采用以下四层：

### 1. Provider Packages

provider packages 提供原子能力实现，例如：

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

### 2. Foundation Packages

foundation packages 负责把一类 provider 组织成稳定能力面，例如：

- `agent-runtime`
- `prompt-system`
- `workflow-orchestrator`
- `tool-runtime-core`
- `channel-core`

### 3. Feature Packages

feature packages 负责把多个 capability 组合成用户能感知的产品功能块，例如：

- `weft-claw-chat-feature`
- `weft-claw-channel-feature`
- `weft-claw-automation-feature`
- `weft-claw-extensions-feature`

### 4. Product Package

最上层是 product package。它不是大单体，也不应薄到只剩 capability 列表。

product package 的职责是：

- 选择默认启用哪些 feature packages
- 选择默认 bindings 指向哪些 providers
- 选择默认 workflow / prompt 组合
- 暴露 UI 与产品级入口声明
- 保持产品身份稳定

## 七、当前文档地位

从这一轮收敛开始，本文件不再是主模型定义文档，而是旧 App 模型向 Product Package + Instance 模型迁移时的解释文档。

后续如需继续收敛，应优先修改：

- `../foundations/13-product-package-instance-model.md`
- `../foundations/03-package-model.md`
- `../runtime/04-declaration-config-lock.md`
- `../runtime/05-binding-and-resolution.md`
