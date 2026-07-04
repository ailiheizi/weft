# 前端与通信层架构

## 前端架构

```
clients/frontend/src/main/
├── weft-core-api-client.ts     # 主 API 客户端 (5037 行)
├── weft-core-plugin-client.ts  # 插件通信客户端 (215 行)
├── weft-core-host-service.ts   # 主机服务管理
└── services/
    ├── weft-suite-sync-engine.ts   # 套餐同步引擎
    ├── weft-suite-install-engine.ts # 安装引擎
    ├── weft-suite-ai-planner.ts    # AI 规划器
    └── weft-default-plugin-aliases.ts # 默认别名
```

## 通信通道矩阵

### 通道 1: Plugin WebSocket
- **协议**: WebSocket
- **端点**: `ws://{host}:{port}/ws/plugins/{plugin_name}`
- **消息格式**: `{id, type:"request"/"response", payload:{action, data}}`
- **用途**: 前端 ↔ WASM 插件 (如 skills, mcp-client, agent-core)
- **实现**: `WeftCorePluginClient.callViaWebSocket()`
- **特点**: 每次调用开新连接，匹配 id 后关闭

### 通道 2: Plugin Call (HTTP)
- **协议**: HTTP POST
- **端点**: `POST /api/plugins/{plugin_name}/call`
- **消息格式**: `Json<{action, data}>`
- **用途**: 服务进程 ↔ WASM 插件
- **实现**: `companion-core/server.py::plugin_call()`
- **特点**: 每次独立 HTTP 请求

### 通道 3: Plugin Webhook
- **协议**: HTTP POST
- **端点**: `POST /api/plugins/{plugin_name}/webhook[/{channel}]`
- **消息格式**: raw body + headers → `{channel, headers, body}`
- **用途**: 外部事件推送 → WASM 插件
- **实现**: 调用 WASM 的 `handle_webhook` 导出

### 通道 4: Service Webhook
- **协议**: HTTP POST
- **端点**: `POST /api/services/{plugin_name}/webhook`
- **消息格式**: `{action, data}`
- **用途**: 前端/服务 ↔ 服务进程 (companion-core, context-engine 等)
- **实现**: core 透明代理 → 转发到服务的 `/webhook` 端点

### 通道 5: OpenAI Chat
- **协议**: HTTP POST
- **端点**: `POST /v1/chat/completions`
- **用途**: LLM API 直接调用
- **实现**: `WeftCoreApiClient.chatCompletion()`

### 通道 6: Electron IPC (Named Pipe)
- **协议**: Windows Named Pipe
- **端点**: `\\.\pipe\weft-*`
- **用途**: 桌面前端 ↔ Core 进程
- **实现**: `weft-core-host-service.ts`

### 通道 7: JSON-RPC stdio
- **协议**: stdio 逐行 JSON
- **用途**: 编辑器/IDE 集成
- **实现**: `core/src/bin/weft-rpc.rs`

## callPlugin 路由决策树

`WeftCorePluginClient.call()` 的完整决策逻辑：

```
call(pluginName, action, data)
  │
  ├─ serviceWebhookUrls.has(pluginName)?
  │   YES → callViaServiceWebhook()  // 手动注册的直接 URL
  │   NO  ↓
  │
  ├─ isServicePlugin(pluginName)?
  │   │ (每 3 秒缓存，查 GET /api/services)
  │   YES → callViaServiceWebhook()  // core 代理到服务进程
  │   NO  ↓
  │
  └─ callViaWebSocket()  // WASM 插件
```

### 判断依据

`isServicePlugin()` 判定标准：**插件的 `plugin.toml` 中 `[package] runtime = "service"`**。

core 启动时：
1. 解析所有 `plugin.toml`
2. `runtime == "service"` → 调 `build_service_config()` → 提取 `health_url` → 注册到 `process_manager`
3. `GET /api/services` 返回所有已注册服务名
4. 前端缓存这个列表 3 秒

## Suite/Binding 系统

### 文件结构

```
<runtimeDir>/
├── suites/
│   └── weft.json          # Suite 定义 (产品 + bindings)
└── suite-state/
    └── weft.lock.json     # Suite 锁 (最终解析的 product_graph)
```

### 绑定解析

```ts
resolveBoundPlugin(slot: string): string | undefined {
    // 1. 读 suite 定义
    const suite = readManagedSuiteDefinition()  // suites/weft.json
    // 2. 读 suite 锁
    const lock = readManagedSuiteLock()         // suite-state/weft.lock.json
    // 3. 合并 product_graph
    const merged = mergeManagedProductGraphs(suite?.product_graph, lock?.product_graph)
    // 4. 查 slot_bindings
    return deriveBindingsFromProductGraph(merged)[slot]
    //     e.g. slot="skills" → "skills-runtime"
}
```

### 合并优先级

1. lock 的 `slot_bindings` (最高优先级)
2. suite 的 `slot_bindings`
3. suite 的 `extension_plugins`
4. suite 的 `default_plugins`
5. lock 的 `plugin_nodes` + `edges`

## 用户输入 → 工具调用 完整流程

### 流程 A: Chat Completion (直接 LLM 调用)

```
用户输入
  ↓
chatCompletion(request)
  ↓
POST /v1/chat/completions
  ↓ core pipeline (Router → KeySelector → Transform → ErrorHandler)
  ↓
LLM Provider (DeepSeek/OpenAI/Anthropic)
  ↓
响应 → 直接返回给前端
```

### 流程 B: Companion Turn (伴侣规划 + 代理执行)

```
用户输入 "帮我联网查 React 最新文档"
  ↓
companionHandleUserTurn(input)
  ↓ resolveBoundPlugin("companion_runtime") → "companion-core"
  ↓ isServicePlugin → true
  ↓ callViaServiceWebhook()
  ↓
POST /api/services/companion-core/webhook
  body: {action: "handle_user_turn", data: {content, history, ...}}
  ↓
companion-core/server.py::handle_user_turn()
  ├─ 构建 prompt
  ├─ request_completion() → LLM 判断 direct/delegate
  ├─ LLM 返回: {kind: "delegate", reason: "联网搜索 React 最新文档"}
  ├─ parse_turn_decision()
  └─ execute_delegate_turn()
       ├─ plugin_call("agent_runtime", "create_session", ...)
       │    POST /api/plugins/agent_runtime/call → WASM agent-core
       ├─ generate_agent_completion_structured()
       │    LLM 决定: call web_search("React latest docs")
       ├─ agent-core → skills.execute_tool("web_search", args)
       │    → WASM skills-runtime → do_web_search()
       ├─ 最多 3 轮工具调用
       └─ 返回最终结果
  ↓
响应 → companion → Electron UI
```

### 流程 C: Tool Execution (直接工具执行)

```
executeTool("web_search", {query: "React docs"})
  ↓ shouldExecuteToolWithJsFirst("web_search") → true
  ↓ executeJsExtensionRuntime({tool: "web_search", args})
  │   → POST /api/js-extension-runtime/execute
  │   → js-extension-runtime/server.js
  │   → web_search with 9-provider fallback chain
  ↓ 成功 → 返回结果
  ↓ 失败 → fallback:
  ↓ callPlugin("skills", "execute_tool", {tool: "web_search", args})
      → WS → WASM skills-runtime → do_web_search()
```

## 插件别名系统

core 维护 `plugin_aliases` 映射，将抽象槽位名映射到实际插件名：

```toml
[plugin_aliases]
skills = "skills-runtime"
mcp = "mcp-client"
memory = "memory-store"
agent_runtime = "agent-runtime"
scheduler = "cron"
channels = "channel-core"
companion_runtime = "companion-core"
```

这些别名由 `buildDefaultPluginAliasMap()` 从 `product_graph.slot_bindings` 生成。
