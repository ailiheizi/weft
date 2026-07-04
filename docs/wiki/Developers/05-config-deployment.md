# 配置、部署与启动

## 配置文件层次

### 文件位置

| 文件 | 用途 |
|------|------|
| `config/config.toml` | 活跃运行时配置 |
| `config/config.dev.toml` | 开发环境配置 |
| `config/config.example.toml` | 带注释的模板 |

### 完整配置结构

```toml
[core]
host = "127.0.0.1"        # 绑定 IP
port = 42617               # 绑定端口
log_level = "info"         # trace | debug | info | warn | error
data_dir = "./data"        # 数据目录

[routing]
default_provider = "deepseek"   # 默认 LLM 提供商
default_model = "deepseek-chat" # 默认模型

[[providers]]
name = "deepseek"
base_url = "https://api.deepseek.com"
format = "openai"               # openai | anthropic
api = "chat_completions"        # chat_completions | responses
keys = [{ value = "sk-...", label = "primary" }]
models = ["deepseek-chat", "deepseek-reasoner"]

[key_strategy]
mode = "failover"           # failover | round_robin | random

[fallback]
retry_count = 2
switch_key = true
switch_provider = true
priority = ["deepseek", "openai"]

[[virtual_keys]]
key = "dev-key-1"
label = "Development"
provider = "deepseek"
model = "deepseek-chat"
max_rpm = 100
max_budget = 50.0

[[services]]
name = "companion-core"
command = "python"
args = ["server.py"]
auto_start = true
health_url = "http://127.0.0.1:43127/health"

[[plugins]]
name = "skills-runtime"
path = "packages/official/skills"
enabled = true

[registry]
gitea_url = "https://gitea.alhz.org"
package_source_url = "local://packages"

[plugin_aliases]
skills = "skills-runtime"
mcp = "mcp-client"
memory = "memory-store"
agent_runtime = "agent-runtime"
channels = "channel-core"
scheduler = "cron"
companion_runtime = "companion-core"
```

## CLI 工具

### `weft` - 客户端 CLI

```
weft gen current  <app>            显示当前 generation
weft gen list     <app>            列出所有 generations
weft gen diff     <app> <f> <t>    Diff 两个 generations
weft gen generate <app>            生成新 generation
weft gen verify   <app>            验证当前 generation
weft gen activate <app> [<id>]    激活 generation
weft gen rollback <app>            回滚 generation

weft scene create <app> <scene>   创建 scene
weft scene bind   <app> <scene>   绑定 scene
weft scene list   <app>           列出 scenes
weft scene show   <app> <scene>   查看 scene 详情

选项:
  --core-url URL   Core 服务器地址 (默认 http://127.0.0.1:42617)
```

### `weft-rpc` - JSON-RPC stdio 服务器

无 CLI 参数。从 stdin 读取 JSON-RPC 2.0 请求，逐行返回响应。

支持的方法：
- `health` / `health/check`
- `model/list` / `models/list`

### `weft-sign` - 包签名工具

```
weft-sign <private-key-hex> <name> <version> <sha512> <source>
weft-sign digest <source>
```

## 部署模式

### 1. 源码树运行 (Standalone)

```bash
cd core
cargo run
# repo_root = CARGO_MANIFEST_DIR/..
# config: repo_root/config/config.toml
```

### 2. 托管桌面模式 (Managed Desktop)

```pwsh
$env:WEFT_CORE_MANAGED = "1"
.\weft-core.exe
# repo_root = CWD
# config: CWD/config/config.toml
# 由 Electron 桌面前端作为子进程启动
```

### 3. SDK 嵌入模式 (Library)

```rust
use weft_core::WeftCore;

let core = WeftCore::load(WeftCoreOptions {
    config_path: "config.toml".into(),
    repo_root: "./".into(),
    load_mode: SdkLoadMode::Lightweight, // 无网络，无插件发现
    start_services: false,
})?;

// 程序化调用
let response = core.chat(chat_request).await?;
let models = core.models();

// 或启动 HTTP 服务器
let router = core.router();
axum::serve(listener, router).await?;
```

### 4. JSON-RPC stdio 模式

```bash
weft-rpc
# 从 stdin 读取 JSON-RPC 请求
# 输出到 stdout
# 适合编辑器/IDE 集成 (LSP 风格)
```

## 启动流程 (详细)

### 步骤 1-5: 基础初始化
1. Tracing 初始化 (`tracing-subscriber`, env filter)
2. Runtime root 检测
3. 配置加载 (`config.toml`)
4. Pipeline 构建 (Router + KeySelector + Transform + ErrorHandler)
5. ProcessManager + VirtualKeyStore 初始化

### 步骤 6-10: 插件发现
6. 包索引解析 (`packages/index.toml`)
7. 插件发现 (扫描 `packages/` 目录下所有 `plugin.toml`)
8. 注册 Chat 频道提供者
9. 注册 Service 插件到 ProcessManager (不启动)
10. 加载 WASM 插件到 Extism 主机

### 步骤 11-15: 产品激活
11. 产品包解析 (扫描 `package.toml` 文件)
12. 能力注册表构建
13. Profile 检测 (从 app config 读取)
14. KV 存储加载 (`data/plugin_kv.json`)
15. 原生插件加载 (`.dll`/`.so`/`.dylib`)

### 步骤 16-20: 运行时启动
16. Generation 存储构建 (`.weft/<app>/lock.toml`)
17. AppState 组装
18. WASM 主机注入 (AppState → host 函数)
19. 自动启动服务 (`auto_start: true`)
20. HTTP 服务器启动 (Axum, graceful shutdown on Ctrl+C)

### 步骤 21-22: 后台任务
21. Cron ticker (每 10 秒)
    - 调 WASM cron 插件的 `tick()`
    - 每 30 ticks (~5 分钟): 清理过期记忆
22. KV 保存器 (每 5 分钟)
    - 写入 `data/plugin_kv.json`

## Profile 系统

| Profile | Rank | 能力限制 |
|---------|------|---------|
| Safe | 0 (默认) | 无文件系统读写、无命令执行、无原生插件 |
| Developer | 1 | 允许 `core.files` + `core.execution` |
| Trusted | 2 | 允许 `core.native_execution` (原生 .dll/.so/.dylib) |

配置方式: 在 `.weft/<app>/config.toml` 的 `app_runtime.profile` 设置

```toml
[app_runtime]
profile = "developer"
```

## 数据目录布局

```
data/
├── plugin_kv.json             # WASM 插件持久化 KV 存储
├── global-sense/
│   └── pc-status.json         # 系统状态
├── source-cache/
│   └── app-source-index.json  # 来源索引缓存
├── registry-server.*.log      # Registry 服务日志
├── weft-core.*.log           # Core 运行时日志
├── __pycache__/               # Python 字节码缓存
└── (generated tool scripts)
```

## 配置解析顺序

```
config.toml (文件) → 默认值 (Rust Default trait) → 验证 (validate_config)
                                                        ↓ 失败
                                                   全默认值回退
```

- 无环境变量覆盖单个配置键
- 无 CLI 参数覆盖配置值
- `WEFT_CORE_MANAGED=1` 只控制 runtime root (CWD)
- `RUST_LOG` 控制 tracing 日志级别
