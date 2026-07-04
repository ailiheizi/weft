# WEFT vNext 产品蓝图总览

## 一、文档目标

本组文档定义 WEFT 下一代产品架构蓝图。当前这版蓝图不再围绕“尽量把一切都抽象成更细的 runtime 层”展开，而是明确收敛到一个更容易落地的方向：

- Core 仍然是能力中心
- Package 是唯一源码发布单位
- Product Package 承担产品默认意图
- Instance 承担运行态配置与锁定结果
- WASM Package 是主路线
- Native 扩展是少量辅助路线
- AI 主要通过修改 Product Package、Binding、Instance Config 来实现功能定制

这组蓝图要解决的核心问题包括：

- 如何让用户面对稳定产品实例，而不暴露 Package 图、依赖树和内部实现复杂性
- 如何让 Core 提供尽可能多的宿主能力，同时避免因为每个新需求都重写核心
- 如何让 WASM 成为默认扩展方式，并承担编排、监管、保活等职责
- 如何在 WASM 不足时，引入 Native 能力补充，而不让 Native 反过来吞掉整个架构
- 如何让 AI 能安全地调整 Product Package 默认装配与 Instance 运行配置，并完成验证与切换

## 二、当前最终理念

WEFT vNext 现在采用的最新理念是：

**WASM 为主，Core 为能力中心，Native 为补充，Package 为源码单位，Instance 为运行入口。**

这意味着：

- 用户看到和操作的是某个 product package 的 instance
- Core 提供统一能力、装载能力提供者、负责验证与切换
- Package 提供具体能力实现
- WASM Package 是主流扩展方式
- Native Package / Native Plugin 只在 WASM 不足时补足能力

我们不再把系统设计成“先定义大量层，再希望层次本身自动解决复杂度”，而是强调主次关系：

- Core 必须足够强
- WASM 必须足够易用和可控
- Native 必须有限引入

## 三、核心对象

### 1. Core

Core 是系统能力中心和调度中心，负责：

- 提供宿主能力
- 维护 capability registry
- 装载和管理 Package
- 解析 Product Package Graph 与 Instance State
- 执行 binding / resolve / verify / activate
- 管理 profile / policy / lifecycle

Core 不是一个只会转发的薄层，而是完整的平台层。

### 2. Package

Package 是唯一源码层发布对象。它既承载内部实现，也在 `product package` 这一层承载产品默认意图。

Package 负责描述：

- 它提供什么 capability
- 它依赖什么 capability
- 它属于 provider、foundation、feature 还是 product
- 它如何被 Core 装载和验证

当前 Package 主要有两种主流实现形态：

- WASM Package
- Native Package / Native Plugin

其中 WASM 是主路线，Native 是补充路线。

### 3. Instance

Instance 是某个 product package 在当前设备、当前用户、当前项目环境中的运行实例。

Instance 负责描述：

- 当前 profile / policy
- 当前 workspace 与 secret 引用
- 当前 feature override
- 当前 binding override
- 当前 lock 与 generation 状态

### 4. Lock

Lock 是解析后的不可变结果。它记录：

- 实际使用了哪些 Package
- 精确版本与 digest
- 来源
- 绑定结果
- 验证状态
- generation 信息

Lock 用于复现、回滚和审计，不用于表达用户意图。

### 5. Generation

Generation 是某个 instance 某一时刻的完整可运行快照。

凡是以下变化，都会生成新 generation：

- Product Package 声明变化
- Config 变化
- Binding 变化
- Package 版本变化
- Profile 变化

新 generation 必须通过验证，才能被激活。

## 四、主路线与辅助路线

### 1. 主路线：WASM Package

WASM 是默认的扩展方式。它适合：

- AI 自动生成和修改
- 跨平台能力
- 可控扩展
- 业务编排
- 监管与保活逻辑

WASM 不要求自己直接拥有所有宿主能力，它主要通过 Core 提供的 capability 调用接口访问能力。

### 2. 辅助路线：Native 扩展

当 WASM 无法满足某些能力需求时，引入 Native 扩展作为补充。Native 扩展适用于：

- 深度系统集成
- 高性能本地能力
- 平台专属 API
- 特殊硬件或工具链接入

Native 不是默认主路径，只在必要场景下启用。

## 五、WASM 的新定位

在最新蓝图里，WASM 不只是“调用 Core 能力的普通插件”。它还承担更重要的职责：

- 编排能力
- 监管其他插件
- 设置保活与恢复策略
- 承担业务逻辑和 AI 自定义逻辑

也就是说：

- Core 负责提供能力
- Native 负责补足能力
- WASM 负责组织、约束和治理能力

## 六、为什么选择这条路线

这条路线是当前最平衡的结果，因为它同时避免了两种极端：

### 极端一：完全依赖 WASM

问题：

- 最终能力边界不够
- 很多深系统功能不得不不断改 Core

### 极端二：大量 Native 直接吞掉系统

问题：

- 风险过高
- 架构失控
- Product Package / Instance 自定义失去统一模型

现在的方案允许：

- 用 WASM 解决大多数扩展
- 用 Core 提供尽可能多的能力
- 用 Native 只补少数高能力缺口

## 七、目录 taxonomy 与职责划分

从这一版开始，`vNext` 文档不再继续堆在同一层目录里，而是按职责分组，保持 `00-overview.md` 作为唯一入口。

- `foundations/`：基础模型与长期成立的核心对象定义
- `runtime/`：运行态解析、绑定、策略、宿主能力与执行边界
- `workflows/`：AI 和系统侧变更流程
- `products/weft-claw/`：Weft Claw 产品蓝图与矩阵
- `products/weft-claw/devteam/`：Weft Claw DevTeam 子域设计
- `products/weft-code/`：WEFT-Code 产品运行时与 client surface
- `legacy/`：仍保留但不再作为主模型入口的历史文档

这种分法的目标不是改变架构内容，而是把“基础概念、运行机制、工作流、产品蓝图、旧模型参考”分开，避免继续依赖不断增长的扁平编号列表。

## 八、建议阅读路径

建议按“基础模型 -> runtime 机制 -> runtime implementation plan -> 工作流 -> 产品蓝图 -> 旧模型参考”的顺序阅读：

1. `00-overview.md`，总览与核心原则
2. `foundations/01-core-kernel.md`，Core 作为能力中心与调度中枢
3. `foundations/03-package-model.md`，Package 模型与实现形态
4. `foundations/13-product-package-instance-model.md`，Product Package + Instance 收敛模型
5. `runtime/04-declaration-config-lock.md`，Declaration / Config / Lock 三层分离
6. `runtime/05-binding-and-resolution.md`，Binding、resolve 与 capability 调用
7. `runtime/06-profiles-and-policy.md`，安全、开发者、完全信任模式
8. `runtime/16-package-service.md`，Dedicated Package Service boundary proposal
9. `runtime/17-package-service-implementation-plan.md`，Package Service phased delivery plan
10. `runtime/07-execution-and-files.md`，文件、执行与宿主能力暴露方式
11. `workflows/08-ai-change-workflow.md`，AI 的 propose / resolve / verify / activate 工作流
12. `products/weft-claw/09-product-blueprint.md`，Weft Claw 作为首个产品型装配蓝图
13. `products/weft-claw/10-capability-package-matrix.md`，Weft Claw capability package 对照矩阵
14. `products/weft-claw/devteam/11-feature-design.md`，Weft Claw DevTeam feature 设计
15. `products/weft-claw/devteam/12-capability-package-matrix.md`，Weft Claw DevTeam capability package 对照矩阵
16. `products/weft-code/14-runtime-model.md`，WEFT-Code runtime + clients 模型
17. `products/weft-code/15-client-surface.md`，WEFT-Code client surface
18. `legacy/02-app-model.md`，旧 App 模型与收敛说明，只作为迁移参考

如果只想快速建立主线心智，可以先读 1 到 10。需要看具体产品分层时，再进入 `products/` 下的矩阵与子域文档。只有在追溯旧模型来源时，才回看 `legacy/`。

## 九、设计原则总结

当前最终遵循的核心原则是：

**用户面对稳定 instance，Core 提供能力，Package 组织装配，WASM 组织能力，Native 补足能力。**

## 十、当前实现落地状态

截至当前代码状态，vNext 蓝图的核心机制已经落地：

- Package / Instance / Lock / Generation 主模型
- capability registry 与统一 capability 调用入口
- profile / policy enforcement
- core host capabilities（files / execution）
- WASM 主路径与临时加载 probe
- Native loader skeleton 与 native capability routing
- AI 工作流中的 propose / verify / activate / rollback

当前剩余的主要工作，已经不是“架构是否成立”，而是：

- 提供真实 native 动态库示例产物
- 对缺失 wasm 产物的自动构建链路
- 更进一步的生产级安全与供应链加固

同时，server 侧已经开始朝“source authority”方向演进：

- server 不再只做 plugin/suite registry
- 还开始暴露 package source，并为后续收敛 instance 运行态管理预留边界
- 这为后续做成更像 NixOS 的中心源提供了骨架
