# WEFT Core

WEFT 核心服务 — Rust AI 服务编排代理。

暴露 OpenAI 兼容 API（`/v1/chat/completions`），将请求路由到多个 LLM 提供商（OpenAI、Anthropic、DeepSeek、OpenRouter 等），支持 API Key 故障转移/轮询策略，以及 WASM 插件（Extism）覆盖核心管道行为。

## 构建

```bash
cargo build --release
```

## 运行

```bash
# 需要 config/config.toml 配置文件
cargo run --release
# 默认监听 127.0.0.1:3004
```

## 配置

复制 `config/config.example.toml` 为 `config/config.toml`，填入 provider API key：

```toml
[core]
host = "127.0.0.1"
port = 3004

[[providers]]
name = "deepseek"
base_url = "https://api.deepseek.com"
format = "openai"
keys = ["sk-xxx"]
models = ["deepseek-chat", "deepseek-coder"]
```

## 请求管道

```
Request → Router → KeySelector → Transform → HTTP → Transform (response)
                                                  ↘ ErrorHandler (on failure)
```

每一层都可以被 WASM 插件覆盖。

## API

| 路径 | 方法 | 说明 |
|------|------|------|
| `/v1/chat/completions` | POST | OpenAI 兼容聊天（支持 SSE 流式） |
| `/v1/models` | GET | 列出所有模型 |
| `/api/health` | GET | 健康检查 |
| `/api/providers` | GET | 列出提供商 |
| `/api/plugins` | GET | 列出已加载插件 |
| `/api/chat-providers` | GET | 列出聊天提供者（插件） |

## 插件

WASM 插件放在 `plugins/installed/` 目录下，启动时自动扫描加载。插件通过 `plugin.toml` 声明能力：

```toml
[plugin]
name = "my-plugin"
version = "0.1.0"
entry = "plugin.wasm"
provides = ["chat_channel"]
```

## 许可证

Private
