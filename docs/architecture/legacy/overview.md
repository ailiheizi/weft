# WEFT Architecture Overview

## System Architecture

WEFT is designed as a modular, extensible AI service orchestration proxy with a plugin-first architecture. The system consists of three main components:

1. **Core Service** - Rust-based HTTP proxy with pipeline architecture
2. **Plugin System** - WASM-based extensibility layer
3. **Desktop Application** - Electron + Vue 3 management interface

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Client Layer                             │
│  (OpenAI SDK, curl, custom clients)                             │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             │ HTTP/WebSocket
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                      WEFT Core Service                          │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                   HTTP API Layer (Axum)                   │  │
│  │  /v1/chat/completions  /v1/models  /api/*                │  │
│  └────────────────────────┬─────────────────────────────────┘  │
│                           │                                     │
│  ┌────────────────────────▼─────────────────────────────────┐  │
│  │                  Request Pipeline                         │  │
│  │                                                            │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐ │  │
│  │  │  Router  │→ │   Key    │→ │Transform │→ │  Error   │ │  │
│  │  │  Layer   │  │ Selector │  │  Layer   │  │ Handler  │ │  │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘ │  │
│  │       ↑              ↑             ↑             ↑        │  │
│  │       │              │             │             │        │  │
│  │  ┌────┴──────────────┴─────────────┴─────────────┴────┐  │  │
│  │  │         Plugin System (WASM/Extism)                 │  │  │
│  │  │  Plugins can override any layer implementation      │  │  │
│  │  └─────────────────────────────────────────────────────┘  │  │
│  └────────────────────────┬─────────────────────────────────┘  │
│                           │                                     │
│  ┌────────────────────────▼─────────────────────────────────┐  │
│  │              Configuration & State                        │  │
│  │  • Providers  • Virtual Keys  • Process Manager          │  │
│  └───────────────────────────────────────────────────────────┘  │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             │ HTTP Requests
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Provider Layer                              │
│  OpenAI  Anthropic  DeepSeek  OpenRouter  Custom Providers      │
└─────────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. HTTP API Layer

Built on Axum web framework, provides:

- **OpenAI-Compatible Endpoints**
  - `POST /v1/chat/completions` - Chat completion (streaming & non-streaming)
  - `GET /v1/models` - List available models

- **Management Endpoints**
  - `GET /api/health` - Health check
  - `GET /api/providers` - List providers
  - `GET /api/services` - List managed services
  - `GET /api/plugins` - List plugins
  - `GET /api/virtual-keys` - List virtual keys
  - `POST /api/plugins/install` - Install plugin
  - `GET /api/plugins/registry` - Plugin marketplace

- **WebSocket Endpoints**
  - `/ws` - Real-time communication with plugins

### 2. Request Pipeline

The pipeline is the heart of WEFT, processing every request through 4 layers:

#### Layer 1: RouterLayer
**Responsibility**: Select which provider to use

**Default Implementation**: `DefaultRouter`
- Matches model name to provider's model list
- Falls back to `default_provider` if no match
- Respects `x_provider` header for forced routing

**Plugin Override**: Plugins can implement custom routing logic (e.g., load balancing, A/B testing)

#### Layer 2: KeySelectorLayer
**Responsibility**: Choose which API key to use

**Default Implementations**:
- `FailoverSelector` - Try keys in order until one works
- `RoundRobinSelector` - Distribute requests evenly across keys
- `RandomSelector` - Random key selection

**Plugin Override**: Plugins can implement custom key selection (e.g., cost-based, quota-aware)

#### Layer 3: TransformLayer
**Responsibility**: Convert request/response formats

**Default Implementations**:
- `OpenAITransform` - Pass-through for OpenAI-compatible APIs
- `AnthropicTransform` - Convert OpenAI ↔ Anthropic Messages API

**Plugin Override**: Plugins can add custom provider support

#### Layer 4: ErrorHandlerLayer
**Responsibility**: Handle errors and implement retry logic

**Default Implementation**: `DefaultErrorHandler`
- 429 (rate limit) → Switch to next key
- 5xx (server error) → Retry with exponential backoff
- 4xx (client error) → Fail immediately

**Plugin Override**: Plugins can implement custom error handling strategies

### 3. Plugin System

WEFT uses Extism for WASM plugin execution, providing:

**Features**:
- Sandboxed execution environment
- Language-agnostic (Rust, Go, C, C++, etc.)
- Hot reload without service restart
- Host function API for plugin-to-core communication

**Plugin Lifecycle**:
1. Load WASM file from `plugins/installed/`
2. Call `plugin_info()` to discover capabilities
3. Register plugin exports (init, route, select_key, handle_error, handle_ws_message)
4. Call `init()` on startup
5. Route requests through plugin-overridden layers

**Host Functions** (available to plugins):
- `host_log(level, message)` - Logging
- `host_http_request(request)` - Make HTTP requests
- `host_storage_get/set(key, value)` - Persistent storage
- `host_auth_verify(token)` - Authentication

### 4. Configuration System

**Configuration Files**:
- `config/config.toml` - Production configuration
- `config/config.dev.toml` - Development configuration
- `config/config.test.toml` - Test configuration

**Configuration Structure**:
```toml
[core]
host = "127.0.0.1"
port = 3004

[[providers]]
name = "openai"
base_url = "https://api.openai.com/v1"
format = "openai"
keys = ["sk-..."]
models = ["gpt-4", "gpt-3.5-turbo"]

[routing]
default_provider = "openai"

[key_strategy]
type = "failover"

[[virtual_keys]]
key = "vk-..."
name = "My Key"
rate_limit = 100

[[services]]
name = "weft-claw"
command = "node"
args = ["backend.js"]
working_dir = "./data/weft-claw"

[[plugins]]
name = "agent-core"
path = "./plugins/installed/agent-core"
enabled = true
```

**Hot Reload**: Configuration changes are detected and applied without restart (where possible)

### 5. Virtual Key System

Virtual keys abstract real API keys and provide:

- **Rate Limiting** - Per-key request limits
- **Usage Tracking** - Monitor key usage
- **Access Control** - Restrict key access to specific models/providers
- **Key Rotation** - Automatic key rotation on failure

**Virtual Key Flow**:
```
Client Request (vk-xxx) → Validate Virtual Key → Map to Real Keys → Select Key → Make Request
```

### 6. Process Manager

Manages lifecycle of external services (e.g., weft-claw, airi):

**Features**:
- Automatic startup on core service start
- Health monitoring with configurable checks
- Automatic restart on failure
- Graceful shutdown
- Log capture and forwarding

**Service Configuration**:
```toml
[[services]]
name = "weft-claw"
command = "node"
args = ["backend.js"]
working_dir = "./data/weft-claw"
health_check_url = "http://127.0.0.1:3005/health"
health_check_interval = 30
auto_restart = true
```

## Data Flow

### Non-Streaming Request Flow

```
1. Client sends POST /v1/chat/completions
   ↓
2. Axum handler validates request
   ↓
3. Pipeline.execute() starts
   ↓
4. RouterLayer selects provider (e.g., "openai")
   ↓
5. KeySelectorLayer chooses API key
   ↓
6. TransformLayer converts request format (if needed)
   ↓
7. HTTP client sends request to provider
   ↓
8. Provider responds
   ↓
9. TransformLayer converts response format
   ↓
10. Response returned to client
```

**Error Path**:
```
7. HTTP client gets error (e.g., 429)
   ↓
8. ErrorHandlerLayer decides action (e.g., retry with different key)
   ↓
9. Loop back to step 5 with new key
```

### Streaming Request Flow

```
1. Client sends POST /v1/chat/completions with stream=true
   ↓
2. Axum handler validates request
   ↓
3. Pipeline.execute_stream() starts
   ↓
4-6. Same as non-streaming (Router → KeySelector → Transform)
   ↓
7. HTTP client opens SSE stream to provider
   ↓
8. For each chunk:
   - TransformLayer converts chunk format
   - Stream chunk to client via SSE
   ↓
9. Stream completes
```

## Plugin Architecture

### Plugin Types

WEFT supports two types of plugins:

#### 1. Pipeline Plugins
Override one or more pipeline layers:
- Router plugins
- Key selector plugins
- Transform plugins
- Error handler plugins

#### 2. Service Plugins
Provide standalone functionality:
- Agent management (agent-core)
- Memory systems (memory)
- Skill execution (skills)
- Communication channels (channels)
- MCP integration (mcp-client)
- Scheduled tasks (cron)

### Plugin Communication

**Plugin → Core**:
- Host functions (logging, HTTP, storage, auth)
- Return values from layer functions

**Core → Plugin**:
- Layer function calls (route, select_key, transform, handle_error)
- WebSocket messages (handle_ws_message)
- Lifecycle events (init, shutdown)

**Plugin → Plugin**:
- Via core as intermediary
- Shared storage
- Event system (future)

## Security Model

### Sandboxing
- WASM plugins run in isolated sandbox
- No direct file system access
- No direct network access (must use host functions)
- Limited memory and CPU

### Authentication
- Virtual keys for client authentication
- API keys stored encrypted (future)
- Plugin authentication via host functions

### Rate Limiting
- Per-virtual-key rate limits
- Per-provider rate limits
- Global rate limits

## Performance Considerations

### Optimization Strategies

1. **Connection Pooling** - Reuse HTTP connections to providers
2. **Request Batching** - Batch multiple requests when possible
3. **Caching** - Cache provider responses (future)
4. **Async I/O** - Tokio async runtime for high concurrency
5. **WASM Performance** - Near-native execution speed

### Scalability

**Vertical Scaling**:
- Single instance can handle 1000+ req/s
- Memory usage: ~50MB base + ~5MB per plugin

**Horizontal Scaling** (future):
- Stateless design allows multiple instances
- Shared configuration via distributed store
- Load balancer in front of multiple instances

## Monitoring & Observability

### Metrics (future)
- Request count per provider
- Request latency percentiles
- Error rates
- Key usage statistics
- Plugin execution time

### Logging
- Structured logging with tracing
- Log levels: ERROR, WARN, INFO, DEBUG, TRACE
- Plugin logs forwarded to core logger

### Health Checks
- `/api/health` endpoint
- Provider health status
- Service health status
- Plugin health status

## Deployment Architecture

### Single Instance Deployment
```
┌─────────────────────────────────────┐
│         Server / Container          │
│                                     │
│  ┌──────────────────────────────┐  │
│  │      weft-core              │  │
│  │      (port 3004)             │  │
│  └──────────────────────────────┘  │
│                                     │
│  ┌──────────────────────────────┐  │
│  │   Managed Services           │  │
│  │   (weft-claw, airi, etc.)     │  │
│  └──────────────────────────────┘  │
│                                     │
│  ┌──────────────────────────────┐  │
│  │   Plugins (WASM)             │  │
│  └──────────────────────────────┘  │
└─────────────────────────────────────┘
```

### Multi-Instance Deployment (future)
```
                ┌──────────────┐
                │ Load Balancer│
                └──────┬───────┘
                       │
        ┌──────────────┼──────────────┐
        │              │              │
   ┌────▼────┐    ┌────▼────┐    ┌────▼────┐
   │ weft-1 │    │ weft-2 │    │ weft-3 │
   └────┬────┘    └────┬────┘    └────┬────┘
        │              │              │
        └──────────────┼──────────────┘
                       │
              ┌────────▼────────┐
              │ Shared Config   │
              │ (Redis/etcd)    │
              └─────────────────┘
```

## Technology Stack

### Core Service
- **Language**: Rust 1.75+
- **Web Framework**: Axum 0.8
- **Async Runtime**: Tokio
- **HTTP Client**: Reqwest
- **WASM Runtime**: Extism 1.9
- **Serialization**: Serde + serde_json
- **Configuration**: TOML

### Desktop Application
- **Framework**: Electron
- **UI**: Vue 3 + TypeScript
- **Build Tool**: Vite
- **State Management**: Pinia
- **Styling**: Tailwind CSS

### Plugin SDK
- **Language**: Rust
- **Target**: wasm32-wasip1
- **PDK**: Extism PDK

## Future Enhancements

### Planned Features
1. **Advanced Caching** - Response caching with TTL
2. **Request Deduplication** - Merge identical concurrent requests
3. **Cost Tracking** - Track costs per provider/key/client
4. **A/B Testing** - Route percentage of traffic to different providers
5. **Circuit Breaker** - Automatic provider failover on repeated failures
6. **Distributed Tracing** - OpenTelemetry integration
7. **Plugin Marketplace** - Enhanced discovery and ratings
8. **Multi-Tenancy** - Isolated environments for different users

### Research Areas
1. **AI-Powered Routing** - ML-based provider selection
2. **Semantic Caching** - Cache based on semantic similarity
3. **Adaptive Rate Limiting** - Dynamic limits based on usage patterns
4. **Plugin Composition** - Chain multiple plugins together
