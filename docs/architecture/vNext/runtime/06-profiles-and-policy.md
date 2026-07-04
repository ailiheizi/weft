# WEFT vNext 蓝图：运行档位与 Core Policy

## 一、为什么不做细粒度权限弹窗

WEFT 不是手机操作系统，也不是浏览器。

如果把权限设计成：

- 文件读取
- 文件写入
- 文件删除
- 命令执行
- 网络访问
- 子进程调用

然后在运行时逐项弹窗让用户确认，会带来以下问题：

- 普通用户难以理解
- 弹窗过多打断产品使用
- AI 工作流频繁被中断
- 用户最终只会点“全部允许”

因此，vNext 不采用以用户为中心的细粒度权限交互模型，而采用：

**以 Core Policy 为核心的预设运行档位模型。**

## 二、三种标准运行档位

### 1. Safe

面向普通用户的默认模式。

目标：

- 最大化安全性
- 最小化用户理解成本
- 不暴露不必要宿主能力

特点：

- 文件能力受限
- 执行能力受限
- 优先使用受控运行环境
- 验证要求较严格

### 2. Developer

面向开发者和高级用户的日常调试模式。

目标：

- 足够灵活
- 仍保持可控与可审计

特点：

- 工作区操作能力更完整
- 允许 managed execution
- 允许 AI 调整 mutable binding
- 保留 lock / generation / verification

### 3. Trusted

面向完全本地信任场景的高级模式。

目标：

- 减少系统阻拦
- 方便开发和系统集成

特点：

- 更开放的文件能力
- 允许 native execution
- 更宽松的 provider 选择
- 允许更自由的依赖替换

即便在 trusted 模式下，系统也仍保留：

- lock
- audit
- generation rollback

## 三、为什么档位优于细粒度用户权限

档位模型的优势是：

- 用户容易理解
- 产品交互简单
- Core 可统一裁决能力授予
- AI 工作流不会被大量交互打断
- 内部仍可保留细粒度策略实现

也就是说，档位是“对外粗粒度”，策略实现可以“对内细粒度”。

## 四、Policy 的角色

Core Policy 负责决定：

- 当前 profile 下哪些能力可用
- 某个 product package instance 请求的能力最终被授予到什么程度
- 哪些命令能力实现允许启用
- 哪些 binding 虽可替换，但当前模式不允许切换到某些 provider

Policy 的最终作用是：

**让 Package 和 Product Package 只声明需求，而由 Core 决定实际授予。**

## 五、需求声明与最终授权的关系

Product Package / Package 声明的是“需要什么能力”，而不是“我最终能做什么”。

例如一个 product package 可以声明：

```toml
[needs]
capabilities = ["files", "execution", "communication"]
```

但在不同 profile 下，Core 实际授予可能不同：

### Safe

- `files` = 仅限受控工作区
- `execution` = 仅限 managed execution
- `communication` = 受代理和受控出口约束

### Developer

- `files` = 完整工作区读写
- `execution` = 允许 managed shell 与部分工具链
- `communication` = 更广泛调用

### Trusted

- `files` = 更宽松文件访问
- `execution` = 原生执行允许
- `communication` = 基本不做额外限制

## 六、能力组建议

为了避免能力过多、用户难理解，vNext 不建议把能力拆成几十种公开项。建议先收敛为少数几组：

- `state`
- `files`
- `execution`
- `communication`

必要时内部再细分，但不应直接把这种细分暴露给普通用户。

## 七、Profile 对用户的呈现方式

用户侧最好只看到一句话和一个选择，而不是复杂权限矩阵。

例如安装产品实例时：

- 安全运行
- 开发者运行
- 完全信任运行

系统可在详情页提供简短解释：

- 安全运行：适合普通使用
- 开发者运行：适合本地调试和扩展
- 完全信任运行：适合高级本地开发

用户不需要知道每种模式背后启用了哪些细项。

## 八、Policy 与 Binding 的关系

即使 Product Package 声明某个 binding 可替换，Core Policy 仍可以限制最终可用候选。

例如：

- `execution.capability` 允许多个命令包候选
- 但在 Safe 模式下，只能选受控命令包

因此：

- Declaration 决定是否可替换
- Policy 决定当前能替换到哪一步

## 九、Policy 与验证的关系

不同 profile 下，验证强度也可以不同：

### Safe

- 更严格，确保基本功能都能完整通过

### Developer

- 保持核心功能验证，但允许更灵活的试验迭代

### Trusted

- 允许开发者跳过部分非关键验证，但仍建议保留基础校验和回滚能力

## 十、设计结论

WEFT vNext 的权限与能力控制，不应由普通用户逐条选择，而应由 Core Policy 在 Product Profile 之下统一决策。

这套模型的核心是：

- 对用户简单
- 对系统可控
- 对 AI 连续工作友好

一句话总结：

**用户选择模式，Core 决定授予，系统内部保留精细控制。**

## 十一、当前实现状态

当前代码里，Profile / Policy 已经不只是概念，而是直接影响：

- capability 是否允许调用
- resolve 阶段是否接受 capability
- native capability 是否只在 Trusted 下开放
- `core.files` 是否被限制在 instance workspace 内

另外，verify 阶段现在还会追加 provider 完整性校验：

- package source 真实 SHA-512 digest
- lock package 完整性记录

并且运行时 `capability.call(...)` 也会按当前 profile 对 provider 的 trust / signature 做拦截。

也就是说，Policy 不再只是“运行前说明”，而是已经参与到 resolve、call、verify 三个阶段。
