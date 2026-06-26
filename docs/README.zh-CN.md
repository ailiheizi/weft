# Weft

**模块化 AI Agent 平台。** 一个 Rust 核心运行时对外暴露 OpenAI 兼容 API 与一套
基于能力(capability)的插件系统;行为由 WASM 与原生**包(package)**扩展;再由一个
跨平台 Flutter 桌面客户端统一驱动——多 Agent 团队编排、持久记忆、工具调用、技能、
一块 AI 视频剪辑画布,以及一个 AI 增强的 RSS 阅读器。

```
Flutter 桌面客户端  ⇄  weft-core(Rust,OpenAI 兼容 API)  ⇄  包(WASM / 原生 / 内嵌)
```

![Weft 桌面客户端](images/chat-auto-tool-selection.png)

**[English](../README.md) | 简体中文**

> [!WARNING]
> **Weft 处于早期阶段,正在活跃开发中,尚不稳定。** 它正处在一次打包体系
> 重构的中途。各类 API(HTTP 端点、capability 契约、`packages/index.toml`)可能在
> 不另行通知的情况下发生变化,部分包仍是实验性的,你应当预期会遇到粗糙之处。
> 请把它当作用于探索的预览版,而不是生产依赖。

---

## 下载

最快的体验方式是从 [**Releases**](https://github.com/ailiheizi/weft/releases)
页面下载预编译的桌面整合包——它内置了客户端与 `weft-core` 边车进程,无需单独
安装、也不需要构建工具链即可启动运行。首次启动时添加一个 AI provider 即可开始使用。

想从源码构建?见 [构建](#构建)。

---

## 目录

- [Weft](#weft)
  - [目录](#目录)
  - [这是什么 / 定位](#这是什么--定位)
    - [一块试验田,不止是应用](#一块试验田不止是应用)
  - [下载](#下载)
  - [核心竞争力](#核心竞争力)
  - [功能总览](#功能总览)
  - [功能展示](#功能展示)
    - [聊天 + 自动选择工具](#聊天--自动选择工具)
    - [工具选择器](#工具选择器)
    - [AI 增强的 RSS 阅读器](#ai-增强的-rss-阅读器)
    - [包管理](#包管理)
    - [常驻服务管理](#常驻服务管理)
    - [多 Agent 编排(详见文档)](#多-agent-编排详见文档)
    - [AI Director — AI 视频剪辑画布(详见文档)](#ai-director--ai-视频剪辑画布详见文档)
  - [架构](#架构)
  - [能力系统](#能力系统)
  - [官方包全家福](#官方包全家福)
  - [仓库结构](#仓库结构)
  - [构建](#构建)
    - [核心(Rust)](#核心rust)
    - [客户端(Flutter)](#客户端flutter)
  - [运行](#运行)
  - [配置](#配置)
  - [文档](#文档)
  - [路线图](#路线图)
  - [持续集成](#持续集成)
  - [许可证](#许可证)

---

## 这是什么 / 定位

Weft 不是"又一个套壳聊天应用"。它是一个把"AI Agent 能做的所有事"都拆成**可插拔能力**
的运行时平台,核心理念只有一句:

> **一切皆 capability,产品靠声明式组装。**

核心(`weft-core`)本身只做最小的事——把请求按 OpenAI 兼容协议路由到各家大模型。
其余所有能力(Agent 运行时、记忆、工具、技能、MCP、团队编排、视频渲染……)都以
**包**的形式存在,在 `packages/index.toml` 里声明"谁提供什么能力",由核心在启动时
解析、绑定、分发。这意味着:

- **换实现不用改代码。** 想换一套记忆引擎?改 `index.toml` 里 `memory.store` 的绑定即可,
  上层产品一行不动。
- **产品是能力的编排,而非硬编码。** `weft-claw`(多角色 AI 开发助手)的"源码"本质上
  就是一份"我需要这些 capability"的清单,核心负责把它们拼起来。

### 一块试验田,不止是应用

Weft 被设计成一块**试验田(experiment platform)**:它既可以作为面向用户的桌面应用直接使用,
也可以作为底层运行时被嵌入到其他场景——包括**硬件**。因为:

- 核心是一个**单一进程、回环 HTTP 服务**(默认 `127.0.0.1:17830`),任何能发 HTTP 的东西
  都能驱动它——脚本、其他应用、嵌入式设备上的控制程序。
- 能力以 **WASM 沙箱**为主,可移植、可隔离,适合在资源受限或安全敏感的环境里跑。
- 工具选择用的是**本地、离线、无需 GPU** 的语义模型(见下),天然适合边缘/硬件部署。

换句话说:今天它是你桌面上的一个 AI 工作台;明天同一个核心可以是一台硬件设备里的
"AI 大脑"。这正是 Weft 想验证的方向。

---

## 核心竞争力

| 能力 | 说明 | 为什么重要 |
|---|---|---|
| **一切皆 capability** | 每个能力是一个稳定字符串 id(如 `agent.runtime`、`tool.shell`),包提供它、应用声明需要它 | 解耦到极致;换实现/加功能只动配置 |
| **声明式组装产品** | 产品 = 一份 capability 需求清单,核心在启动时解析成具体绑定 | 同一套运行时拼出完全不同的产品 |
| **本地语义选工具** | `tool-selector` 用 ONNX INT8 余弦相似度匹配,5–50ms/次,**无需 GPU、可离线** | 工具多了也能精准路由;适合边缘/硬件 |
| **三种包运行时** | WASM(Extism 沙箱)/ 原生 service / 内嵌,统一在 `index.toml` 声明 | 安全、可移植与性能之间自由取舍 |
| **一个 API,多家模型** | OpenAI 兼容核心,路由到 OpenAI / Anthropic / DeepSeek / OpenRouter 等,带 key 轮换与故障转移 | 上层完全不感知后端切换 |
| **多 Agent 团队 + DAG 编排** | 一个目标拆成 `depends_on` 依赖图,分派给角色化 Agent,输出实时回流 | 复杂任务并行协作 |
| **客户端零业务硬编码** | 桌面端不写死任何"应用"逻辑,全由核心下发 app surface 渲染 | 装新产品包即出现新界面,无需重新打包客户端 |
| **MCP 一等公民** | 注册任意 [MCP](https://modelcontextprotocol.io/) 服务器,其工具自动暴露给 Agent | 即插即用接入海量外部工具 |

---

## 功能总览

桌面客户端是一个带左侧导航栏的 **app shell**,下面每一项都是真实存在的界面或由包提供的
app surface。完整逐屏讲解见 **[docs/FEATURES.md](FEATURES.md)**。

| 功能 | 入口 | 一句话 |
|---|---|---|
| 仪表盘 | `/dashboard` | 连接状态、统计、应用启动器 |
| 聊天(自动选工具) | `/chat` | 流式、多会话、Markdown、产物面板,自动挑工具 |
| 工具调用 | 聊天内 | shell / 文件 / Web / git,统一契约 + 语义路由 |
| 多 Agent 编排 | `/orchestration` | 目标 → DAG → 角色化 Agent 并行 → 实时流 |
| AI Director 画布 | `/apps/ai-director` | 无限画布 + AI 自动生成节点图的视频剪辑 |
| RSS 阅读器 | app surface | 订阅/抓取/AI 摘要 + 划词翻译 + 划词问 AI |
| 包管理 | `/packages` | 浏览/在线安装/本地导入/配置/启停 |
| 常驻服务管理 | `/services` | 启停/重启长驻服务(记忆、上下文引擎…) |
| Provider 配置 | `/providers` | 多家模型、多 key、轮换/故障转移 |
| 设置 | `/settings` | 客户端偏好(动画、视觉选项…) |
| 应用 surface | `/apps/:name` | 核心下发、客户端动态渲染的产品界面 |

---

## 功能展示

### 聊天 + 自动选择工具

流式、多会话的对话工作台,渲染富 Markdown(代码块、表格、列表),并把对话过程中产生的
**产物(artifact)**收进侧边面板。最关键的是:Weft 会在每一轮**自动为任务挑选合适的工具**
(shell / 文件 / Web / git),背后由一个**本地语义工具选择器**(ONNX)对候选工具打分排序——
一条消息就能触发"先搜网页、再写文件、最后跑命令"的组合,你不需要手动接线。

不同工具在聊天里有**专属富气泡**展示:网页搜索气泡、文件读写气泡、终端执行气泡,各自高亮
关键信息;未登记的工具或 MCP 工具(`mcp:server:tool`)则自动回退到通用气泡,并生成
人性化的友好名(如 `run_python` → `Run Python`)。

### 工具选择器

直观查看并控制每一轮对话能触及哪些工具,也可以交给语义选择器自动路由到最合适的那个。
想要更多工具?安装新包,或注册一个 MCP 服务器即可。

### AI 增强的 RSS 阅读器

一个真正接入运行时的 RSS 阅读器包(`rss.reader`):订阅、抓取、解析 RSS/Atom,并能调用
大模型做**AI 摘要**。它有一个专门的**看论文视图**,并且全程 AI 增强:

- **划词翻译**——原地把整篇外文文章翻译成你的语言,外文资料和论文不用离开阅读器就能读。
- **划词问 AI**——选中任意段落直接问 AI(解释这段、总结、这个术语什么意思),把阅读器
  变成研究助手而不是被动的信息流。
- **自动推荐新文章**——基于你的阅读偏好,自动从订阅源里筛选并推荐值得读的新文章
  (`recommend_articles`),帮你从信息洪流里捞出真正相关的内容。

![RSS 阅读器](images/rss-reader.png)

### 包管理

浏览已安装的包、从远程源在线安装、从磁盘导入本地包、逐包配置参数、启用/停用——
全程无需重新编译客户端。包可以是 WASM(Extism 沙箱)、原生或内嵌,它们在
`packages/index.toml` 里声明自己提供的能力,这份索引就是核心启动时读取的"权威来源"。

![包管理](images/package-manager.png)

### 常驻服务管理

部分包以长驻**服务(service)**形式运行(例如记忆运行时、上下文引擎、浏览器 surface、
语义工具选择器),而不是每次调用临时拉起的 WASM。这个界面就是它们的控制台:列出核心
管理的所有服务及其状态,单独启动 / 停止 / 重启,实时刷新——让 Weft 里"常开"的那部分
(持久记忆、后台上下文摄取、定时任务)变得可观察、可控制。

### 多 Agent 编排(详见文档)

编排界面把一个高层目标变成一支协作的 **Agent 团队**:你输入目标 → 工作流编排器提出计划
并拆成并行子任务 → 子任务组成 `depends_on` 有向无环图(独立分支并行、依赖步骤等待)→
分派给角色化 Agent(planner / executor / reviewer……)→ 每个 Agent 的输出实时回流到界面。
任务看板协调跨角色交接(handoff),编排器对每步结果做校验,Agent 还能自行把大任务继续
拆分、或中途暂停让你在多个选项里做选择。完整细节见 [docs/FEATURES.md](FEATURES.md)。

### Weft Claw — 多角色 AI 开发助手

从同一套运行时组装出来的代码助手:你描述想做什么,它自己选工具、写代码、跑测试——
每一步都摊开来给你看,而不是一个黑盒。它是用 Weft 的 agent、工具、运行时能力**声明式
组装**出来的产品,不是另一套独立代码库。

### AI Director — AI 视频剪辑画布

一个构建在运行时之上的"会学习你风格"的 AI 视频剪辑助手,以**无限画布**呈现:AI **自动
生成节点图(DAG)**——把镜头、剪辑、依赖关系铺成画布上的节点并自动连边,而不是线性时间轴;
每个节点是一个镜头或操作,镜头库面板把可复用片段喂进图里;一个专属的 Director 聊天面板
用对话驱动整块画布;底层由 `ffmpeg-runtime`(视频渲染)和 `image-gen`(图像生成)支撑。
这是从同一套运行时长出的另一个产品,印证"一套架构,长出万物"。

![AI Director](images/ai-director.png)

> **多 Agent 编排**的完整细节见 [docs/FEATURES.md](FEATURES.md)。

---

## 架构

Weft 分三层:**客户端**、**核心运行时**、核心加载的**包**。

```
┌─────────────────────────────┐
│  weft_client (Flutter)       │   Windows / macOS / Linux 桌面 UI
│  聊天 · 团队 · DAG 视图 ·    │
│  应用 · 包 · 配置            │
└──────────────┬──────────────┘
               │  HTTP(OpenAI 兼容 + 管理 API)
               │  127.0.0.1:17830(回环,token 守卫)
┌──────────────▼──────────────┐
│  weft-core (Rust 服务)       │
│                              │
│  • OpenAI 兼容 API           │  /v1/chat/completions, /v1/models
│  • 管理 API                  │  /api/apps, /api/capabilities, /api/packages…
│  • Provider 路由             │  多家模型 + key 故障转移/轮换
│  • 能力注册表                │  绑定 capability id → 提供方
│  • 包加载器(Extism)        │  加载 WASM + 原生包
│  • 管道(Pipeline)          │  请求 → 变换 → 提供方 → 响应
└──────────────┬──────────────┘
               │  capability 调用
┌──────────────▼──────────────┐
│  包(Packages)              │
│  agent-runtime · memory ·    │  WASM(Extism)或原生,声明在
│  tools · skills · mcp ·      │  packages/index.toml(权威来源)
│  workflow · team-runtime …   │
└─────────────────────────────┘
```

**客户端 ⇄ 核心**是进程间的回环 HTTP 调用,所以同一个核心既能服务桌面客户端,也能服务
脚本或任何 OpenAI 兼容工具。核心在**运行时加载**能力包,而不是把它们编译进去。

完整深潜见 **[docs/ARCHITECTURE.md](ARCHITECTURE.md)**。

---

## 能力系统

核心除了"裸的大模型路由"之外能做的一切,都被建模成一个**能力(capability)**——一个稳定的
字符串 id(如 `agent.runtime`、`memory.store`、`tool.shell`、`workflow.orchestration`),
由某个包*提供(provides)*、被某个应用*需要(requires)*。

`packages/index.toml` 是**权威来源(source authority)**:它把每个 capability 映射到提供它的包、
包的种类(`provider` / `product` / `foundation`)以及运行时(`wasm` / `native` / `embedded` /
`service`),并记录信任元数据(ed25519 签名、公钥、来源)。

核心在启动时:

1. 读取 `index.toml`,构建能力注册表。
2. 把每个应用需要的 capability 解析成具体的提供方(**bindings**)。
3. 运行时把 capability 调用分发到绑定的包。

因为绑定是声明式的,一个产品本质上就是一份"我需要这些能力"的清单——没有任何提供方被硬编码。

---

## 官方包全家福

`packages/official/` 下共 **31 个**官方包,按家族分组如下。完整的 provides/requires 对照表见
[能力参考](FEATURES.md#capabilities-reference)。

**Agent 与编排**

| 包 | 提供能力 | 角色 |
|---|---|---|
| `agent-core` | `agent.runtime`, `team.delegate` | Agent 轮次、会话感知对话、工具分发 |
| `workflow-orchestrator` | `workflow.orchestration` | 任务提案、校验、DAG 步骤编排 |
| `team-runtime` | `team.runtime`, `team.role.catalog`, `team.context.shared` | 团队角色、共享上下文、委派路由 |
| `team-task-board` | `team.taskboard`, `team.handoff` | 任务看板 + 跨角色交接 |
| `generic-agent-runtime` | `generic_agent.plan/run/verify/crystallize` | 实验性的自演化任务运行时 |
| `workflow-template-devteam` / `workflow-template-creative` | 工作流模板 | 预置的研发/创意团队工作流 |

**记忆与上下文**

| 包 | 提供能力 | 角色 |
|---|---|---|
| `memory` / `memory-runtime` | `memory.store`, `memory.runtime`, `memory.curated` | 跨会话的持久化、精炼记忆 |
| `context-engine` | `context.engine`, `context.ingest`, `context.match` | 摄取信号,主动推荐技能 |
| `prompt-system` | `prompt.system` | 系统提示词管理 |
| `session-events` | `session.events` | 会话生命周期事件 |

**工具与扩展**

| 包 | 提供能力 | 角色 |
|---|---|---|
| `tool-runtime-core` | `tool.runtime` | 统一的工具分发契约 |
| `tool-shell` / `tool-files` / `tool-web` / `tool-git` | `tool.shell/files/web/git` | 内置工具 |
| `tool-selector` | `tool.selector` | 语义工具路由(ONNX,本地推理) |
| `tool-browser` | `tool.browser` | 经 chrome-devtools-mcp 的浏览器自动化 |
| `skills` | `ext.skills`, `skills.evolution/governance/review/maintenance` | 技能发现、加载、执行 |
| `mcp-client` | `ext.mcp` | 注册 MCP 服务器,暴露其工具 |
| `js-extension-runtime` | `extension.runtime.js`, `skill.discovery` | 运行 JavaScript 扩展 |
| `channels` | `channel.bridge` | 通道桥接与路由 |
| `cron` | `scheduler.cron`, `maintenance.tick` | 定时任务与维护 tick |

**产品与媒体**

| 包 | 提供能力 | 角色 |
|---|---|---|
| `weft-claw` | `weft_claw.turn`, `ui.surface` | 多角色 AI 开发助手(产品) |
| `ai-director` | `director.plan`, `director.turn` | 会学习风格的 AI 视频剪辑助手(产品) |
| `rss-reader` | `rss.reader` | AI 增强的 RSS 阅读器(翻译、摘要、问 AI) |
| `ffmpeg-runtime` | `video.render` | 视频渲染后端 |
| `image-gen` | `image.generate` | 图像生成 |
| `ai-workspace-browser` | `browser.window`, `browser.tabs`, `browser.context.dom`, … | 内嵌工作区浏览器,支持 grounded 操作 |
| `creative-role-catalog` | `team.role.catalog` | 创意团队角色 |

> 由**核心自身**(而非某个包)提供的能力包括 `core.execution`(命令执行,带 dry-run 守卫)
> 和 `core.files`(工作区文件访问)。

---

## 仓库结构

```
core/                 Rust 核心运行时(weft-core, weft, weft-sign, weft-rpc)
crates/               配套 crate(weft-code-runtime)
packages/
  sdk/                所有包的公共 SDK(path 依赖)
  official/           31 个官方包
  installed/          已安装包的元数据(清单;wasm 单独构建)
  index.toml          权威来源:capability → 包 映射
  weft-code/          产品声明
clients/
  weft_client/        Flutter 桌面客户端(Windows/macOS/Linux)
docs/                 功能讲解、架构、截图
config.example.toml   Provider 配置模板
.github/workflows/    CI + 发布
scripts/              构建辅助(如 build-wasm-packages.sh)
```

---

## 构建

### 核心(Rust)

需要稳定版 Rust 工具链。

```bash
cargo build --workspace --release
```

产出 `weft-core`(服务)外加 `weft`、`weft-sign`、`weft-rpc`。

WASM 包需针对 `wasm32-wasip1` 目标单独构建:

```bash
cargo build -p <包名> --target wasm32-wasip1 --release
```

### 客户端(Flutter)

需要 Flutter SDK(`>=3.12`)。

```bash
cd clients/weft_client
flutter pub get
flutter run -d windows   # 或:macos / linux
```

---

## 运行

```bash
# 1. 配置 provider
cp config.example.toml config/config.toml
#    编辑 config/config.toml,填入你的 provider API key

# 2. 启动核心(默认监听 127.0.0.1:17830)
cargo run --release --bin weft-core

# 3. 启动桌面客户端(自动连接核心)
cd clients/weft_client && flutter run -d windows
```

---

## 配置

把 `config.example.toml` 复制为 `config/config.toml` 并填入 provider key:

```toml
[core]
host = "127.0.0.1"
port = 17830

[[providers]]
# name, base_url, api keys, ...
```

`config/config.toml` 已被 gitignore——仓库里只提交示例模板。

---

## 文档

- **[docs/FEATURES.md](FEATURES.md)** —— 逐屏、逐功能的完整讲解,含截图与完整能力参考表。
- **[docs/ARCHITECTURE.md](ARCHITECTURE.md)** —— 客户端、核心、包如何协同:请求管道、
  能力解析、包运行时与管理 API。

> 深度文档目前为英文版;中文 README 已覆盖核心内容,中文深度文档在路线图中。

---

## 路线图

Weft 是一块持续演进的试验田,以下方向按大致优先级排列(可能调整):

- **稳定 API 契约**:冻结 HTTP 端点与 capability 契约,给出版本化保证。
- **打包体系收尾**:完成包系统的结构整理与索引规整。
- **包市场 / 分发**:在线安装的源、签名校验、版本与升级策略。
- **更多 Provider 与路由策略**:更丰富的故障转移、成本/延迟感知路由。
- **打包发布**:Windows / macOS / Linux 的开箱即用安装包与 release 产物。
- **硬件 / 边缘场景验证**:把核心作为嵌入式"AI 大脑"跑在设备上(本地工具选择器已为此铺路)。
- **中文深度文档**:FEATURES / ARCHITECTURE 的中文版。

---

## 持续集成

为节省 GitHub Actions 额度，工作流不会在每次 push 时触发：

- **CI**（[`.github/workflows/ci.yml`](../.github/workflows/ci.yml)）—— 手动触发。
  构建并测试 Rust 核心（`cargo build --workspace`、`cargo test`），并对 Flutter
  客户端跑 `flutter pub get` 与 `flutter analyze`。
- **发布**（[`.github/workflows/release.yml`](../.github/workflows/release.yml)）
  —— 在 `v*` 标签（或手动触发）时运行：构建核心二进制、WASM 包与打包好的桌面
  应用，并发布为 GitHub Release。

---

## 关于

Weft 由一名在深圳的自学开发者独立打造。我一直相信的一句话:技术不再是壁垒,判断才是
关键。架构和取舍由我来定,实现大量用 AI 提速。Weft 就是我的一次实践——一个每层都能换的
AI 平台,让同一套运行时长成许多不同的产品。

**我正在找工作** —— 后端 / 系统 / AI 工程 / 全栈。

📮 ailiheizi@gmail.com · 🌐 [me.alhz.org](https://me.alhz.org)

---

## 许可证

基于 [Apache License 2.0](../LICENSE) 授权。

Weft 构建于诸多优秀的开源项目之上,包括 [Extism](https://extism.org/)(WASM 插件
运行时)、[ONNX Runtime](https://onnxruntime.ai/)(本地语义工具选择)与
[Flutter](https://flutter.dev/)(桌面客户端)。完整清单见各包的 manifest 以及
依赖清单(`Cargo.toml`、`pubspec.yaml`)。
