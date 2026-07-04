# WEFT vNext 蓝图：AI 变更工作流

## 一、为什么 AI 工作流仍然重要

在最新方案里，虽然 Core 更强、WASM 更重要、Native 也被明确引入，但 AI 工作流并没有被弱化，反而更关键。

因为系统最终希望达到的效果是：

- 用户通过 AI 轻松自定义功能
- AI 通过修改 Product Package 与 Instance Config 来改变内部能力组合
- 系统仍然可验证、可切换、可回滚

所以 AI 不能直接修改当前运行中的系统，而必须通过结构化工作流推进变化。

## 二、AI 的角色

AI 在系统中不是“直接改 Core 的人”，而是：

**Product Package 默认结构与 Instance 运行配置的提议者。**

它可以：

- 增加或移除 Package
- 调整 mutable binding
- 修改 Instance Config
- 升级内部实现
- 在 WASM / Native 补充之间提出选择建议

但它不能跳过 Core 的解析、验证和切换机制。

## 三、标准工作流

最新蓝图中，AI 工作流仍然保持四步：

1. propose
2. resolve
3. verify
4. activate

## 四、Step 1：Propose

AI 首先生成变更提议，而不是直接修改 active generation。

提议可能包括：

- 给 Product Package 新增一个 Package
- 调整某个 mutable binding
- 修改默认配置
- 把某个能力从 WASM 实现切到 Native 补充实现
- 调整保活或监管策略

提议的重点不是“直接生效”，而是把变化结构化表达出来。

## 五、Step 2：Resolve

Core 读取 AI 提议后，执行解析过程：

- 检查变更是否符合 Declaration 约束
- 检查当前 profile 是否允许
- 收集目标 Package
- 重新生成 candidate bindings
- 生成 candidate lock
- 生成 candidate generation

也就是说，AI 不直接决定结果，Core 负责把提议转成候选结果。

## 六、Step 3：Verify

Verify 是 AI 工作流里最关键的一步。

在当前方案中，验证至少应覆盖：

### 1. Boot Validation

- 新 generation 是否可加载
- 关键 Package 是否可装载

### 2. Capability Validation

- 核心 capability 是否仍可调用
- 绑定后的 provider 是否按预期工作

### 3. Product Validation

- 关键用户路径是否可用

例如：

- 对话
- 工具执行
- 记忆读写
- 目标程序是否能被正确拉起或保活

### 当前实现状态

当前代码中，verify 已经具备以下落地能力：

- 元数据验证：
  - bindings 是否存在
  - capabilities 是否声明
  - binding coverage
  - registry coverage
- live probe 验证：
  - 对 core / service 路径执行真实 capability 探测
  - 对已加载的 wasm provider 执行真实 live probe
  - 对未加载的 wasm provider，先尝试临时加载再 probe
  - 如果 wasm source 存在但缺少产物，会先尝试构建再 probe
  - 对仍无法加载的 wasm provider 与 native 当前做受控跳过或失败记录

这意味着 AI 工作流里的 `verify` 已经不是纯结构检查，而是开始具备真实运行态验证语义。

## 七、Step 4：Activate

只有当 candidate generation 验证通过后，Core 才允许激活：

- 将 candidate generation 切换为 active generation
- 保留旧 generation 作为回滚目标
- 记录变更日志和验证结果

激活必须是原子的。

## 八、为什么 AI 不能直接改 active generation

如果 AI 能直接改 active generation：

- 会立刻污染当前用户正在使用的产品实例
- 无法形成稳定回滚点
- 无法清晰比较变更前后差异

所以 AI 必须始终工作在“候选代”上。

## 九、AI 与 WASM / Native 的关系

最新方案下，AI 最重要的不是自己直接实现底层能力，而是：

- 优先组合和修改 WASM Package
- 在 WASM 明显不足时，选择 Native 补充能力
- 通过 Product Package、Binding 与 Instance Config 组织这些能力

也就是说，AI 面向的是：

- Product Package 结构
- Package 选择
- Binding 策略
- 配置与验证

而不是直接面向 Core 内部代码。

## 十、AI 工作流的成功标准

AI 工作流设计成功，不取决于它能不能无限自由，而取决于：

- 能不能安全改 Product Package / Instance
- 能不能让用户感觉自定义很轻松
- 能不能把风险压在 candidate generation 里
- 能不能在失败时稳定回滚

## 十一、设计结论

AI 在 WEFT vNext 中不是直接操作内核的人，而是：

**通过修改 Product Package、Binding、Config 来驱动系统变化的高权限提议者。**

一句话总结：

**AI 负责提议新结构，Core 负责解析与验证，Generation 负责安全切换。**
