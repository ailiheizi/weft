# 插件体系架构

## 双运行时

WEFT-plug 有两种插件运行时：

| 运行时 | 格式 | 通信方式 | 示例 |
|--------|------|---------|------|
| **WASM** | `.wasm` (Extism) | WebSocket / host 函数 | mcp-client, agent-core |
| **Service** | Python/Node.js 进程 | HTTP webhook (core 代理) | companion-core, memory-runtime, context-engine, js-extension-runtime |

## 插件类型分类

### 1. Core Platform (WASM Foundation)
这些是平台基础，由 Rust 编译为 WASM：

| 插件名 | provides | 用途 |
|--------|----------|------|
| mcp-client | ext.mcp | MCP 服务器管理与工具调用 |
| agent-core | agent.runtime, team.delegate | 代理运行时调度 |
| prompt-system | prompt.system | Prompt 管理 |
| channel-core | channel.bridge | 通道桥接 |
| cron | scheduler | 定时任务 |

### 2. JS Plugin
通过 JS 运行时加载，不需要编译为 WASM：

| 插件名 | provides | 用途 |
|--------|----------|------|
| skills | ext.skills, tool.web, web.search, web.fetch | 技能注册与执行 |

### 3. WASM Provider
提供具体功能实现：

| 插件名 | provides | 用途 |
|--------|----------|------|
| tool-runtime-core | tool.runtime | 工具运行时 |
| tool-shell | tool.shell | Shell 命令（官方 stub，尚未出现在 `installed/`） |
| tool-files | tool.files | 文件操作（官方 stub，尚未出现在 `installed/`） |
| tool-web | tool.web | Web 工具 |
| tool-git | tool.git | Git 操作（官方 stub，尚未出现在 `installed/`） |
| workflow-orchestrator | workflow.orchestration | 工作流编排 |

### 4. Service (独立进程)
Python/Node.js 进程，由 core 管理生命周期：

| 服务名 | 端口 | 用途 |
|--------|------|------|
| companion-core | 43127 | 伴侣规划器 (direct/delegate 决策) |
| memory-runtime | 43129 | Hermes 风格持久化记忆 |
| context-engine | 动态 | 上下文引擎 |
| generic-agent-runtime | 43133 | 自进化任务运行时代理 |
| js-extension-runtime | 动态 | JS 扩展工具运行时 |
| weft-code-runtime | 动态 | WEFT-Code 运行时 |

### 4. Feature Bundles
不提供能力，只组合其他插件：

| Feature | 包含插件 |
|---------|---------|
| weft-claw-chat-feature | agent-runtime, prompt-system, memory-store |
| weft-claw-extensions-feature | skills-runtime, mcp-client |
| weft-claw-channel-feature | channel-core |
| weft-claw-devteam-feature | team-runtime, team-task-board, workflow-template-devteam, ... |
| weft-code-core-feature | agent-runtime, prompt-system, workflow-orchestrator, tool-* |
| weft-code-team-feature | team-runtime, team-task-board, mcp-client, skills-runtime |

### 5. Product
最顶层的产品定义：

| Product | 定位 | 标志性依赖 |
|---------|------|-----------|
| weft-claw | 多角色协作助手 | channel-core, team-runtime, companion-core |
| weft-code | TUI/CLI 代码助手 | weft-code-runtime, tool-shell, tool-git |

## 绑定图 (Capability → Provider)

这是 weft-claw 的核心绑定：

```
agent.runtime      → agent-runtime
prompt.system      → prompt-system
workflow.orchestration → workflow-orchestrator
tool.runtime       → tool-runtime-core
tool.shell         → tool-shell
tool.files         → tool-files
tool.web           → tool-web
tool.git           → tool-git
ext.skills         → skills-runtime
ext.mcp            → mcp-client
memory.store       → memory-store
core.execution     → core
ui.surface         → weft-claw-ui
channel.bridge     → channel-core
team.runtime       → team-runtime
team.taskboard     → team-task-board
team.handoff       → team-task-board
team.delegate      → agent-runtime
team.role.catalog  → team-runtime
team.context.shared → team-runtime
workflow.template.devteam → workflow-template-devteam
companion_runtime  → companion-core
```

## WASM 插件通信

WASM 插件之间通过 host 函数互调：

```
agent-core (WASM)
  ↓ host_call_plugin("skills", "execute_tool", {tool, args})
skills-runtime (WASM) 收到 handle_ws_message
  ↓
  ↓ 内置工具: do_web_search() / do_shell_exec() / do_fs_read() ...
  ↓ MCP 工具: host_call_plugin("mcp-client", "call_tool", {server, tool, args})
  ↓
mcp-client (WASM) 收到 handle_ws_message
  ↓ JSON-RPC 2.0 (stdio 或 HTTP)
外部 MCP 服务器
```

## Service 插件通信

Service 插件通过 core 的 webhook 代理互调：

```
companion-core (Python, port 43127)
  ↓ plugin_call("agent_runtime", "create_session", {...})
  ↓ POST http://127.0.0.1:{core_port}/api/plugins/agent_runtime/call
core → dispatch_plugin_payload → WASM agent-core

context-engine (Python)
  ↓ call_service_plugin("companion-core", "handle_context_event", {...})
  ↓ POST http://127.0.0.1:{core_port}/api/plugins/companion-core/call
core → dispatch_to_service_plugin → POST companion-core/webhook
```

## 端口分配

| 服务 | 端口 |
|------|------|
| weft-core (HTTP) | config.core.port (默认 42617) |
| companion-core | 43127 |
| memory-runtime | 43129 |
| generic-agent-runtime | 43133 |
| registry-server | 3005 |
