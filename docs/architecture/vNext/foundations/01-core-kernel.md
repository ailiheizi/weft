# WEFT vNext 蓝图：Core 作为能力中心与调度中枢

## 一、角色定义

在最新方案中，Core 不再被理解为“尽量薄、尽量把所有事情推给插件”的层，而是被明确定位为：

**能力中心 + 调度中枢 + 验证与切换平台**

也就是说，Core 必须足够强，才能支撑：

- WASM 作为主扩展路线
- AI 对 Product Package 与 Instance 的持续改写
- Native 扩展的有限接入
- generation 的稳定切换

Core 不是某个 product package 的业务实现本体，但它是所有产品实例运行的基础平台。

## 二、Core 的核心职责

### 1. 提供宿主能力

Core 应尽可能多地提供宿主能力，例如：

- 文件操作
- 工作区操作
- 命令执行
- 外部通信
- 进程/插件状态管理
- 校验与日志

这些能力不应要求每次都让 Product Package 或 Package 直接对接宿主系统。

### 2. 维护 Capability Registry

Core 内部维护 capability registry，用于记录：

- 当前有哪些 capability
- 哪些 Package 提供这些 capability
- 当前 product package / instance 绑定到了哪个 provider
- 当前 profile 是否允许使用
- 当前 provider 是否可用

registry 是 Core 内部机制，不是独立于 Core 的外部系统。

### 3. 装载 Package

Core 负责装载和管理 Package，主要包括：

- WASM Package
- Native Package / Native Plugin

但两者地位不同：

- WASM 是主路线
- Native 是补充路线

Core 必须为这两类提供统一调度入口。

### 4. 解析 Product Package Graph

Core 负责把以下输入解析成可运行结果：

- Product Package Declaration
- Instance Config
- Instance Lock
- Package 元数据
- Profile / Policy

解析输出是候选 generation，而不是临时推断结果。

### 5. 执行验证与切换

Core 负责：

- 生成 candidate generation
- 校验 candidate generation
- 激活 active generation
- 保留 rollback generation
- 失败时回退

这使得 AI 或开发者对 Product Package / Instance 的内部修改不会直接污染当前稳定版本。

## 三、Core 与 WASM 的关系

WASM 是主扩展路线，但不应被误解为“WASM 拿到所有底层能力自己完成一切”。

Core 与 WASM 的关系应是：

- Core 提供能力
- WASM 通过统一调用方式访问能力
- WASM 负责业务编排、监管和治理逻辑

也就是说，WASM 是主要的**组合层和治理层**，不是宿主底层能力本身。

## 四、Core 与 Native 的关系

Core 允许 Native 扩展接入，但必须明确 Native 是补充路线。

Native 扩展的目标不是取代 WASM，而是解决 WASM 无法覆盖的场景，例如：

- 平台专属 API
- 深系统能力
- 高性能本地能力
- 特殊硬件/工具链对接

Core 必须对 Native 扩展进行：

- 装载控制
- profile/policy 控制
- 生命周期控制
- 与 capability registry 的统一对接

## 五、Core 的统一调用面

当前最新理念下，Core 最关键的统一调用面是 capability 调用模型。

最核心的形式可以理解为：

- `capability.call(name, payload)`

它不是系统全部协议，但它是统一入口。

在这个模型下：

- Package 不直接理解所有宿主细节
- Core 通过 registry 和 binding 把调用路由到 provider
- provider 可以是 WASM，也可以是 Native

所以 Core 的真正价值不是“定义一长串 syscall 表”，而是：

**提供稳定的能力调用协议与路由机制。**

## 六、Core 不负责什么

为了避免膨胀，Core 仍明确不负责以下事情：

- 不负责具体 product package 的业务逻辑
- 不负责 UI 细节
- 不负责把用户暴露给内部 Package 结构
- 不负责让普通用户逐项管理细粒度权限

同时，Core 也不应把“每一个新增业务能力”都写死在自己内部。

Core 负责的是：

- 平台机制
- 能力宿主
- 路由与验证

而不是所有能力的最终业务实现。

## 七、Core 的稳定边界

Core 长期应尽量稳定的对象有：

- Product Package / Instance
- Package
- Binding
- Config
- Lock
- Generation
- Policy
- Capability Registry

未来高频变化的应主要是：

- Package 的具体实现
- Product Package 默认绑定
- Native/WASM 扩展内容
- 验证规则

也就是说，Core 稳定的是“平台协议”，不是“功能细节集合”。

## 八、Core 与前端、AI 的关系

### 1. Core 与前端

前端应只把 Core 当成产品实例平台来调用，例如：

- `/api/apps`
- `/api/apps/{name}`
- `/api/apps/{name}/run`
- `/api/apps/{name}/update`
- `/api/apps/{name}/rollback`

前端不需要知道底层到底用了 WASM 还是 Native。

### 2. Core 与 AI

AI 不是直接改系统运行态，而是通过 Core 工作：

- propose
- resolve
- verify
- activate

Core 负责把 AI 的修改变成可验证、可回滚的候选代。

## 九、设计结论

Core 的使命不是“变得更薄”，而是：

**成为一个足够强、但边界清晰的能力平台，让 WASM 成为主扩展层，让 Native 只在必要时补足能力。**
