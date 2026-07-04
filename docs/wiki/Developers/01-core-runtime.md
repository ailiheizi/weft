# Core Rust 运行时架构

## 二进制目标

| Binary | 源文件 | 用途 |
|--------|--------|------|
| `weft-core` | `core/src/main.rs` | 主服务器守护进程 |
| `weft` | `core/src/bin/weft.rs` | CLI 客户端（generation/scene 管理） |
| `weft-rpc` | `core/src/bin/weft-rpc.rs` | JSON-RPC stdio 服务器 |
| `weft-sign` | `core/src/bin/weft-sign.rs` | 包签名工具 |

## 插件运行时分类

| 运行时 | 格式 | 通信方式 | 示例 |
|--------|------|---------|------|
| **WASM** | `.wasm` (Extism) | WebSocket / host 函数 | mcp-client, agent-core |
| **Service** | Python/Node.js 进程 | HTTP webhook (core 代理) | companion-core, memory-runtime, context-engine, js-extension-runtime |

## 模块结构

```
core/src/
├── main.rs          # 守护进程入口，启动流程
├── lib.rs           # 库入口
├── sdk.rs           # SDK/嵌入 API
├── api/             # HTTP API 路由
│   ├── mod.rs       # 路由注册 (70+ 路由)
│   ├── openai_compat.rs  # OpenAI 兼容层
│   ├── plugin_ws.rs      # WebSocket 插件调用
│   ├── plugin_webhook.rs # HTTP webhook 插件调用
│   ├── services.rs       # 服务代理 (webhook)
│   ├── plugins.rs        # 插件安装/卸载/重载
│   ├── js_extension_runtime.rs # JS 扩展代理
│   ├── scenes.rs         # Scene 管理
│   ├── apps.rs           # App 管理
│   ├── generations.rs    # Generation 管理
│   ├── suites.rs         # Suite 管理
│   └── virtual_keys.rs   # 虚拟密钥管理
├── config/          # 配置系统
│   └── mod.rs       # AppConfig, CoreConfig, ProviderConfig 等
├── defaults/        # 默认实现
│   ├── mod.rs
│   ├── router.rs    # 默认路由
│   ├── transforms/openai.rs  # OpenAI 格式转换
│   └── error_handler.rs     # 错误处理
├── pipeline/        # 请求管道
│   └── mod.rs       # Pipeline 结构 (Router → Selector → Transform → Handler)
├── layers/          # 管道中间件层
│   ├── key_selector.rs   # API Key 选择 (failover, round_robin, random)
│   └── error_handler.rs  # 错误处理与重试
├── plugin/          # 插件系统
│   ├── mod.rs       # 插件发现、加载、ServiceConfig 构建
│   ├── bridge.rs    # WASM Host 函数 (host_call_plugin, host_exec 等)
│   └── config.rs    # 插件 manifest 解析
├── process/         # 进程管理
│   └── mod.rs       # 服务进程生命周期 (start/stop/healthcheck)
├── services/        # Service 插件目录
│   ├── memory-runtime/      # 持久化记忆服务
│   └── js-extension-runtime/ # JS 扩展工具运行时
├── rpc/             # JSON-RPC
│   └── mod.rs       # JSON-RPC 2.0 处理 (stdio)
├── app/             # App/Generation/Scene 管理
│   └── ...
└── vkeys/           # 虚拟密钥
    └── mod.rs
```

## 启动流程（简化版）

1. Tracing 初始化
2. 检测 runtime root (managed vs source tree)
3. 加载 `config/config.toml`
4. 构建 Pipeline (Router + KeySelector + OpenAI Transform + ErrorHandler)
5. 初始化 ProcessManager + VirtualKeyStore
6. 解析 `packages/index.toml` 发现所有插件
7. 加载 WASM 插件到 Extism 主机
8. 注册 Service 插件到 ProcessManager
9. 解析 Product packages (weft-claw, weft-code 等)
10. 构建能力注册表
11. 检测 Profile (Safe/Developer/Trusted)
12. 自动启动 auto_start 服务
13. 启动 Axum HTTP 服务器
14. 启动后台任务 (cron tick, KV save)

## HTTP API 路由

### OpenAI 兼容
| 方法 | 路径 | 用途 |
|------|------|------|
| POST | `/v1/chat/completions` | LLM 聊天补全 |
| GET | `/v1/models` | 模型列表 |

### 插件系统
| 方法 | 路径 | 用途 |
|------|------|------|
| GET | `/ws/plugins/{name}` | WebSocket 插件 RPC |
| POST | `/api/plugins/{name}/call` | HTTP 插件调用 |
| POST | `/api/plugins/{name}/webhook` | HTTP webhook |
| POST | `/api/plugins/{name}/webhook/{channel}` | webhook (带通道) |
| POST | `/api/plugins/{name}/reload` | 重载插件 |
| POST | `/api/plugins/{name}/toggle` | 启用/禁用 |
| DELETE | `/api/plugins/{name}` | 卸载 |
| POST | `/api/plugins/install` | 安装 |
| GET | `/api/plugins/{name}/dependencies` | 依赖信息 |

### 服务管理
| 方法 | 路径 | 用途 |
|------|------|------|
| GET | `/api/services` | 列出所有服务 |
| POST | `/api/services/{name}/webhook` | 代理到服务 |
| POST | `/api/services/{name}/start` | 启动服务 |
| POST | `/api/services/{name}/stop` | 停止服务 |

### App/Scene/Generation
| 方法 | 路径 | 用途 |
|------|------|------|
| GET | `/api/apps` | 列出 apps |
| GET | `/api/apps/{app}/scenes` | 列出 scenes |
| POST | `/api/apps/{app}/scenes` | 创建 scene |
| GET | `/api/apps/{app}/scenes/{scene}` | 查看 scene |
| POST | `/api/apps/{app}/scenes/{scene}/bind` | 绑定 scene |
| GET | `/api/apps/{app}/generations` | 列出 generations |
| GET | `/api/apps/{app}/generations/current` | 当前 generation |
| POST | `/api/apps/{app}/generations/generate` | 生成新 generation |
| POST | `/api/apps/{app}/generations/activate` | 激活 generation |

### 其他
| 方法 | 路径 | 用途 |
|------|------|------|
| GET | `/api/health` | 健康检查 |
| GET | `/api/status` | 运行状态 |
| GET | `/api/packages` | 列出包 |
| GET | `/api/suites` | 列出套餐 |
| POST | `/api/virtual-keys` | 创建虚拟密钥 |

## Pipeline 架构

请求经过 4 层管道：

```
Request
  ↓
Router (provider + model 选择)
  ↓
KeySelector (API key 选择, 支持 failover/round_robin/random)
  ↓
Transform (格式转换, OpenAI ↔ Anthropic)
  ↓
ErrorHandler (重试, key 切换, provider 切换)
  ↓
Response
```

### 错误恢复策略 (ErrorAction)
| Action | 触发条件 |
|--------|---------|
| RetrySame | 临时错误 (429 rate limit, 5xx) |
| SwitchKey | Key 耗尽 |
| SwitchProvider | Provider 不可用 |
| Fail | 不可恢复错误 |

## JSON-RPC (weft-rpc)

基于 stdio 的 JSON-RPC 2.0，每行一个请求/响应。

### 支持的方法
- `health` / `health/check` — 返回 `{"status":"ok","version":"..."}`
- `model/list` / `models/list` — 返回模型列表

### 用途
编辑器/IDE 集成场景（类似 LSP），无需 HTTP 服务器。

## Process Manager

管理所有 `runtime = "service"` 的插件进程。

### 功能
- 进程生命周期 (start/stop/restart)
- 健康检查 (HTTP health URL 定期探测)
- 崩溃重启
- stdin/stdout/stderr 读写
- 状态查询 (Running/Stopped/Unhealthy)

### ServiceConfig
```rust
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

## Profile 系统

| Profile | Rank | 限制 |
|---------|------|------|
| Safe | 0 (默认) | 无文件系统写入、无命令执行、无原生插件 |
| Developer | 1 | 允许文件系统、命令执行 |
| Trusted | 2 | 允许原生插件加载 (.dll/.so/.dylib) |

Profile 从 app config 读取，默认为 Safe。
