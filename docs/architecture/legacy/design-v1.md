# WEFT-plug 架构设计文档

## 一、项目定位

WEFT-plug 是一个轻量化的 AI 服务编排核心，用 Rust 编写。

核心职责：
- 管理多供应商、多 API key、多模型的配置与路由
- 管理外部服务进程的生命周期（weft-claw、airi 等）
- 为 UI 前端提供统一的 HTTP/WebSocket API
- 通过 Trait 接口实现可扩展，未来支持 WASM 插件热加载

设计原则：
- 核心可选 — UI 可以绕过核心直连执行端
- Trait 优先 — 所有可变行为用 Trait 定义，内置默认实现
- 配置驱动 — 普通用户只需编辑 config.toml
- 渐进式插件化 — 第一阶段 Trait + 配置，第二阶段 WASM 动态加载

## 二、整体架构

```
┌─────────────────────────────────────────────────┐
│                   UI 层（独立进程）                │
│  Electron 聊天窗口 / Live2D 桌宠 / Web / 视频工作台  │
└──────────────────┬──────────────────────────────┘
                   │ HTTP / WebSocket
┌──────────────────▼──────────────────────────────┐
│              weft-core（本服务）                  │
│                                                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────────────┐ │
│  │ API 路由  │ │ Key 管理  │ │  进程管理器       │ │
│  │ (Router) │ │(KeyStore)│ │(ProcessManager)  │ │
│  └────┬─────┘ └────┬─────┘ └───────┬──────────┘ │
│       │            │               │             │
│  ┌────▼────────────▼───────────────▼──────────┐  │
│  │           请求管道 (Pipeline)                │  │
│  │  route → select_key → build → send → parse │  │
│  │    ↑         ↑          ↑             ↑     │  │
│  │  Trait     Trait      Trait         Trait    │  │
│  └────────────────────────────────────────────┘  │
└──────────────────┬──────────────────────────────┘
                   │ HTTP / WebSocket
┌──────────────────▼──────────────────────────────┐
│              执行端（独立进程）                     │
│  weft-claw / airi / 其他 AI 服务                   │
└─────────────────────────────────────────────────┘
```

## 三、核心 Trait 定义

### 3.1 请求路由 — Router

决定请求发给哪个 provider。

```rust
pub trait Router: Send + Sync {
    /// 根据请求内容选择 provider
    fn route(&self, request: &Request, providers: &[ProviderConfig]) -> Result<String>;
}
```

默认实现：`DefaultRouter` — 按 config.toml 中的 default_provider 字段路由。

### 3.2 Key 选择 — KeySelector

从多个 key 中选一个。

```rust
pub trait KeySelector: Send + Sync {
    /// 从候选 key 列表中选择一个
    fn select(&self, provider: &str, keys: &[ApiKey], context: &RequestContext) -> Result<ApiKey>;

    /// 标记某个 key 失败（用于 failover）
    fn mark_failed(&self, key: &ApiKey);

    /// 标记某个 key 成功（用于统计）
    fn mark_success(&self, key: &ApiKey);
}
```

默认实现：
- `RoundRobinSelector` — 轮询
- `RandomSelector` — 随机
- `FailoverSelector` — 失败自动切换下一个
- `LeastUsedSelector` — 最少使用优先

### 3.3 请求构建 — RequestBuilder

把统一格式转成 provider 的私有格式。

```rust
pub trait RequestBuilder: Send + Sync {
    /// 构建 HTTP 请求
    fn build(&self, request: &Request, key: &ApiKey, provider: &ProviderConfig) -> Result<HttpRequest>;

    /// 构建 WebSocket 消息
    fn build_ws(&self, message: &WsMessage, key: &ApiKey, provider: &ProviderConfig) -> Result<Vec<u8>>;
}
```

默认实现：
- `OpenAICompatBuilder` — OpenAI 兼容格式（覆盖 OpenRouter、大部分国产 API）
- `AnthropicBuilder` — Anthropic 原生格式
- `Weft ClawBuilder` — Weft Claw WebSocket 协议

### 3.4 响应解析 — ResponseParser

把 provider 的响应转成统一格式。

```rust
pub trait ResponseParser: Send + Sync {
    /// 解析 HTTP 响应
    fn parse(&self, response: &HttpResponse, provider: &ProviderConfig) -> Result<Response>;

    /// 解析流式 chunk
    fn parse_chunk(&self, chunk: &[u8], provider: &ProviderConfig) -> Result<StreamChunk>;
}
```

默认实现：与 RequestBuilder 对应。

### 3.5 错误处理 — ErrorHandler

请求失败时的策略。

```rust
pub enum ErrorAction {
    Retry,                          // 用同一个 key 重试
    SwitchKey,                      // 换 key 重试
    SwitchProvider(String),         // 换 provider 重试
    Fail(String),                   // 直接失败，返回错误信息
}

pub trait ErrorHandler: Send + Sync {
    fn handle(&self, error: &RequestError, context: &RequestContext) -> ErrorAction;
}
```

默认实现：`DefaultErrorHandler` — 429 换 key，5xx 重试，其他直接失败。

### 3.6 进程插件 — ServicePlugin

描述一个可被核心管理的外部服务。

```rust
pub trait ServicePlugin: Send + Sync {
    fn name(&self) -> &str;
    fn start(&mut self, config: &ServiceConfig) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn restart(&mut self, config: &ServiceConfig) -> Result<()>;
    fn health_check(&self) -> Result<HealthStatus>;
    fn status(&self) -> ServiceStatus;
}
```

默认实现：`ProcessService` — 通过 `Command` 启动子进程，定期 HTTP 健康检查。

## 四、配置文件设计

```toml
# config.toml

[core]
host = "127.0.0.1"
port = 3000
log_level = "info"

# ── 供应商配置 ──

[[providers]]
name = "openrouter"
base_url = "https://openrouter.ai/api/v1"
format = "openai"  # openai / anthropic / custom
keys = ["sk-or-xxx", "sk-or-yyy"]
models = ["anthropic/claude-sonnet-4", "openai/gpt-4o"]

[[providers]]
name = "anthropic"
base_url = "https://api.anthropic.com/v1"
format = "anthropic"
keys = ["sk-ant-xxx"]
models = ["claude-sonnet-4"]

[[providers]]
name = "openai"
base_url = "https://api.openai.com/v1"
format = "openai"
keys = ["sk-xxx"]
models = ["gpt-4o", "gpt-4o-mini"]

# ── 路由与策略 ──

[routing]
default_provider = "openrouter"
default_model = "anthropic/claude-sonnet-4"

[key_strategy]
mode = "round_robin"  # round_robin / random / least_used / failover

[fallback]
retry_count = 2
switch_key = true
switch_provider = true
priority = ["openrouter", "anthropic", "openai"]

# ── 服务进程管理 ──

[[services]]
name = "weft-claw"
command = "./weft-claw"
args = ["--port", "42617", "--config", "./weft-claw-config/config.toml"]
workdir = "./services/weft-claw"
health_url = "http://127.0.0.1:42617/health"
health_interval = 10  # 秒
auto_start = true
restart_on_crash = true

[services.env]
WEFT_CLAW_API_KEY = "{key:openrouter}"  # 引用 providers 中的 key

[[services]]
name = "airi"
command = "pnpm"
args = ["start", "--port", "5174"]
workdir = "./services/airi"
health_url = "http://127.0.0.1:5174"
auto_start = true
restart_on_crash = true

# ── 未来：WASM 插件（第二阶段）──

# [[plugins]]
# name = "video-gen"
# wasm = "./plugins/video-gen.wasm"
# permissions = ["network"]
```

## 五、HTTP/WebSocket API

### 5.1 核心管理

```
GET  /api/health                    # 核心健康状态
GET  /api/config                    # 获取当前配置（key 脱敏）
PUT  /api/config                    # 更新配置
```

### 5.2 供应商与 Key 管理

```
GET    /api/providers               # 列出所有供应商
POST   /api/providers               # 添加供应商
PUT    /api/providers/:name         # 更新供应商
DELETE /api/providers/:name         # 删除供应商

GET    /api/providers/:name/keys    # 列出某供应商的 key（脱敏）
POST   /api/providers/:name/keys    # 添加 key
DELETE /api/providers/:name/keys/:index  # 删除 key

GET    /api/providers/:name/models  # 列出可用模型
```

### 5.3 服务进程管理

```
GET    /api/services                # 所有服务状态
POST   /api/services/:name/start   # 启动
POST   /api/services/:name/stop    # 停止
POST   /api/services/:name/restart # 重启
GET    /api/services/:name/health  # 健康检查
GET    /api/services/:name/logs    # 最近日志
```

### 5.4 请求代理（核心功能）

```
# HTTP 代理 — 统一格式，核心自动路由到对应 provider
POST   /api/chat/completions       # 聊天补全（OpenAI 兼容格式）
POST   /api/proxy/:service/*path   # 透传到指定服务

# WebSocket 代理
WS     /ws/chat                    # 聊天 WebSocket（核心自动路由）
WS     /ws/proxy/:service          # 透传到指定服务的 WebSocket
```

### 5.5 统计

```
GET    /api/stats/usage             # key 使用统计
GET    /api/stats/costs             # 费用统计（如果 provider 支持）
```

## 六、请求管道流程

```
UI 发送请求
    │
    ▼
[Router.route()]          ← 选择 provider
    │
    ▼
[KeySelector.select()]    ← 选择 API key
    │
    ▼
[RequestBuilder.build()]  ← 构建 provider 格式的请求
    │
    ▼
[HTTP/WS 发送]            ← 核心执行网络请求
    │
    ├── 成功 → [ResponseParser.parse()] → 返回统一格式
    │
    └── 失败 → [ErrorHandler.handle()]
                  │
                  ├── Retry → 回到 [HTTP/WS 发送]
                  ├── SwitchKey → 回到 [KeySelector.select()]
                  ├── SwitchProvider → 回到 [Router.route()]
                  └── Fail → 返回错误
```

## 七、项目结构

```
weft-core/
├── Cargo.toml
├── config.toml                  # 默认配置
├── src/
│   ├── main.rs                  # 入口
│   ├── lib.rs                   # 库导出
│   │
│   ├── config/
│   │   ├── mod.rs               # 配置结构体
│   │   └── store.rs             # TOML 读写
│   │
│   ├── traits/
│   │   ├── mod.rs               # 所有 trait 导出
│   │   ├── router.rs            # Router trait
│   │   ├── key_selector.rs      # KeySelector trait
│   │   ├── request_builder.rs   # RequestBuilder trait
│   │   ├── response_parser.rs   # ResponseParser trait
│   │   ├── error_handler.rs     # ErrorHandler trait
│   │   └── service_plugin.rs    # ServicePlugin trait
│   │
│   ├── defaults/
│   │   ├── mod.rs               # 所有默认实现导出
│   │   ├── router.rs            # DefaultRouter
│   │   ├── key_selectors.rs     # RoundRobin/Random/Failover/LeastUsed
│   │   ├── builders/
│   │   │   ├── mod.rs
│   │   │   ├── openai.rs        # OpenAI 兼容格式
│   │   │   ├── anthropic.rs     # Anthropic 格式
│   │   │   └── weft-claw.rs      # Weft Claw WebSocket 协议
│   │   ├── parsers/
│   │   │   ├── mod.rs
│   │   │   ├── openai.rs
│   │   │   ├── anthropic.rs
│   │   │   └── weft-claw.rs
│   │   ├── error_handler.rs     # DefaultErrorHandler
│   │   └── process_service.rs   # ProcessService（子进程管理）
│   │
│   ├── pipeline/
│   │   ├── mod.rs               # Pipeline 组装与执行
│   │   └── context.rs           # RequestContext
│   │
│   ├── api/
│   │   ├── mod.rs               # axum 路由注册
│   │   ├── providers.rs         # 供应商 CRUD
│   │   ├── services.rs          # 服务管理
│   │   ├── proxy.rs             # 请求代理
│   │   ├── stats.rs             # 统计
│   │   └── ws.rs                # WebSocket 处理
│   │
│   └── process/
│       ├── mod.rs               # ProcessManager
│       └── health.rs            # 健康检查循环
│
├── plugins/                     # 未来 WASM 插件目录
│   └── README.md
│
└── services/                    # 服务二进制/配置目录
    ├── weft-claw/
    └── airi/
```

## 八、核心数据结构

```rust
// ── 请求/响应统一格式 ──

pub struct Request {
    pub id: String,
    pub action: String,           // "chat", "completion", "image", "video" 等
    pub model: Option<String>,    // 指定模型，不指定则用默认
    pub provider: Option<String>, // 指定 provider，不指定则走路由
    pub messages: Vec<Message>,
    pub params: HashMap<String, Value>,  // temperature, max_tokens 等
    pub stream: bool,
}

pub struct Message {
    pub role: String,             // user / assistant / system / tool
    pub content: String,
    pub metadata: Option<HashMap<String, Value>>,
}

pub struct Response {
    pub id: String,
    pub model: String,
    pub provider: String,
    pub content: String,
    pub usage: Option<Usage>,
    pub metadata: HashMap<String, Value>,
}

pub struct StreamChunk {
    pub delta: String,
    pub done: bool,
    pub metadata: Option<HashMap<String, Value>>,
}

// ── 配置结构 ──

pub struct ProviderConfig {
    pub name: String,
    pub base_url: String,
    pub format: String,           // "openai" / "anthropic" / "custom"
    pub keys: Vec<ApiKey>,
    pub models: Vec<String>,
}

pub struct ApiKey {
    pub value: String,
    pub label: Option<String>,    // 用户自定义标签
    pub usage_count: u64,
    pub last_used: Option<u64>,
    pub failed: bool,
}

pub struct ServiceConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub workdir: Option<String>,
    pub env: HashMap<String, String>,
    pub health_url: Option<String>,
    pub health_interval: u64,
    pub auto_start: bool,
    pub restart_on_crash: bool,
}
```

## 九、阶段规划

### 第一阶段：核心可用（当前）

- [x] 设计文档
- [ ] Trait 定义 + 默认实现
- [ ] 配置文件读写
- [ ] 请求管道（Pipeline）
- [ ] HTTP API（axum）
- [ ] WebSocket 代理
- [ ] 进程管理器
- [ ] 多 key 轮换 + failover

### 第二阶段：WASM 插件

- [ ] wasmtime 集成
- [ ] WASM host 函数（http_request, get_api_key 等）
- [ ] 插件加载/卸载
- [ ] 用户可通过 WASM 覆盖任意 Trait

### 第三阶段：生态扩展

- [ ] 视频生成插件
- [ ] 图片生成插件
- [ ] TTS/STT 插件
- [ ] 插件市场 / 分发机制

## 十、"核心可选"机制

UI 插件的配置中可以选择是否经过核心：

```toml
# UI 插件配置示例

# 走核心（推荐）
[connection]
mode = "core"
core_url = "http://127.0.0.1:3000"

# 直连执行端（不需要核心）
[connection]
mode = "direct"
weft-claw_url = "ws://127.0.0.1:42617/ws/chat"
api_key = "sk-xxx"
```

当 mode = "direct" 时，UI 自己持有 API key 并直接与执行端通信，核心进程不需要运行。
