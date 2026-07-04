# WEFT vNext 蓝图：文件、执行与宿主能力暴露

## 一、为什么这一层仍然关键

在最新方案中，Core 仍然被要求提供尽可能多的宿主能力，而不是刻意把自己削成一个过薄的转发层。

这是因为很多产品实例的实际价值最终都会落到几类关键动作上：

- 读写文件
- 操作工作区
- 执行命令
- 调用系统程序
- 与外部服务通信

如果 Core 不把这些能力组织好，结果只会是：

- 不断新增 host function
- 不断扩 SDK wrapper
- 不断把复杂度推回 Core 内部

因此这一层的目标不是逃避宿主能力，而是：

**让宿主能力暴露得强，但暴露方式仍然统一。**

## 二、最新原则

这层现在遵循以下原则：

### 1. Core 继续提供尽可能多的能力

Core 不回避文件、执行、通信这些宿主能力，而是主动提供它们。

### 2. WASM 不直接碰所有宿主细节

WASM 主要通过 capability 调用使用这些能力。

### 3. Native 用于补足能力缺口

当某些能力超出 WASM 舒适区时，由 Native Package / Native Plugin 补足。

### 4. 统一入口比无限新增 syscall 更重要

能力增长不应主要表现为“又多了一批低级 syscall”，而应表现为“又多了一批可绑定 capability”。

## 三、Files 能力

Files 能力是 Core 最基础的宿主能力之一。

它至少应覆盖：

- list directory
- read file
- write file
- patch file
- move / copy / delete
- search in files
- read metadata

这些能力足以支撑大多数 AI 工作流，例如：

- 读代码
- 改配置
- 写文档
- 做重构
- 管理输出产物

Files 能力不要求做成重型 VFS，但必须足够统一。

## 四、Execution 能力

Execution 是另一类关键宿主能力。

它至少应覆盖：

- 单次命令执行
- 长命令执行
- 会话式命令交互
- 调用本地程序
- 返回 stdout/stderr
- 管理 cwd / env / timeout

Execution 能力的意义不只是“能调用 shell”，而是：

**让 Core 能把目标程序和工具链真正跑起来。**

## 五、WASM 在这一层的角色

WASM 在文件与执行层里不是最低层实现者，而是：

- 主要调用者
- 主要编排者
- 监管者
- 保活策略执行者

这意味着 WASM 可以：

- 监控目标程序状态
- 触发启动/停止/重启
- 组合文件与执行能力
- 做恢复逻辑

也就是说，WASM 在这层最重要的职责是：

**治理和编排，而不是直接替代宿主接口。**

## 六、Native 在这一层的角色

当 WASM 不足以完成某些能力时，引入 Native 扩展。

典型场景包括：

- 平台专属 API
- 更深的本地进程控制
- 特殊工具链对接
- 硬件/驱动能力
- 更高性能要求

因此：

- 普通能力优先交给 Core + WASM
- 高能力缺口交给 Native

## 七、为什么不直接把所有东西都写进 Core

虽然 Core 仍然提供尽可能多的能力，但不意味着每个新功能都该直接长成新的低级接口。

正确方向是：

- Core 维护统一 capability 调用协议
- Core 暴露关键宿主能力
- 上层通过 Package 和 Binding 组织这些能力

这使得 Core 仍然很强，但不会无限散成杂乱的 syscall 表。

## 八、Profile 对宿主能力的影响

### 1. Safe

- 文件操作受控
- 执行能力受限
- 更适合普通用户

### 2. Developer

- 文件能力更完整
- 可执行能力更强
- 适合 AI 辅助开发和调试

### 3. Trusted

- 宿主能力最开放
- 允许更自由地使用 Native 扩展
- 适合高级用户和系统集成

这里的关键不是“暴露多少权限项给用户”，而是让用户只选 profile，由 Core 决定能力边界。

## 九、用户看到什么

普通用户不需要知道：

- 底层到底是 WASM 还是 Native
- 某个 execution capability 到底是哪个 provider
- 某个文件操作具体走了哪条实现路径

用户只需要知道：

- 产品实例是否能运行
- 是否需要更高模式
- 升级后是否通过验证

## 十、设计结论

文件和执行能力在 vNext 里并不是被弱化，而是被重新组织：

- Core 负责提供尽可能多的宿主能力
- WASM 负责调用、编排、监管与保活
- Native 负责补足少数高能力缺口

一句话总结：

**宿主能力要强，调用方式要统一，治理逻辑优先交给 WASM。**

## 十一、当前实现状态

当前代码已经落地以下机制：

- `core.files`
  - list / read / write / delete / metadata
- 基于 instance workspace 的路径沙箱
  - 阻止路径穿越和 workspace 外访问
- `core.execution`
  - run command
  - timeout
  - stdout / stderr / exit_code 返回
- `host_capability_call`
  - 允许 WASM 插件通过 capability 名称请求宿主能力
- native capability routing
  - 当 capability 绑定到 native provider 时，Core 可路由到 native loader

因此这层蓝图已经进入可运行状态，而不是停留在抽象设计。
