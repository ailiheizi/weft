# WEFT vNext 蓝图：声明、配置、锁定三层分离

## 一、为什么必须分层

WEFT vNext 要支持：

- 用户面对稳定产品实例
- Core 能解析并稳定运行 product package instance
- AI 可以修改内部依赖与绑定关系
- 系统可以精确回滚和复现

如果把这些信息都塞在一个文件里，最终会出现以下问题：

- 用户配置和系统解析结果混在一起
- product package 声明目标和当前 instance 状态混在一起
- AI 改完之后无法区分“意图变化”和“结果变化”
- 无法可靠比较两个 generation 的差异

因此 vNext 必须严格分成三层：

1. Declaration
2. Config
3. Lock

## 二、Declaration：表达产品意图

Declaration 用于描述 product package 的结构目标，而不是用户当前设置或系统当前结果。

Declaration 至少应包含：

- product identity
- 需要哪些能力
- 默认绑定哪些 Package
- 哪些绑定可替换，哪些不可替换
- 升级策略
- 验证策略

Declaration 回答的问题是：

**“这个 product package 默认想装成什么样？”**

### Declaration 的特点

- 可被版本控制
- 可被 AI 读写和修改
- 可作为 resolver 的输入
- 不包含环境私密信息
- 不包含实际解析后的精确 hash 结果

### Declaration 不应包含什么

- API key
- 用户本地目录路径
- 当前实际锁定的 Package 内容摘要
- 当前 active generation id

## 三、Config：表达运行时配置

Config 用于描述用户和环境对某个 instance 的运行方式，而不是 package 的结构定义。

Config 至少应包含：

- profile（safe / developer / trusted）
- 默认模型
- workspace 路径
- API key / credential 引用
- 运行时参数
- 开关项和偏好项

Config 回答的问题是：

**“这个产品实例在当前设备和当前用户环境下如何运行？”**

### Config 的特点

- 用户可修改
- 环境相关
- 可能包含密钥引用
- 不应改变 product package 的产品身份
- 改变后可能触发新 generation

### Config 不应包含什么

- 内部 Package 的强绑定 digest
- 不可变解析结果
- 与具体发布内容绑定的 closure 信息

## 四、Lock：表达解析与安装结果

Lock 是系统解析后的不可变结果，它不表达用户意图，只表达：

**“这次实际运行的到底是什么”**

Lock 至少应包含：

- product package 名称
- generation 编号
- 实际解析到的 Package 列表
- 每个 Package 的精确版本
- 每个 Package 的 source
- 每个 Package 的 digest / hash
- 每个 Package 的 manifest digest
- 实际能力绑定结果
- 验证结果

### 为什么必须有 hash

仅有版本号不够，因为：

- 同一版本可能被重新发布
- 本地开发包可能未变版本但内容变化
- AI 修改依赖后需要精确回滚
- 系统需要确认当前运行内容的不可变身份

因此 Lock 中必须保留至少一种强摘要，例如 `sha512`。

这里的 `sha512` 应理解为：

- 解析后生成的精确内容摘要
- 用来锁定“当前实际装载内容”
- 语义上更接近调用库系统中的 lock hash，而不是人工说明字段

也就是说，Lock 里的 hash 不表达“想要什么”，而表达“最终生成出来并被系统接受的内容身份”。

## 五、三层各自的边界

### 1. Declaration 负责“目标”

例如：

- 我需要 `memory.backend`
- 默认用 `memory`
- 允许换成另一个 memory package

### 2. Config 负责“当前运行设置”

例如：

- 当前 profile 是 `developer`
- 当前 workspace 是 `D:/work/project-a`
- 当前默认模型是 `deepseek-chat`

### 3. Lock 负责“实际装了什么”

例如：

- `memory` 解析到了 `0.2.4`
- digest 是 `sha512:...`
- 当前 generation 是 17
- 验证通过

## 六、建议文件结构

新的典型结构建议如下：

```text
packages/
  weft-claw/
    package.toml

%AppData%/WEFT/instances/weft-claw/
  config.toml
  lock.toml
```

### `package.toml`

product package declaration 文件。

### `config.toml`

instance config 文件。

### `lock.toml`

instance lock 文件。

## 七、Declaration 示例，按 Weft Claw 多 Package 装配模型重写

下面的例子不再把 `weft-claw` 写成“几个默认插件”的薄壳，而是明确表达：

- product package 需要哪些 capability
- 默认启用哪些 feature packages
- 哪些 binding 固定，哪些允许 mutable
- feature package 自己也能声明对底层 capability 的默认依赖

```toml
schema_version = 2

[identity]
name = "weft-claw"
version = "0.4.0"
display_name = "Weft Claw"
description = "Assembled assistant product built from capability providers and feature packages"

[package]
kind = "product"

[requires]
capabilities = [
  "agent.runtime",
  "prompt.system",
  "workflow.orchestration",
  "tool.runtime",
  "tool.shell",
  "tool.files",
  "tool.web",
  "tool.git",
  "memory.store",
  "channel.bridge",
  "channel.telegram",
  "ext.mcp",
  "ext.skills",
  "ui.surface"
]

[features]
default_enabled = [
  "weft-claw-chat-feature",
  "weft-claw-automation-feature",
  "weft-claw-extensions-feature"
]
optional = [
  "weft-claw-channel-feature"
]

[feature_bindings."weft-claw-chat-feature"]
requires = ["agent.runtime", "prompt.system", "memory.store"]

[feature_bindings."weft-claw-automation-feature"]
requires = ["workflow.orchestration", "tool.runtime", "tool.shell", "tool.files", "tool.web", "tool.git"]

[feature_bindings."weft-claw-extensions-feature"]
requires = ["ext.mcp", "ext.skills"]

[feature_bindings."weft-claw-channel-feature"]
requires = ["channel.bridge", "channel.telegram"]

[bindings."agent.runtime"]
provider = "agent-runtime"
mutable = false

[bindings."prompt.system"]
provider = "prompt-system"
mutable = false

[bindings."workflow.orchestration"]
provider = "workflow-orchestrator"
mutable = true
allowed = ["workflow-orchestrator"]

[bindings."tool.runtime"]
provider = "tool-runtime-core"
mutable = false

[bindings."tool.shell"]
provider = "tool-shell"
mutable = true
allowed = ["tool-shell"]

[bindings."tool.files"]
provider = "tool-files"
mutable = true
allowed = ["tool-files"]

[bindings."tool.web"]
provider = "tool-web"
mutable = true
allowed = ["tool-web"]

[bindings."tool.git"]
provider = "tool-git"
mutable = true
allowed = ["tool-git"]

[bindings."memory.store"]
provider = "memory-store"
mutable = true
allowed = ["memory-store", "memory-store-sqlite"]

[bindings."channel.bridge"]
provider = "channel-core"
mutable = true
allowed = ["channel-core"]

[bindings."channel.telegram"]
provider = "channel-telegram"
mutable = true
allowed = ["channel-telegram"]

[bindings."ext.mcp"]
provider = "mcp-client"
mutable = true
allowed = ["mcp-client"]

[bindings."ext.skills"]
provider = "skills-runtime"
mutable = true
allowed = ["skills-runtime"]

[bindings."ui.surface"]
provider = "weft-claw-ui"
mutable = false

[validation]
checks = ["boot", "health", "chat-basic", "tools-basic"]
```

## 八、Config 示例，包含 feature enablement 和 binding override

Config 负责表达当前设备和当前用户的运行形态，所以它应该写运行时启用状态和 override，而不是重写产品意图本身。

```toml
schema_version = 2

[app_runtime]
profile = "developer"
workspace = "D:/work/weft-claw"
default_model = "deepseek-chat"

[features]
enabled = [
  "weft-claw-chat-feature",
  "weft-claw-automation-feature",
  "weft-claw-extensions-feature",
  "weft-claw-channel-feature"
]
disabled = []

[feature_config."weft-claw-channel-feature"]
enabled_adapters = ["channel-telegram"]

[feature_config."weft-claw-extensions-feature"]
enable_mcp = true
enable_skills = true

[binding_overrides."memory.store"]
provider = "memory-store-sqlite"
reason = "developer profile uses local sqlite persistence"

[binding_overrides."channel.bridge"]
provider = "channel-core"
reason = "channel feature enabled for telegram bridge"

[binding_overrides."channel.telegram"]
provider = "channel-telegram"
reason = "single-channel rollout for local testing"

[secrets]
llm_api_key_ref = "local://llm/default"
telegram_bot_token_ref = "local://channels/telegram/dev-bot"
```

## 九、Lock 示例，锁定整次产品装配结果

Lock 不只锁 provider package，还要锁 feature package、输入摘要、最终 binding 快照和验证结果。这样 generation 才能表达“这次具体装配出的整个产品实例”。

```toml
lock_version = 2

[product_instance]
name = "weft-claw"
version = "0.4.0"
generation = 42
profile = "developer"

[inputs]
declaration_schema_version = 2
config_schema_version = 2
declaration_digest = "sha512:package-toml-weft-claw-v040"
config_digest = "sha512:instance-config-weft-claw-dev"

[assembly]
enabled_features = [
  "weft-claw-chat-feature",
  "weft-claw-automation-feature",
  "weft-claw-extensions-feature",
  "weft-claw-channel-feature"
]

[[packages]]
name = "weft-claw-chat-feature"
version = "0.4.0"
package_kind = "feature"
source = "registry:official"
digest = "sha512:pkg-weft-claw-chat-feature-040"
manifest_digest = "sha512:manifest-weft-claw-chat-feature-040"
signature = "sigstore:weft-claw-chat-feature@0.4.0"
entry_kind = "wasm"
provides = []
requires = ["agent.runtime", "prompt.system", "memory.store"]

[[packages]]
name = "weft-claw-automation-feature"
version = "0.4.0"
package_kind = "feature"
source = "registry:official"
digest = "sha512:pkg-weft-claw-automation-feature-040"
manifest_digest = "sha512:manifest-weft-claw-automation-feature-040"
signature = "sigstore:weft-claw-automation-feature@0.4.0"
entry_kind = "wasm"
provides = []
requires = ["workflow.orchestration", "tool.runtime", "tool.shell", "tool.files", "tool.web", "tool.git"]

[[packages]]
name = "weft-claw-extensions-feature"
version = "0.4.0"
package_kind = "feature"
source = "registry:official"
digest = "sha512:pkg-weft-claw-extensions-feature-040"
manifest_digest = "sha512:manifest-weft-claw-extensions-feature-040"
signature = "sigstore:weft-claw-extensions-feature@0.4.0"
entry_kind = "wasm"
provides = []
requires = ["ext.mcp", "ext.skills"]

[[packages]]
name = "weft-claw-channel-feature"
version = "0.4.0"
package_kind = "feature"
source = "registry:official"
digest = "sha512:pkg-weft-claw-channel-feature-040"
manifest_digest = "sha512:manifest-weft-claw-channel-feature-040"
signature = "sigstore:weft-claw-channel-feature@0.4.0"
entry_kind = "wasm"
provides = []
requires = ["channel.bridge", "channel.telegram"]

[[packages]]
name = "agent-runtime"
version = "0.4.1"
package_kind = "provider"
source = "registry:official"
digest = "sha512:pkg-agent-runtime-041"
manifest_digest = "sha512:manifest-agent-runtime-041"
signature = "sigstore:agent-runtime@0.4.1"
entry_kind = "wasm"
provides = ["agent.runtime"]

[[packages]]
name = "prompt-system"
version = "0.4.0"
package_kind = "provider"
source = "registry:official"
digest = "sha512:pkg-prompt-system-040"
manifest_digest = "sha512:manifest-prompt-system-040"
signature = "sigstore:prompt-system@0.4.0"
entry_kind = "wasm"
provides = ["prompt.system"]

[[packages]]
name = "workflow-orchestrator"
version = "0.4.0"
package_kind = "provider"
source = "registry:official"
digest = "sha512:pkg-workflow-orchestrator-040"
manifest_digest = "sha512:manifest-workflow-orchestrator-040"
signature = "sigstore:workflow-orchestrator@0.4.0"
entry_kind = "wasm"
provides = ["workflow.orchestration"]

[[packages]]
name = "tool-runtime-core"
version = "0.4.0"
package_kind = "provider"
source = "registry:official"
digest = "sha512:pkg-tool-runtime-core-040"
manifest_digest = "sha512:manifest-tool-runtime-core-040"
signature = "sigstore:tool-runtime-core@0.4.0"
entry_kind = "wasm"
provides = ["tool.runtime"]

[[packages]]
name = "tool-shell"
version = "0.4.0"
package_kind = "provider"
source = "registry:official"
digest = "sha512:pkg-tool-shell-040"
manifest_digest = "sha512:manifest-tool-shell-040"
signature = "sigstore:tool-shell@0.4.0"
entry_kind = "service"
provides = ["tool.shell"]

[[packages]]
name = "tool-files"
version = "0.4.0"
package_kind = "provider"
source = "registry:official"
digest = "sha512:pkg-tool-files-040"
manifest_digest = "sha512:manifest-tool-files-040"
signature = "sigstore:tool-files@0.4.0"
entry_kind = "wasm"
provides = ["tool.files"]

[[packages]]
name = "tool-web"
version = "0.4.0"
package_kind = "provider"
source = "registry:official"
digest = "sha512:pkg-tool-web-040"
manifest_digest = "sha512:manifest-tool-web-040"
signature = "sigstore:tool-web@0.4.0"
entry_kind = "service"
provides = ["tool.web"]

[[packages]]
name = "tool-git"
version = "0.4.0"
package_kind = "provider"
source = "registry:official"
digest = "sha512:pkg-tool-git-040"
manifest_digest = "sha512:manifest-tool-git-040"
signature = "sigstore:tool-git@0.4.0"
entry_kind = "service"
provides = ["tool.git"]

[[packages]]
name = "memory-store-sqlite"
version = "0.4.0"
package_kind = "provider"
source = "registry:community"
digest = "sha512:pkg-memory-store-sqlite-040"
manifest_digest = "sha512:manifest-memory-store-sqlite-040"
signature = "sigstore:memory-store-sqlite@0.4.0"
entry_kind = "native"
provides = ["memory.store"]

[[packages]]
name = "mcp-client"
version = "0.4.0"
package_kind = "provider"
source = "registry:official"
digest = "sha512:pkg-mcp-client-040"
manifest_digest = "sha512:manifest-mcp-client-040"
signature = "sigstore:mcp-client@0.4.0"
entry_kind = "service"
provides = ["ext.mcp"]

[[packages]]
name = "skills-runtime"
version = "0.4.0"
package_kind = "provider"
source = "registry:official"
digest = "sha512:pkg-skills-runtime-040"
manifest_digest = "sha512:manifest-skills-runtime-040"
signature = "sigstore:skills-runtime@0.4.0"
entry_kind = "wasm"
provides = ["ext.skills"]

[[packages]]
name = "channel-core"
version = "0.4.0"
package_kind = "provider"
source = "registry:official"
digest = "sha512:pkg-channel-core-040"
manifest_digest = "sha512:manifest-channel-core-040"
signature = "sigstore:channel-core@0.4.0"
entry_kind = "service"
provides = ["channel.bridge"]

[[packages]]
name = "channel-telegram"
version = "0.4.0"
package_kind = "provider"
source = "registry:official"
digest = "sha512:pkg-channel-telegram-040"
manifest_digest = "sha512:manifest-channel-telegram-040"
signature = "sigstore:channel-telegram@0.4.0"
entry_kind = "service"
provides = ["channel.telegram"]

[[packages]]
name = "weft-claw-ui"
version = "0.4.0"
package_kind = "provider"
source = "registry:official"
digest = "sha512:pkg-weft-claw-ui-040"
manifest_digest = "sha512:manifest-weft-claw-ui-040"
signature = "sigstore:weft-claw-ui@0.4.0"
entry_kind = "native"
provides = ["ui.surface"]

[bindings]
"agent.runtime" = "agent-runtime"
"prompt.system" = "prompt-system"
"workflow.orchestration" = "workflow-orchestrator"
"tool.runtime" = "tool-runtime-core"
"tool.shell" = "tool-shell"
"tool.files" = "tool-files"
"tool.web" = "tool-web"
"tool.git" = "tool-git"
"memory.store" = "memory-store-sqlite"
"ext.mcp" = "mcp-client"
"ext.skills" = "skills-runtime"
"channel.bridge" = "channel-core"
"channel.telegram" = "channel-telegram"
"ui.surface" = "weft-claw-ui"

[binding_sources]
"agent.runtime" = "declaration-default"
"prompt.system" = "declaration-default"
"workflow.orchestration" = "declaration-default"
"tool.runtime" = "declaration-default"
"tool.shell" = "declaration-default"
"tool.files" = "declaration-default"
"tool.web" = "declaration-default"
"tool.git" = "declaration-default"
"memory.store" = "config-override"
"ext.mcp" = "declaration-default"
"ext.skills" = "declaration-default"
"channel.bridge" = "config-override"
"channel.telegram" = "config-override"
"ui.surface" = "declaration-default"

[verification]
boot = "passed"
health = "passed"
chat_basic = "passed"
tools_basic = "passed"
integrity = "passed"
trust = "warning:community-provider-present"
```

## 十、为什么不能把 Lock 当配置

把 Lock 当配置会直接破坏系统设计目标，因为：

- 配置是用户和环境可以编辑的
- Lock 是系统解析后的结果
- 配置要可变，Lock 要稳定
- 配置表达偏好，Lock 表达事实

一旦把两者混在一起，就无法明确区分：

- 用户想要什么
- 系统实际装了什么

## 十一、为什么不能把 Declaration 当 Lock

如果 Declaration 中写精确 digest，也会失去它作为“产品意图”文档的意义。

Declaration 应该允许：

- 升级
- 替换 provider
- 重新解析

而 Lock 的作用正好相反：

- 精确冻结
- 支持复现
- 支持回滚

## 十二、迁移说明，schema 演进默认采用加法兼容

为了让旧 `app.toml`、`app.config.toml`、`app.lock` 与新的 `package.toml`、instance `config.toml`、`lock.toml` 能跟着多 Package 装配模型逐步演进，schema 更新应默认采用加法兼容，而不是破坏式替换。

### 1. 对旧字段做兼容映射，而不是立刻删除

例如从较粗 capability：

- `agent.capability`
- `tool.registry`
- `memory.backend`
- `execution.capability`

迁移到更细 capability：

- `agent.runtime`
- `tool.runtime`
- `tool.shell`
- `tool.files`
- `tool.web`
- `tool.git`
- `memory.store`
- `workflow.orchestration`

第一阶段应允许 resolver 继续读旧字段，并在 resolve 时映射为新 capability 集合，而不是要求所有旧声明一次性重写完成。

### 2. 新结构优先新增快照块，不覆盖旧语义

例如：

- Declaration 新增 `[features]` 与 `[feature_bindings.*]`
- Config 新增 `[features]`、`[feature_config.*]` 与 `[binding_overrides.*]`
- Lock 新增 `[inputs]`、`[assembly]`、`[binding_sources]`

这些字段都属于“把旧语义展开写清”，不是推翻 Declaration / Config / Lock 三层边界本身。

### 3. Lock 要能同时表达旧 generation 事实和新装配细节

也就是说，旧版 lock 只要还能表达 generation、resolved bindings、package digests，就仍然是合法历史记录。新版 lock 只是补充：

- 哪些 feature packages 参与装配
- 哪些 binding 来自 declaration，哪些来自 config override
- 本次 generation 依赖的 declaration/config 输入摘要

### 4. 迁移目标是“平滑扩展”，不是“格式大爆炸”

因此这三层文件的演进原则应明确为：

- 先新增字段
- 再新增 resolver 兼容逻辑
- 最后才考虑淘汰长期废弃字段

这样做的好处是，AI、Core、现有产品实例与未来产品实例可以在多个 generation 周期内逐步完成迁移，不会因为 schema 调整而失去回滚、比较和复现能力。

## 十三、设计结论

Declaration、Config、Lock 三层分离，是 WEFT vNext 能够同时满足产品简洁性、AI 可维护性和系统可回滚性的基础。

一句话总结：

- Declaration 说“应该是什么”
- Config 说“现在怎么跑”
- Lock 说“实际跑的是啥”

## 十四、当前实现状态

当前代码里，Lock 已经包含：

- generation id
- resolved bindings
- package source
- 真实 SHA-512 digest
- `trusted` 标记

其中 SHA-512 现在按生成结果写入 lock，用于表达 package source 的精确身份，语义上接近“依赖锁文件里的生成摘要”。

并且：

- activate 会自动回写 lock
- verify 会追加 integrity / trust 校验结果
- rollback 语义已经和 generation 切换逻辑联动

这意味着 Declaration / Config / Lock 三层关系已经进入实际运行闭环。
