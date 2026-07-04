# WEFT-plug 架构设计文档 v2

> 基于 v1 + 开源 AI Gateway 生态调研 + Rust 插件系统调研后的改进版

## 一、项目定位

WEFT-plug 是一个轻量化的 AI 服务编排核心，用 Rust 编写。

核心职责：
- 管理多供应商、多 API key、多模型的配置与路由
- 管理外部服务进程的生命周期（weft-claw、airi 等）
- 为 UI 前端提供统一的 HTTP/WebSocket API（OpenAI 兼容格式）
- 通过 Trait + Extism WASM 实现可扩展的插件系统

设计原则：
- 核心可选 -- UI 可以绕过核心直连执行端
- Trait 优先 -- 所有可变行为用 Trait 定义，内置默认实现
- 配置驱动 -- 普通用户只需编辑 config.toml
- Extism WASM -- 第一阶段就集成，用户可用任意语言写插件覆盖 Trait

## 二、技术选型（调研结论）

| 组件 | 选择 | 理由 |
|------|------|------|
| HTTP 框架 | axum | Tower middleware 生态，内置 WS/SSE，社区最活跃 |
| WASM 运行时 | Extism (基于 wasmtime) | 专为插件系统设计，host_fn! 宏简化 host 函数，沙箱隔离 |
| 配置格式 | TOML | Rust 生态标准，serde 支持好 |
| 异步运行时 | tokio | axum 依赖，进程管理需要 |
| HTTP 客户端 | reqwest | 流式响应支持好，tokio 兼容 |
| 序列化 | serde + serde_json | 标准选择 |

### 关键调研发现

1. Extism vs 裸 wasmtime：Extism 封装了内存管理、数据传递、host 函数注册，代码量减少 70%+
2. OpenAI 兼容格式是事实标准：LiteLLM、LLM Gateway、OpenRouter 都用它，我们的核心 API 也采用
3. Tower middleware 模式：axum 的 middleware 链天然适合我们的请求管道
4. Virtual Key 机制：参考 LiteLLM，核心发虚拟 key 给 UI，内部映射到真实 key，安全且灵活

## 三、整体架构

```
┌─────────────────────────────────────────────────┐
│                   UI 层（独立进程）                │
│  Electron 聊天窗口 / Live2D 桌宠 / Web / 视频工作台  │
└──────────────────┬──────────────────────────────┘
                   │ HTTP / WebSocket (OpenAI 兼容格式)
┌──────────────────▼──────────────────────────────┐
│              weft-core（本服务）                  │
│                                                  │
│  ┌──────────────────────────────────────────┐   │
│  │         Axum HTTP Server                 │   │
│  │  /v1/chat/completions (OpenAI 兼容)      │   │
│  │  /ws/chat (WebSocket)                    │   │
│  │  /api/providers, /api/services, ...     │   │
│  └────────────────┬─────────────────────────┘   │
│                   │                              │
│  ┌────────────────▼─────────────────────────┐   │
│  │      Tower Middleware Pipeline           │   │
│  │  Auth → Route → SelectKey → Transform    │   │
│  │    ↑      ↑         ↑           ↑        │   │
│  │  Trait  Trait     Trait       Trait      │   │
│  │    ↓      ↓         ↓           ↓        │   │
│  │  可被 Extism WASM 插件覆盖              │   │
│  └────────────────┬─────────────────────────┘   │
│                   │                              │
│  ┌────────────────▼─────────────────────────┐   │
│  │  Provider Clients (reqwest)              │   │
│  │  OpenAI / Anthropic / OpenRouter / ...   │   │
│  └──────────────────────────────────────────┘   │
│                                                  │
│  ┌──────────────────────────────────────────┐   │
│  │  Process Manager (tokio::process)        │   │
│  │  weft-claw / airi / 其他服务               │   │
│  └──────────────────────────────────────────┘   │
└──────────────────┬──────────────────────────────┘
                   │ HTTP / WebSocket
┌──────────────────▼──────────────────────────────┐
│              执行端（独立进程）                     │
│  weft-claw / airi / 其他 AI 服务                   │
└─────────────────────────────────────────────────┘
```

## 四、核心 Trait 定义（简化版）

基于 Tower middleware 模式，我们把原来的 6 个 Trait 简化为 4 个核心 Layer：

### 4.1 RouterLayer

决定请求发给哪个 provider。

```rust
#[async_trait]
pub trait RouterLayer: Send + Sync {
    async fn route(
        &self,
        request: &mut ChatRequest,
        providers: &[ProviderConfig],
    ) -> Result<String>;
}
```

默认实现：
- `DefaultRouter` -- 按 config.toml 的 default_provider
- `ModelBasedRouter` -- 根据 model 字段自动选择 provider
- `LoadBalancingRouter` -- 负载均衡

### 4.2 KeySelectorLayer

从多个 key 中选一个。

```rust
#[async_trait]
pub trait KeySelectorLayer: Send + Sync {
    async fn select(
        &self,
        provider: &str,
        keys: &[ApiKey],
    ) -> Result<ApiKey>;

    fn mark_failed(&self, key_id: &str);
    fn mark_success(&self, key_id: &str);
}
```

默认实现：
- `RoundRobinSelector`
- `RandomSelector`
- `FailoverSelector`
- `LeastUsedSelector`

### 4.3 TransformLayer

请求/响应格式转换。

```rust
#[async_trait]
pub trait TransformLayer: Send + Sync {
    async fn transform_request(
        &self,
        request: &ChatRequest,
        provider: &ProviderConfig,
    ) -> Result<HttpRequest>;

    async fn transform_response(
        &self,
        response: HttpResponse,
        provider: &ProviderConfig,
    ) -> Result<ChatResponse>;

    async fn transform_stream_chunk(
        &self,
        chunk: Bytes,
        provider: &ProviderConfig,
    ) -> Result<Option<StreamChunk>>;
}
```

默认实现：
- `OpenAITransform` -- OpenAI 兼容格式（覆盖 OpenRouter、大部分国产 API）
- `AnthropicTransform` -- Anthropic 原生格式
- `Weft ClawTransform` -- Weft Claw WebSocket 协议

### 4.4 ErrorHandlerLayer

请求失败时的策略。

```rust
#[async_trait]
pub trait ErrorHandlerLayer: Send + Sync {
    async fn handle(
        &self,
        error: &RequestError,
        context: &RequestContext,
    ) -> ErrorAction;
}

pub enum ErrorAction {
    Retry { delay_ms: u64 },
    SwitchKey,
    SwitchProvider { to: String },
    Fail { message: String },
}
```

默认实现：
- `DefaultErrorHandler` -- 429 换 key，5xx 重试，其他直接失败

## 五、Extism WASM 插件系统

### 5.1 为什么选 Extism

- 专为"让应用可扩展"设计，不是通用 WASM 运行时
- `host_fn!` 宏一行代码注册 host 函数
- `plugin.call::<&str, &str>("fn_name", input)` 调用插件
- 内置内存管理，不需要手动操作指针
- 沙箱隔离，插件崩溃不影响核心
- 支持 Rust/Go/JS/Python/C 编写插件

### 5.2 核心暴露给 WASM 的 Host 函数

```rust
// 网络请求 -- 插件通过核心发 HTTP 请求
host_fn!(http_request(user_data: CoreContext;
    method: String, url: String, headers: String, body: String
) -> String {
    // 核心代为执行 HTTP 请求，返回 JSON 响应
});

// 获取 API key -- 插件不直接持有 key
host_fn!(get_api_key(user_data: CoreContext; provider: String) -> String {
    // 从 KeyStore 中选一个 key 返回
});

// 获取配置
host_fn!(get_config(user_data: CoreContext; key: String) -> String {
    // 读取 config.toml 中的值
});

// 日志
host_fn!(log_message(user_data: CoreContext; level: String, msg: String) {
    // 写入核心日志系统
});

// 存储（简单 KV）
host_fn!(kv_get(user_data: CoreContext; key: String) -> String { ... });
host_fn!(kv_set(user_data: CoreContext; key: String, value: String) { ... });
```

### 5.3 WASM 插件导出的函数

```rust
// 插件信息
#[plugin_fn]
pub fn plugin_info() -> String {
    json!({
        "name": "my-router",
        "version": "0.1.0",
        "overrides": ["router"]  // 声明覆盖哪个 Layer
    }).to_string()
}

// 覆盖路由逻辑
#[plugin_fn]
pub fn route(input: String) -> String {
    let req: RouteInput = serde_json::from_str(&input).unwrap();
    // 自定义路由逻辑
    json!({ "provider": "anthropic" }).to_string()
}

// 覆盖 key 选择逻辑
#[plugin_fn]
pub fn select_key(input: String) -> String {
    let ctx: KeySelectInput = serde_json::from_str(&input).unwrap();
    // 自定义选 key 逻辑
    json!({ "key_index": 0 }).to_string()
}

// 覆盖请求转换
#[plugin_fn]
pub fn transform_request(input: String) -> String { ... }

// 覆盖响应转换
#[plugin_fn]
pub fn transform_response(input: String) -> String { ... }

// 覆盖错误处理
#[plugin_fn]
pub fn handle_error(input: String) -> String { ... }
```

### 5.4 插件加载流程

```
核心启动
  │
  ├── 扫描 plugins/ 目录下的 .wasm 文件
  │
  ├── 对每个 .wasm：
  │     1. Extism::Plugin::new(manifest) 加载
  │     2. 调用 plugin_info() 获取元信息
  │     3. 根据 "overrides" 字段注册到对应 Layer
  │
  └── Pipeline 组装时：
        如果某 Layer 有 WASM 覆盖 → 用 WASM 插件
        否则 → 用默认 Trait 实现
```

## 六、Virtual Key 机制

参考 LiteLLM 的设计，核心为每个 UI 客户端发放虚拟 key：

```
UI 持有: vk-weft-xxxx (虚拟 key)
    │
    ▼ Authorization: Bearer vk-weft-xxxx
核心收到请求
    │
    ▼ 查 virtual_keys 表
映射到: provider=openrouter, real_key=sk-or-yyyy
    │
    ▼ 用真实 key 发请求
```

好处：
- UI 不接触真实 API key
- 可以给不同 UI 设置不同权限/额度
- 可以随时吊销虚拟 key 而不影响真实 key
- 核心可选时，UI 也可以直接用真实 key

配置：

```toml
[[virtual_keys]]
key = "vk-weft-chat-001"
label = "聊天窗口"
provider = "openrouter"
model = "anthropic/claude-sonnet-4"
max_rpm = 60          # 每分钟最大请求数
max_budget = 10.0     # 美元预算上限

[[virtual_keys]]
key = "vk-weft-pet-001"
label = "桌宠"
provider = "openrouter"
model = "openai/gpt-4o-mini"
max_rpm = 30
```

## 七、配置文件设计（v2）

```toml
# config.toml

[core]
host = "127.0.0.1"
port = 3000
log_level = "info"
data_dir = "./data"       # KV 存储、统计数据

# ── 供应商配置 ──

[[providers]]
name = "openrouter"
base_url = "https://openrouter.ai/api/v1"
format = "openai"
models = ["anthropic/claude-sonnet-4", "openai/gpt-4o"]

[[providers.keys]]
value = "sk-or-xxx"
label = "主 key"

[[providers.keys]]
value = "sk-or-yyy"
label = "备用 key"

[[providers]]
name = "anthropic"
base_url = "https://api.anthropic.com/v1"
format = "anthropic"
models = ["claude-sonnet-4"]

[[providers.keys]]
value = "sk-ant-xxx"

# ── 路由与策略 ──

[routing]
default_provider = "openrouter"
default_model = "anthropic/claude-sonnet-4"

[key_strategy]
mode = "failover"  # round_robin / random / least_used / failover

[fallback]
retry_count = 2
switch_key = true
switch_provider = true
priority = ["openrouter", "anthropic"]

# ── 虚拟 Key ──

[[virtual_keys]]
key = "vk-weft-001"
label = "默认"
max_rpm = 60

# ── 服务进程管理 ──

[[services]]
name = "weft-claw"
command = "./weft-claw"
args = ["--port", "42617"]
workdir = "./services/weft-claw"
health_url = "http://127.0.0.1:42617/health"
health_interval = 10
auto_start = true
restart_on_crash = true

[services.env]
WEFT_CLAW_API_KEY = "{key:openrouter}"

[[services]]
name = "airi"
command = "pnpm"
args = ["start", "--port", "5174"]
workdir = "./services/airi"
health_url = "http://127.0.0.1:5174"
auto_start = true

# ── WASM 插件 ──

[[plugins]]
name = "my-router"
wasm = "./plugins/my-router.wasm"
enabled = true

[[plugins]]
name = "video-gen"
wasm = "./plugins/video-gen.wasm"
enabled = false
```

## 八、HTTP/WebSocket API

### 8.1 OpenAI 兼容 API（核心功能）

```
POST /v1/chat/completions          # 聊天补全（流式/非流式）
POST /v1/completions               # 文本补全
POST /v1/embeddings                # 向量嵌入
GET  /v1/models                    # 列出所有可用模型
```

请求示例（和 OpenAI 完全一致）：
```json
{
  "model": "anthropic/claude-sonnet-4",
  "messages": [{"role": "user", "content": "hello"}],
  "stream": true
}
```

### 8.2 核心管理 API

```
GET  /api/health                    # 核心健康状态
GET  /api/config                    # 获取配置（key 脱敏）
PUT  /api/config                    # 热更新配置
```

### 8.3 供应商与 Key 管理

```
GET    /api/providers               # 列出所有供应商
POST   /api/providers               # 添加供应商
PUT    /api/providers/:name         # 更新供应商
DELETE /api/providers/:name         # 删除供应商
GET    /api/providers/:name/keys    # 列出 key（脱敏）
POST   /api/providers/:name/keys    # 添加 key
DELETE /api/providers/:name/keys/:id # 删除 key
```

### 8.4 Virtual Key 管理

```
GET    /api/virtual-keys            # 列出虚拟 key
POST   /api/virtual-keys            # 创建虚拟 key
DELETE /api/virtual-keys/:key       # 吊销虚拟 key
GET    /api/virtual-keys/:key/usage # 查看用量
```

### 8.5 服务进程管理

```
GET    /api/services                # 所有服务状态
POST   /api/services/:name/start
POST   /api/services/:name/stop
POST   /api/services/:name/restart
GET    /api/services/:name/health
GET    /api/services/:name/logs
```

### 8.6 WASM 插件管理

```
GET    /api/plugins                 # 列出已加载插件
POST   /api/plugins/load            # 加载新插件
POST   /api/plugins/:name/reload    # 热重载插件
DELETE /api/plugins/:name           # 卸载插件
```

### 8.7 WebSocket 代理

```
WS  /ws/chat                       # 聊天 WS（核心路由）
WS  /ws/proxy/:service             # 透传到指定服务
```

### 8.8 统计

```
GET  /api/stats/usage               # 按 key/provider/model 的用量
GET  /api/stats/costs               # 费用统计
GET  /api/stats/latency             # 延迟统计
```

## 九、请求管道流程（v2）

```
UI 发送请求 (OpenAI 格式)
    │
    ▼
[Auth Middleware]          ← 验证 Virtual Key / Bearer Token
    │
    ▼
[WASM: route()?]          ← 如果有 WASM 插件覆盖路由
[RouterLayer.route()]     ← 否则用默认 Trait 实现
    │                        选择 provider
    ▼
[WASM: select_key()?]     ← 如果有 WASM 插件覆盖选 key
[KeySelectorLayer.select()] ← 否则用默认实现
    │                        选择 API key
    ▼
[WASM: transform_request()?]
[TransformLayer.transform_request()]
    │                        转换为 provider 格式
    ▼
[reqwest 发送 HTTP 请求]   ← 核心执行
    │
    ├── 成功 ──────────────────────────────────┐
    │                                          ▼
    │                              [transform_response()]
    │                                   │
    │                                   ▼
    │                              返回 OpenAI 格式
    │
    ├── 流式成功 ──────────────────────────────┐
    │                                          ▼
    │                          [transform_stream_chunk()]
    │                              逐 chunk SSE 转发
    │
    └── 失败 → [ErrorHandlerLayer.handle()]
                  │
                  ├── Retry → 回到 [reqwest 发送]
                  ├── SwitchKey → 回到 [KeySelector]
                  ├── SwitchProvider → 回到 [Router]
                  └── Fail → 返回错误
```

## 十、项目结构（v2）

```
weft-core/
├── Cargo.toml
├── config.toml
├── src/
│   ├── main.rs                  # 入口：加载配置、启动 axum
│   ├── lib.rs                   # 库导出
│   │
│   ├── config/
│   │   ├── mod.rs               # AppConfig 结构体
│   │   └── store.rs             # TOML 读写 + 热更新
│   │
│   ├── layers/                  # Trait 定义（替代原 traits/）
│   │   ├── mod.rs
│   │   ├── router.rs            # RouterLayer trait
│   │   ├── key_selector.rs      # KeySelectorLayer trait
│   │   ├── transform.rs         # TransformLayer trait
│   │   └── error_handler.rs     # ErrorHandlerLayer trait
│   │
│   ├── defaults/                # 内置默认实现
│   │   ├── mod.rs
│   │   ├── router.rs            # Default/ModelBased/LoadBalancing
│   │   ├── key_selectors.rs     # RoundRobin/Random/Failover/LeastUsed
│   │   ├── transforms/
│   │   │   ├── mod.rs
│   │   │   ├── openai.rs        # OpenAI 兼容
│   │   │   ├── anthropic.rs     # Anthropic 原生
│   │   │   └── weft-claw.rs      # Weft Claw WS 协议
│   │   └── error_handler.rs
│   │
│   ├── pipeline/                # 请求管道
│   │   ├── mod.rs               # Pipeline 组装与执行
│   │   └── context.rs           # RequestContext（贯穿整个管道）
│   │
│   ├── plugin/                  # Extism WASM 插件系统
│   │   ├── mod.rs               # PluginManager
│   │   ├── host_fns.rs          # host_fn! 定义
│   │   └── bridge.rs            # WASM <-> Trait 桥接
│   │
│   ├── api/                     # Axum 路由
│   │   ├── mod.rs               # 路由注册
│   │   ├── openai_compat.rs     # /v1/chat/completions 等
│   │   ├── providers.rs         # 供应商 CRUD
│   │   ├── virtual_keys.rs      # 虚拟 Key 管理
│   │   ├── services.rs          # 服务管理
│   │   ├── plugins.rs           # 插件管理
│   │   ├── stats.rs             # 统计
│   │   └── ws.rs                # WebSocket 处理
│   │
│   ├── process/                 # 进程管理
│   │   ├── mod.rs               # ProcessManager
│   │   └── health.rs            # 健康检查
│   │
│   └── types.rs                 # 公共类型定义
│
├── plugins/                     # WASM 插件目录
│   └── README.md
│
└── services/                    # 服务目录
    ├── weft-claw/
    └── airi/
```

## 十一、核心数据结构（v2）

```rust
// ── OpenAI 兼容请求/响应 ──

#[derive(Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub max_tokens: Option<u64>,
    // 扩展字段：指定 provider（可选）
    #[serde(default)]
    pub x_provider: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,           // "chat.completion"
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Serialize, Deserialize)]
pub struct StreamChunk {
    pub id: String,
    pub object: String,           // "chat.completion.chunk"
    pub created: u64,
    pub model: String,
    pub choices: Vec<StreamChoice>,
}

// ── 配置结构 ──

#[derive(Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub base_url: String,
    pub format: String,
    pub keys: Vec<ApiKey>,
    pub models: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,               // 自动生成的唯一 ID
    pub value: String,
    pub label: Option<String>,
    #[serde(default)]
    pub usage_count: u64,
    #[serde(default)]
    pub last_used: Option<u64>,
    #[serde(default)]
    pub failed: bool,
}

#[derive(Serialize, Deserialize)]
pub struct VirtualKey {
    pub key: String,              // "vk-weft-xxx"
    pub label: Option<String>,
    pub provider: Option<String>, // 绑定到特定 provider
    pub model: Option<String>,    // 绑定到特定 model
    pub max_rpm: Option<u32>,
    pub max_budget: Option<f64>,
}

#[derive(Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub workdir: Option<String>,
    pub env: HashMap<String, String>,
    pub health_url: Option<String>,
    pub health_interval: Option<u64>,
    pub auto_start: bool,
    pub restart_on_crash: bool,
}

// ── 请求上下文（贯穿管道） ──

pub struct RequestContext {
    pub request_id: String,
    pub virtual_key: Option<VirtualKey>,
    pub selected_provider: Option<String>,
    pub selected_key: Option<ApiKey>,
    pub start_time: Instant,
    pub retry_count: u32,
}
```

## 十二、阶段规划（v2）

### 第一阶段：核心可用

- [x] 设计文档 v1
- [x] 设计文档 v2（调研改进）
- [ ] Cargo.toml + 依赖配置
- [ ] 配置文件读写 (config/)
- [ ] Trait 定义 (layers/)
- [ ] 默认实现 (defaults/) -- 先做 OpenAI 兼容
- [ ] 请求管道 (pipeline/)
- [ ] Axum HTTP 服务 + OpenAI 兼容 API
- [ ] SSE 流式代理
- [ ] 进程管理器 (process/)
- [ ] 多 key failover
- [ ] Virtual Key 基础功能

### 第二阶段：WASM 插件

- [ ] Extism 集成 (plugin/)
- [ ] Host 函数注册 (host_fns.rs)
- [ ] WASM <-> Trait 桥接 (bridge.rs)
- [ ] 插件热加载/卸载 API
- [ ] 示例插件：自定义路由器

### 第三阶段：生态扩展

- [ ] Anthropic 原生格式 Transform
- [ ] Weft Claw WS 协议 Transform
- [ ] WebSocket 代理
- [ ] 用量统计 + 费用追踪
- [ ] 视频/图片生成插件示例

## 十三、"核心可选"机制

UI 插件的配置中可以选择是否经过核心：

```toml
# 走核心（推荐）
[connection]
mode = "core"
core_url = "http://127.0.0.1:3000"
virtual_key = "vk-weft-001"

# 直连执行端（不需要核心）
[connection]
mode = "direct"
weft-claw_url = "ws://127.0.0.1:42617/ws/chat"
api_key = "sk-xxx"
```

## 十四、与 v1 的主要差异

| 项目 | v1 | v2 |
|------|----|----|
| WASM 运行时 | 第二阶段 wasmtime | 第一阶段 Extism |
| API 格式 | 自定义 | OpenAI 兼容（事实标准） |
| Trait 数量 | 6 个 | 4 个 Layer（更简洁） |
| Key 管理 | 直接暴露 | Virtual Key 机制 |
| 中间件模式 | 自定义 Pipeline | Tower middleware 风格 |
| 统计 | 基础 | 用量 + 费用 + 延迟 |

## 十五、参考资料

- [Extism - Universal Plug-in System](https://extism.org/)
- [Axum Web Framework](https://github.com/tokio-rs/axum)
- [LiteLLM - AI Gateway](https://github.com/BerriAI/litellm)
- [LLM Gateway](https://llmgateway.io/)
- [Tower Middleware](https://github.com/tower-rs/tower)
- [WASM Component Model 2025](https://debugg.ai/resources/wasm-component-model-2025-polyglot-plugins-secure-extensibility)
