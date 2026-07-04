# WEFT-plug 架构全景

## 概述

WEFT-plug 是一个多组件 AI 助手框架，核心是 Rust HTTP 服务器，外围是 WASM 插件、Python/Node.js 服务进程、Electron 桌面前端。系统通过 "产品 → 能力 → 绑定 → 插件" 的层次化配置实现模块化组合。

## 架构分层

```
┌─────────────────────────────────────────────────┐
│  Desktop UI (Electron)                          │
│  clients/frontend/                              │
├─────────────────────────────────────────────────┤
│  Core HTTP Server (Rust, Axum)                  │
│  core/src/                                      │
│  ┌──────────┐ ┌──────────┐ ┌────────────────┐  │
│  │ Pipeline │ │  Config  │ │ Process Mgr    │  │
│  │ (LLM)    │ │  System  │ │ (Service Mgmt) │  │
│  ├──────────┤ ├──────────┤ ├────────────────┤  │
│  │ Plugin   │ │  RPC     │ │ API Routes     │  │
│  │ Host     │ │  (stdio) │ │ (HTTP/WS)      │  │
│  └──────────┘ └──────────┘ └────────────────┘  │
├─────────────────────────────────────────────────┤
│  WASM Plugins (Extism)                          │
│  skills-runtime, mcp-client, agent-core, etc.   │
├─────────────────────────────────────────────────┤
│  Service Plugins (Python/Node.js)               │
│  companion-core, memory-runtime, context-engine │
│  js-extension-runtime, generic-agent-runtime    │
└─────────────────────────────────────────────────┘
```

## 关键概念

| 概念 | 说明 |
|------|------|
| **Product (产品)** | weft-claw / weft-code，定义需要哪些能力、绑定哪些插件 |
| **Capability (能力)** | 抽象功能槽位，如 `ext.skills`、`agent.runtime` |
| **Binding (绑定)** | 将 capability 映射到具体 provider 插件名 |
| **Plugin (插件)** | WASM 或 Service 进程，提供具体功能实现 |
| **Suite (套餐)** | 产品 + 绑定 + features 的完整配置快照 |
| **Scene (场景)** | 同一产品下的不同配置变体（如 `team` / `solo`） |

## 通信通道总览

| 通道 | 协议 | 端点 | 用途 |
|------|------|------|------|
| Plugin WebSocket | WS | `/ws/plugins/{name}` | 前端 ↔ WASM 插件 RPC |
| Plugin Call | HTTP POST | `/api/plugins/{name}/call` | 服务进程 ↔ WASM 插件 |
| Plugin Webhook | HTTP POST | `/api/plugins/{name}/webhook` | 外部事件 → WASM 插件 |
| Service Webhook | HTTP POST | `/api/services/{name}/webhook` | 前端/服务 ↔ 服务进程代理 |
| OpenAI Chat | HTTP POST | `/v1/chat/completions` | LLM API 兼容端点 |
| JSON-RPC | stdio | 逐行 JSON | 编辑器集成 (weft-rpc) |
| Electron IPC | Named Pipe | `\\.\pipe\weft-*` | 桌面前端 ↔ Core 进程 |

## 用户输入 → AI 工具调用 完整路径

```
用户输入 (自然语言)
  ↓
Desktop UI (Electron renderer)
  ↓ Electron IPC
WeftCoreApiClient.companionHandleUserTurn()
  ↓
WeftCorePluginClient.call("companion_runtime", "handle_user_turn", ...)
  ↓ resolveBoundPlugin("companion_runtime") → "companion-core"
  ↓ isServicePlugin → true
  ↓
POST /api/services/companion-core/webhook
  ↓ core 代理转发
companion-core/server.py → handle_user_turn()
  ↓ LLM 判断 direct/delegate
  ↓ delegate → agent_runtime
  ↓
agent-core → skills.execute_tool("web_search", args)
  ↓
skills-runtime → do_web_search() 或 mcp-client.call_tool()
  ↓
结果原路返回 → companion → UI
```

## 文档索引

| 文档 | 内容 |
|------|------|
| [01-core-runtime.md](01-core-runtime.md) | Core Rust 运行时：启动、路由、管道、RPC |
| [02-plugin-system.md](02-plugin-system.md) | 插件体系：WASM、Service、Feature、Product |
| [03-communication-layer.md](03-communication-layer.md) | 前端与通信层：通道矩阵、绑定解析、调用决策 |
| [04-tool-skill-execution.md](04-tool-skill-execution.md) | 工具与技能执行：builtin、MCP、JS runtime、evolved skills |
| [05-config-deployment.md](05-config-deployment.md) | 配置、部署、启动：config、CLI、profile、SDK |
