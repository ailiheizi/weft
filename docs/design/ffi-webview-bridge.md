# FFI WebView Bridge — 设计文档

> **Status**: Draft  
> **Author**: (auto-generated)  
> **Date**: 2026-07-02  
> **Scope**: Package WebView UI → Core API 的纯 FFI 调用通道

---

## 1. 动机

Package UI 页面运行在 `webview_windows`（WebView2）中。当前调用 Core API 需通过 HTTP loopback (`127.0.0.1:PORT/api/*`)。这带来：

- 额外端口占用与防火墙风险
- 冷启动时端口竞争
- 不必要的序列化/TCP 开销

目标：让 WebView 中的 JS 代码**通过 Dart FFI 桥直接调用 Core**，消除 HTTP 中间层，同时保留 HTTP 作为 fallback。

---

## 2. 架构总览

```
┌───────────────────────── WebView2 (JS) ─────────────────────────┐
│                                                                   │
│  fetch("/api/v1/chat/send", {method:"POST", body:...})           │
│       │                                                           │
│       ▼                                                           │
│  ┌─────────────────────────────────────────┐                     │
│  │  webview-bridge-shim.js                  │                     │
│  │  • 拦截 CORE_BASE/api/* 的 fetch        │                     │
│  │  • 生成 request id                       │                     │
│  │  • postMessage({id, method, url, body})  │                     │
│  │  • 等待 id 对应的 response Promise       │                     │
│  └───────────────────┬─────────────────────┘                     │
│                      │ chrome.webview.postMessage                  │
└──────────────────────┼────────────────────────────────────────────┘
                       │
          ─────────────┼─────────── WebView2 IPC boundary ─────────
                       │
┌──────────────────────▼───────────────────────────────────────────┐
│              Dart (WebViewBridge)                                  │
│                                                                   │
│  controller.webMessage.listen((msg) {                             │
│    final req = decode(msg);                                       │
│    final resp = await ffiAdapter.send(req → RequestEnvelope);     │
│    controller.postWebMessage(encode({id, status, body}));         │
│  })                                                               │
│                                                                   │
│  ┌────────────────┐       ┌──────────────────┐                   │
│  │ FfiAdapter     │       │ HttpAdapter      │                   │
│  │ (weft_call)    │       │ (fallback)       │                   │
│  └───────┬────────┘       └───────┬──────────┘                   │
│          │                        │                               │
└──────────┼────────────────────────┼───────────────────────────────┘
           │                        │
           ▼                        ▼
    ┌─────────────┐          ┌─────────────┐
    │ weft_core   │          │ HTTP server │
    │ (Rust DLL)  │          │ (loopback)  │
    └─────────────┘          └─────────────┘
```

---

## 3. 消息协议

### 3.1 JS → Dart (Request)

```jsonc
{
  "id": "uuid-v4",          // 用于匹配响应
  "type": "request",        // 消息类型（预留 "cancel" 等扩展）
  "method": "POST",         // HTTP method → RpcMethod 映射
  "url": "/api/v1/chat/send",
  "headers": { "Content-Type": "application/json" },
  "body": { /* request payload */ }
}
```

### 3.2 Dart → JS (Response)

```jsonc
{
  "id": "uuid-v4",          // 与 request.id 对应
  "type": "response",
  "status": 200,
  "headers": { "Content-Type": "application/json" },
  "body": { /* response payload */ }
}
```

### 3.3 Method 映射

| HTTP Method | RpcMethod |
|---|---|
| GET | query |
| POST | call |
| PUT | call |
| DELETE | call |
| SUBSCRIBE (custom header) | subscribe |

---

## 4. 详细流程

### Step 1: 注入 JS Shim

Dart 在 WebView 创建后、页面加载前注入 shim：

```dart
await controller.addScriptToExecuteOnDocumentCreated(shimSource);
```

Shim 来源：打包到 assets 的 `webview-bridge-shim.js`。

### Step 2: Fetch 拦截

Shim monkey-patch `window.fetch`，匹配 `CORE_BASE/api/*` 的请求走 postMessage 通道，其余请求走原生 fetch。

### Step 3: Dart 监听

`WebViewBridge` 订阅 `controller.webMessage` stream，解码 JSON 消息。

### Step 4: FFI 调度

将 JS request 转为 `RequestEnvelope`，调用 `FfiTransportAdapter.send()`。

### Step 5: 回传结果

将 `ResponseEnvelope` 序列化为 JSON，通过 `controller.postWebMessage()` 发回 WebView。

### Step 6: Promise 解析

Shim 中对应 id 的 pending Promise 被 resolve，JS 层拿到标准 `Response` 对象。

---

## 5. 错误处理

| 场景 | 处理方式 |
|---|---|
| FFI adapter 不可用 | Dart 层降级为 HTTP adapter 调用 |
| weft_call 返回错误 | 包装为 status 5xx 回传 JS |
| postMessage 超时 | JS shim 内置 30s timeout，reject Promise |
| 消息格式错误 | Dart 回传 `{id, status:400, body:{error:"..."}}` |
| WebView 已销毁 | Dart 侧 try-catch，静默丢弃 |

---

## 6. 安全考量

1. **Origin 限制**：Shim 只拦截指向 `CORE_BASE` 的请求，不影响外部 URL fetch。
2. **无跨域泄露**：postMessage 仅在 app 进程内流转，不经过网络。
3. **输入校验**：Dart 层对 `url` 做白名单前缀匹配 (`/api/`)，拒绝非法路径。
4. **Body 大小限制**：单条消息上限 16MB（WebView2 postMessage 限制）。

---

## 7. 性能预期

| 指标 | HTTP loopback | FFI bridge |
|---|---|---|
| 单次 RPC 延迟 | ~2-5ms | ~0.1-0.5ms |
| 序列化开销 | JSON×2 + TCP | JSON×1 + postMessage |
| 连接建立 | TCP handshake | 无 |
| 端口占用 | 1 port | 0 |

---

## 8. 实现步骤

### Phase 1: 基础通道（本次）

- [x] 设计文档
- [ ] JS shim 实现 fetch 拦截 + postMessage
- [ ] Dart `WebViewBridge` 类：监听 → 调度 → 回传
- [ ] 在 WebView 页面加载流程中注入 shim
- [ ] 验证：package UI 页面能通过 FFI 完成 API 调用

### Phase 2: 流式支持

- [ ] 扩展协议支持 SSE / streaming response
- [ ] Dart 侧用 `subscribe()` 逐条推送
- [ ] JS shim 支持 ReadableStream 模拟

### Phase 3: 错误恢复 & 监控

- [ ] 超时重试机制
- [ ] Bridge 健康检查 heartbeat
- [ ] 性能指标上报

---

## 9. 文件清单

| 文件 | 说明 |
|---|---|
| `docs/design/ffi-webview-bridge.md` | 本设计文档 |
| `clients/weft_client/assets/webview-bridge-shim.js` | JS 拦截 shim |
| `clients/weft_client/lib/core/transport/webview_bridge.dart` | Dart 桥接处理器 |

---

## 10. 与现有架构的关系

- **TransportAdapter**：`WebViewBridge` 不是新的 adapter，而是 WebView IPC 层到已有 `FfiTransportAdapter` 的**桥接器**。
- **FfiTransportAdapter**：已实现 `send()` 调用 `weft_call`，直接复用。
- **HttpAdapter**：作为 fallback，当 FFI 不可用时自动降级。
- **TransportProvider**：提供当前可用的 adapter 实例，`WebViewBridge` 从中获取。

---

## 11. 配置

```toml
# config.toml 或运行时环境
[webview.bridge]
enabled = true           # 是否启用 FFI bridge（false = 强制 HTTP）
timeout_ms = 30000       # JS 侧超时
core_base = ""           # 留空 = 自动检测；填写则为拦截前缀
```

Package UI 页面无需感知底层传输——它们正常使用 `fetch("/api/...")` 即可，shim 透明拦截。
