# WEFT vNext 蓝图：Product Package + Instance 模型

## 一、为什么要收敛模型

当前 WEFT vNext 同时维护两类源码层对象：

- App
- Package

这套模型在早期阶段有价值，因为它帮助系统把“用户看到的产品”和“内部实现单元”区分开来。
但随着 `weft-claw` 蓝图不断细化，这个分层也开始暴露出新的复杂度：

- `app.toml` 已经不只是产品身份声明，还开始承担越来越多装配职责
- feature package 与 app 层都在描述部分 package 组合关系
- `app.config.toml` 已经开始暴露内部 package 启用信息
- server 侧开始同时维护 app source 与 package source 两套索引
- 开源管理上需要额外维护 `apps/` 目录及其配套说明

这说明当前仓库里的复杂度，已经不再主要来自 capability 体系本身，而是来自“源码层对象重复了一层”。

因此，vNext 在这一阶段应进一步收敛：

**删除 App 作为独立源码层概念，统一以 Package 作为源码发布对象；同时保留 Instance 作为运行态对象。**

一句话总结：

**源码层只有 Package，运行层才有 Instance。**

## 二、收敛目标

本轮模型收敛的目标是：

1. 删除 `App` 作为独立源码工件层
2. 统一所有发布对象为 `Package`
3. 保留产品级语义，但通过 `product package` 表达
4. 将用户配置、lock、generation 归入 `instance` 范畴
5. 简化 source authority、目录结构和开源维护对象

本轮收敛**不**试图：

- 删除 generation / verify / activate / rollback 机制
- 删除 provider / foundation / feature 分层
- 把 lock 与用户配置重新塞回源码树
- 让任意 package 无差别地直接充当产品入口

## 三、核心对象重定义

### 1. Core

Core 仍然是能力中心、调度中枢、验证与切换平台。

Core 负责：

- 维护 capability registry
- 装载与管理 Package
- 解析 product package 与 instance config
- 生成 candidate generation
- 执行 verify / activate / rollback

这部分原则与 `01-core-kernel.md` 保持一致，不因 App 源码层消失而改变。

### 2. Package

从这一版开始，Package 成为唯一源码层发布对象。

Package 不再只是“内部实现单元”的统称，而是一个统一的源码发布模型。不同 Package 通过 taxonomy 区分职责，而不是通过是否属于 App 来区分。

### 3. Instance

Instance 是某个 product package 在当前用户、当前设备、当前项目环境中的运行实例。

Instance 负责：

- 当前用户配置
- 当前项目覆盖配置
- 当前 lock
- active generation
- rollback 历史

也就是说：

- Package 表达“发布与复用”
- Instance 表达“当前环境中的运行事实”

## 四、Package 的新 taxonomy

vNext 中的 Package 统一分为四类：

1. **provider package**
2. **foundation package**
3. **feature package**
4. **product package**

### 4.1 Provider Package

provider package 提供原子 capability 实现，例如：

- `tool-shell`
- `tool-files`
- `tool-web`
- `tool-git`
- `memory-store`
- `mcp-client`
- `skills-runtime`
- `channel-telegram`

这一层的特点是：

- 边界清晰
- 可替换
- 不承担完整产品身份

### 4.2 Foundation Package

foundation package 把一类 provider 组织成稳定能力面，例如：

- `agent-runtime`
- `prompt-system`
- `workflow-orchestrator`
- `tool-runtime-core`
- `channel-core`

这一层负责：

- 统一协议
- 统一状态模型
- 统一调用面
- 统一组合方式

### 4.3 Feature Package

feature package 把多个 capability 组合成用户可感知的功能块，例如：

- `weft-claw-chat-feature`
- `weft-claw-channel-feature`
- `weft-claw-automation-feature`
- `weft-claw-extensions-feature`
- `weft-claw-devteam-feature`

feature package 的职责是保住产品语义，而不是把所有产品逻辑重新压回 product package。

### 4.4 Product Package

product package 用来取代原先的 App 源码层职责。

它负责：

- 产品 identity
- 默认启用哪些 feature package
- 默认 binding 策略
- binding 的 mutable / fixed / conditional mutable 约束
- 产品级验证策略
- 产品级升级策略

它不负责：

- 用户本地配置
- 当前实例 lock
- 当前 active generation
- 当前 workspace / secret / profile 的最终值

因此：

- `weft-claw` 作为 product package 是合理的
- `weft-claw` 不再需要额外再包一层源码级 App

## 五、Product Package 与 Instance 的边界

这是本模型最关键的边界。

### 5.1 Product Package 负责“产品默认意图”

例如：

- 产品叫什么
- 默认启用哪些 feature
- 默认绑定哪些 provider
- 哪些 binding 可以替换
- 默认验证什么

Product Package 回答的问题是：

**“这个产品默认想装成什么样？”**

当某个 product package 需要作为长期运行的 code product 出现时，它可以进一步被实现为一个运行中的 runtime，由多个 clients 连接和驱动。

这并不意味着 App 源码层复活，而只是说明：

- product package 仍然是源码与默认意图载体
- instance 仍然是运行态配置与 lock/generation 载体
- runtime 则是产品在本地运行时暴露出来的统一控制面

例如 `WEFT-Code` 这样的 product package，可以拥有：

- 一个本地 runtime
- 多个 clients（CLI / TUI / GUI）

这个 runtime 应被视为 runtime 本身，而不是第二个 product object；它只负责承载并暴露 product package 的运行中接口，不重新引入源码层 App。

但它仍然不需要重新引入源码层 App 对象。

### 5.2 Instance 负责“当前环境如何运行”

例如：

- 当前 profile 是什么
- 当前 workspace 是哪里
- 当前默认模型是什么
- 当前用户覆盖了哪些 feature
- 当前 override 了哪些 provider
- 当前 lock 锁定到了哪些包和绑定

Instance 回答的问题是：

**“这个产品在当前设备和当前用户环境中到底如何运行？”**

## 六、Declaration / Config / Lock 的新归属

在旧模型中，典型结构是：

```text
apps/
weft-claw/
    app.toml
    app.config.toml
    app.lock
```

在新模型中，三者应重新归位：

### 6.1 Declaration

Declaration 不再属于源码层 App，而属于 product package。

建议形态：

```text
packages/
weft-claw/
    package.toml
```

这个 `package.toml` 至少表达：

- `kind = "product"`
- 产品 identity
- required capabilities
- default enabled features
- default bindings
- validation / upgrade policy

### 6.2 Config

Config 不再跟源码包放在一起，而应归属于 instance。

建议位置：

用户级：

```text
%AppData%/WEFT/instances/weft-claw/config.toml
```

项目级：

```text
<project>/.weft/weft-claw/config.toml
```

Config 至少表达：

- profile
- workspace
- default model
- secret refs
- feature enable / disable override
- binding override

### 6.3 Lock

Lock 也不再属于源码树，而属于 instance。

建议位置：

```text
%AppData%/WEFT/instances/weft-claw/lock.toml
```

或：

```text
<project>/.weft/weft-claw/lock.toml
```

Lock 继续负责记录：

- product package name / version
- generation
- enabled features
- resolved packages
- resolved bindings
- declaration digest
- config digest
- verify 结果

这里必须保持一个原则：

**lock 锁的是一次产品实例装配结果，而不是某个 package 源本身。**

## 七、目录结构建议

### 1. 源码层

```text
packages/
weft-claw/
    package.toml
    README.md
    assets/
    ui/
  weft-claw-chat-feature/
    package.toml
  agent-runtime/
    package.toml
  tool-shell/
    package.toml
```

### 2. 运行层

用户级实例目录：

```text
%AppData%/WEFT/instances/weft-claw/
  config.toml
  lock.toml
  state.json
```

项目级实例目录：

```text
<project>/.weft/weft-claw/
  config.toml
  lock.toml
```

优先级建议：

- project config
- user config
- product package defaults

## 八、Binding 与 Resolve 的新入口

`05-binding-and-resolution.md` 中的核心流程仍成立，但输入对象应重命名并收敛：

旧入口：

- app declaration
- app config
- app lock

新入口：

- product package declaration
- instance config
- instance lock

新的解析流程可以表达为：

1. 读取 product package declaration
2. 读取 instance config
3. 展开 default feature 与 runtime override
4. 收集候选 packages
5. 应用 product-level binding rules
6. 应用 instance-level overrides
7. resolve candidate graph
8. verify
9. 生成 instance lock
10. activate generation

因此，resolve 的语义没有被削弱，只是顶层模型从旧 App 模型收敛为 Product Package + Instance。

## 九、Source Authority 的收敛

当前 server 已经开始同时维护 package source 与 app source 两套索引。

在新模型中，source authority 应收敛为：

- 只维护 package source
- instance 不再属于 source authority 范畴，而属于本地运行态对象

这意味着：

- `/api/sources/packages` 保留
- `/api/sources/apps` 应逐步废弃
- instance 的读取与管理应通过独立的本地运行态 API 暴露

这一点的价值不只是“少一个 API”，而是明确：

- 源码发布对象只有 Package
- 本地运行对象是 Instance

## 十、与现有蓝图文档的关系

本文件不是否定此前蓝图，而是对其进行收敛。

### 1. 对 `00-overview.md` 的影响

需要把“App 是唯一用户入口”的表述收敛为：

- 用户安装和使用的是 product package 的 instance
- Core 解析的是 package graph 与 instance state

### 2. 对 `02-app-model.md` 的影响

该文件的核心语义将被本文件取代。

后续应把 `02-app-model.md` 标记为旧模型参考，或直接重写为 `02-product-package-model.md` 一类的新结构文档。

### 3. 对 `03-package-model.md` 的影响

需要把 taxonomy 从：

- provider
- foundation
- feature

扩展为：

- provider
- foundation
- feature
- product

### 4. 对 `04-declaration-config-lock.md` 的影响

需要把 declaration / config / lock 的归属从 “App 三件套” 改成：

- declaration 属于 product package
- config / lock 属于 instance

### 5. 对 `05-binding-and-resolution.md` 的影响

需要把 resolve 输入统一改写成：

- product package declaration
- instance config
- instance lock

## 十一、迁移原则

为了避免一次性切换导致文档与实现同时失稳，迁移应遵循以下原则：

1. 先新增 product package 蓝图，不立即删除旧 App 文档
2. 先让术语与模型稳定，再进入代码迁移
3. 先兼容读取旧 `apps/<name>/app.toml`，再切换到 `packages/<name>/package.toml`
4. 先把 config / lock 外移为 instance 文件，再移除 `apps/`
5. 在迁移完成前，允许旧路径作为兼容输入存在，但不再作为未来主模型

## 十二、设计结论

本轮收敛后的 vNext 模型可以用一句话总结：

**Package 是唯一源码发布对象，Product Package 表达产品默认意图，Instance 表达当前环境中的运行事实。**

这样做的收益是：

- 少一层源码对象
- 开源发布模型统一
- source authority 简化
- 产品声明与运行态边界更清晰
- capability / package / feature / generation 的关系更容易维护

同时它保住了原来 App 模型中真正有价值的部分，只是把这些价值从“独立源码层 App”收敛成了：

- product package
- instance

这也是 WEFT vNext 从 App-centric 走向 Package-centric、但仍保持产品级运行边界的收敛方向。
