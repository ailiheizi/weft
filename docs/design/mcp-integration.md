# MCP 一键接入设计

> Status: Draft  
> Author: ailiheizi  
> Date: 2026-07-03

---

## 1. 用户体验目标

### 1.1 核心场景

| 场景 | 操作 | 预期结果 |
|------|------|----------|
| 新用户首次配置 | 设置 → MCP 服务器 → 浏览推荐列表 → 点击"启用" | Server 自动下载依赖、启动、工具注入 agent |
| 进阶用户自定义 | "添加自定义" → 填 command + args → 保存 | Server 注册、连接验证通过、工具可用 |
| 导入已有配置 | "导入" → 选择 `claude_desktop_config.json` 或 `mcp.json` | 批量导入所有 server 配置 |
| AI 自动使用 | 用户提问触发 MCP 工具 | Agent 自动发现并调用对应 MCP tool，无需用户干预 |
| 故障恢复 | Server 崩溃 | 自动重启，UI 显示状态变化，最多重试 3 次 |

### 1.2 用户流程

```
设置页 MCP 区块
├── 推荐服务器列表（卡片式，带图标/描述/一键启用）
├── 已启用列表（状态灯 + 工具数 + 启用/禁用开关）
├── "添加自定义" 入口（command/args/env 表单）
└── "导入配置" 入口（支持 Claude Desktop / Cursor 格式）
```

**关键体验约束：**
- 一键启用 = 零配置可用（推荐列表的 server 预置好所有参数）
- 启用后 < 5 秒内工具可用（显示 loading 状态）
- 失败时给出可操作的错误信息（如 "需要安装 Node.js"）

---

## 2. 架构设计

### 2.1 整体分层

```
┌─────────────────────────────────────────────────────────┐
│  Flutter Client                                          │
│  McpPanel UI → McpProvider (Riverpod) → McpApi          │
└────────────────────────┬────────────────────────────────┘
                         │ REST API
┌────────────────────────▼────────────────────────────────┐
│  Core Runtime                                            │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────┐ │
│  │ MCP Manager  │  │ Process Pool │  │ Tool Registry │ │
│  │ (新增模块)    │  │ (进程生命周期) │  │ (工具注入)     │ │
│  └──────┬───────┘  └──────┬───────┘  └───────┬───────┘ │
│         │                  │                   │         │
│  ┌──────▼──────────────────▼───────────────────▼──────┐ │
│  │           mcp-client WASM Package (ext.mcp)         │ │
│  └─────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────┘
                         │
          ┌──────────────┼──────────────┐
          ▼              ▼              ▼
    [stdio server]  [http server]  [sse server]
```

### 2.2 配置存储方案

**选择：独立 `mcp-servers.toml` 文件 + KV 运行时缓存**

理由：
- 与 `config.toml`（含密钥引用）解耦，可安全分享和版本管理
- 文件格式人类可读可编辑，兼容 git 管理
- 运行时仍以 KV 为准（启动时从文件加载到 KV）
- 支持导入/导出标准 JSON 格式

```
.weft/                          # 工作区目录（现有 .weft/ 运行时契约保持不变）
├── mcp-servers.toml            # MCP server 声明式配置
└── mcp-servers.lock.json       # 工具缓存（加速启动，可重生成）
```

**加载优先级：**
1. 工作区级 `.weft/mcp-servers.toml`（当前项目/场景专属）
2. 全局 `~/.weft/mcp-servers.toml`（用户级默认）
3. 内置推荐列表（代码内嵌，fallback）

### 2.3 进程生命周期管理

当前问题：`call_tool` 每次 spawn 新进程，性能差。

**改进方案：持久化进程池**

```
启动流程：
  config 加载 → 标记为 enabled 的 server → 按需启动（首次 call 或手动 start）
  
进程状态机：
  stopped → starting → running → stopping → stopped
                ↓                    ↓
              failed              crashed → auto_restart (max 3)

健康检查：
  - stdio: 定期发送 `ping` JSON-RPC（间隔 30s）
  - http/sse: HEAD 请求到 url（间隔 30s）
  - 超时 10s 无响应 → 标记 unhealthy → 重启
```

**核心改动点（mcp-client WASM 包）：**
- `call_tool` 复用已启动进程而非每次重新 spawn
- 新增 `health_check` 函数
- 进程 stdout/stderr 缓冲管理（避免 pipe buffer 满阻塞）

### 2.4 MCP 工具注入 Agent 工具列表

```
Agent 请求 tools → Core 组装工具列表：
  1. 内置 capabilities 的 tools（现有）
  2. Package 声明的 tools（现有）
  3. MCP servers 的 tools（新增注入点）
     └─ 遍历 enabled servers → 读取 cached tools → 合并
        namespace: "mcp:{server_name}:{tool_name}"
```

**工具发现时机：**
- Server 启动成功后立即 `tools/list`，结果写入 `mcp-servers.lock.json` 缓存
- Agent 请求工具列表时优先读缓存，缓存 miss 则实时探测
- 工具列表变更通过 WebSocket 推送到客户端

### 2.5 与现有 Package 系统的关系

| 维度 | Package (WASM) | MCP Server |
|------|---------------|------------|
| 运行方式 | WASM sandbox 内执行 | 独立进程/远程服务 |
| 权限模型 | `package.toml` 声明 | 进程级隔离 + 可选 tool allowlist |
| 生命周期 | Core 加载即可用 | 需要 spawn/connect |
| 工具暴露 | Capability registry | `tools/list` JSON-RPC |
| 安装方式 | 编译为 `.wasm` 放入 `packages/` | 配置 command/url 即可 |

**结论：MCP 不是 Package 的子类型，而是与 Package 平级的另一种工具来源。** 两者通过统一的 Tool Registry 对外暴露给 Agent。`mcp-client` WASM 包作为桥接层保持不变（它本身是一个 Package，对外提供 `ext.mcp` capability）。

---

## 3. 数据模型

### 3.1 MCP Server Config（`mcp-servers.toml` 格式）

```toml
# 全局配置
[settings]
auto_start = true              # 启动 core 时自动启动 enabled servers
max_restart_attempts = 3
health_check_interval = 30     # 秒

# Server 声明
[servers.filesystem]
enabled = true
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/dir"]
env = {}
description = "文件系统读写"
auto_start = true

[servers.web-search]
enabled = true
transport = "stdio"
command = "npx"
args = ["-y", "@anthropic/mcp-server-web-search"]
env = { "BRAVE_API_KEY" = "${BRAVE_API_KEY}" }
description = "Brave 网络搜索"

[servers.remote-api]
enabled = false
transport = "http"
url = "https://example.com/mcp"
headers = { "Authorization" = "Bearer ${MCP_TOKEN}" }
description = "远程 MCP 服务"
```

**设计要点：**
- env 值支持 `${VAR}` 语法引用环境变量（不存储明文密钥）
- `enabled` 字段实现一键启用/禁用（不删除配置）
- 每个 server 可独立设置 `auto_start`

### 3.2 推荐列表格式（内嵌 + 可远程更新）

```toml
# packages/official/mcp-client/presets.toml

[[presets]]
id = "filesystem"
name = "文件系统"
icon = "folder"
category = "core"
description = "读写本地文件和目录"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem"]
requires_args = [{ name = "path", description = "根目录路径", type = "path" }]
env = {}
popularity = 95

[[presets]]
id = "sqlite"
name = "SQLite"
icon = "database"
category = "data"
description = "查询和操作 SQLite 数据库"
transport = "stdio"
command = "npx"
args = ["-y", "@anthropic/mcp-server-sqlite"]
requires_args = [{ name = "db_path", description = "数据库文件路径", type = "file" }]
env = {}
popularity = 80

[[presets]]
id = "web-search"
name = "网络搜索"
icon = "search"
category = "web"
description = "通过 Brave Search 搜索互联网"
transport = "stdio"
command = "npx"
args = ["-y", "@anthropic/mcp-server-web-search"]
requires_args = []
env = { "BRAVE_API_KEY" = "" }
requires_env = ["BRAVE_API_KEY"]
popularity = 90

[[presets]]
id = "github"
name = "GitHub"
icon = "git"
category = "dev"
description = "操作 GitHub issues、PRs、repos"
transport = "stdio"
command = "npx"
args = ["-y", "@anthropic/mcp-server-github"]
requires_args = []
env = { "GITHUB_TOKEN" = "" }
requires_env = ["GITHUB_TOKEN"]
popularity = 85

[[presets]]
id = "playwright"
name = "浏览器自动化"
icon = "globe"
category = "web"
description = "Playwright 驱动浏览器交互"
transport = "stdio"
command = "npx"
args = ["-y", "@anthropic/mcp-server-playwright"]
requires_args = []
env = {}
popularity = 75
```

**字段说明：**
- `requires_args`: 用户必须提供的参数（UI 渲染为输入框）
- `requires_env`: 必须配置的环境变量（UI 渲染为密钥输入框，标注 🔑）
- `category`: 分组展示（core / data / web / dev / ai）
- `popularity`: 排序权重

### 3.3 运行时状态模型

```rust
// 内存中的 server 运行状态（非持久化）
struct McpServerState {
    name: String,
    status: ServerStatus,          // Stopped | Starting | Running | Failed | Unhealthy
    pid: Option<u32>,              // stdio 进程 PID
    started_at: Option<DateTime>,
    last_health_check: Option<DateTime>,
    restart_count: u32,
    tools: Vec<McpToolDef>,        // 缓存的工具定义
    error: Option<String>,         // 最近错误信息
}

enum ServerStatus {
    Stopped,
    Starting,
    Running,
    Failed { reason: String },
    Unhealthy,
}
```

---

## 4. API 设计

### 4.1 现有 API 保持不变

现有 `POST /api/capabilities/ext.mcp/call` 继续作为底层调度入口。以下为新增的高层管理 API。

### 4.2 新增 REST API

```
# ====== Server CRUD ======

GET    /api/mcp/servers
  → 200: { servers: [{ name, transport, enabled, status, tool_count }] }

POST   /api/mcp/servers
  Body: { name, transport, command?, args?, url?, headers?, env?, enabled? }
  → 201: { server: {...} }

PUT    /api/mcp/servers/{name}
  Body: { enabled?, command?, args?, env?, ... }
  → 200: { server: {...} }

DELETE /api/mcp/servers/{name}
  → 204

# ====== 生命周期 ======

POST   /api/mcp/servers/{name}/start
  → 200: { status: "running" }

POST   /api/mcp/servers/{name}/stop
  → 200: { status: "stopped" }

POST   /api/mcp/servers/{name}/restart
  → 200: { status: "running" }

GET    /api/mcp/servers/{name}/health
  → 200: { status, last_check, uptime, restart_count }

# ====== 工具发现 ======

GET    /api/mcp/tools
  Query: ?server={name}  (可选，不传则返回所有 enabled server 的工具)
  → 200: { tools: [{ server, name, description, input_schema }] }

POST   /api/mcp/tools/{server}/{tool}/call
  Body: { arguments: {...} }
  → 200: { content: [...] }

# ====== 推荐列表 ======

GET    /api/mcp/presets
  → 200: { presets: [{ id, name, icon, category, description, requires_args, requires_env }] }

POST   /api/mcp/presets/{id}/install
  Body: { args?: {...}, env?: {...} }
  → 201: { server: {...}, status: "running" }

# ====== 配置导入导出 ======

POST   /api/mcp/import
  Body: { format: "claude_desktop" | "cursor", config: {...} }
  → 200: { imported: ["server1", "server2"], skipped: [] }

GET    /api/mcp/export?format=claude_desktop
  → 200: { mcpServers: {...} }
```

### 4.3 WebSocket 事件推送

通过现有 WebSocket 通道（`/ws`）推送 MCP 状态变更：

```json
{ "type": "mcp:server_status", "server": "filesystem", "status": "running", "tools": 5 }
{ "type": "mcp:server_status", "server": "web-search", "status": "failed", "error": "BRAVE_API_KEY not set" }
{ "type": "mcp:tools_updated", "server": "filesystem", "tools": ["read_file", "write_file", ...] }
```

---

## 5. UI 设计

### 5.1 设置页 MCP 区块

**位置：** 设置（Settings）→ MCP 服务器（作为独立 Tab 或区块，与现有 McpPanel 整合）

**布局结构：**

```
┌─ MCP 服务器 ──────────────────────────────────────────────┐
│                                                            │
│  [开关] 启用 MCP 功能              [导入配置 ▾]            │
│                                                            │
│  ── 推荐服务器 ────────────────────────────────────────── │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐        │
│  │ 📁      │ │ 🔍      │ │ 🗄️      │ │ 🐙      │        │
│  │文件系统  │ │网络搜索  │ │ SQLite  │ │ GitHub  │        │
│  │ [启用]  │ │ [配置]  │ │ [启用]  │ │ [配置]  │        │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘        │
│                                                            │
│  ── 已配置 ───────────────────────────────────────────── │
│  ┌────────────────────────────────────────────────────┐   │
│  │ 🟢 filesystem    5 tools    [禁用] [删除]          │   │
│  │ 🔴 web-search    错误:KEY   [重试] [配置] [删除]   │   │
│  │ 🟡 sqlite        启动中...                         │   │
│  └────────────────────────────────────────────────────┘   │
│                                                            │
│  [+ 添加自定义服务器]                                      │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

### 5.2 推荐列表卡片

每张卡片展示：
- 图标 + 名称
- 一句话描述
- 状态：未安装 → "启用" 按钮 / 需配置 → "配置" 按钮（引导填 API key）/ 已启用 → 绿色勾
- 点击卡片展开详情（完整描述 + 提供的工具列表预览）

**需要额外输入的 preset：**
- 点击"配置"后弹出 bottom sheet
- 展示 `requires_args`（如路径选择器）和 `requires_env`（密钥输入框）
- 填完后一键启用

### 5.3 自定义添加表单

```
┌─ 添加自定义 MCP 服务器 ───────────────────────┐
│                                                │
│  名称:      [________________]                 │
│  传输方式:  [stdio ▾]                          │
│                                                │
│  ── stdio 配置 ──                              │
│  命令:      [npx______________]                │
│  参数:      [-y, @scope/server]  [+ 添加]      │
│                                                │
│  ── 环境变量（可选）──                          │
│  [KEY______] = [VALUE______]     [+ 添加]      │
│                                                │
│  [取消]                    [测试连接] [保存]    │
└────────────────────────────────────────────────┘
```

- 传输方式切换 stdio/http 时动态显示不同字段
- "测试连接"按钮：spawn → initialize → tools/list → 展示工具数 → 停止
- 验证通过后"保存"才可点击

### 5.4 工具列表集成

在 Agent 的 Chat Settings → Tools tab 中：
- MCP 工具以 server 名分组展示
- 每个工具显示名称 + 描述 + 来源 server
- 支持单独禁用某个 MCP 工具（`tool_allowlist` / `tool_denylist`）

---

## 6. 实现步骤

### Phase 1: 配置持久化 + 进程池（2 周）

**目标：** 解决最大痛点——配置丢失和重复 spawn。

| 任务 | 改动位置 | 说明 |
|------|----------|------|
| 1.1 设计 `mcp-servers.toml` schema | `core/src/app/` | 新增 `McpServersConfig` 结构体 |
| 1.2 启动时加载配置到 KV | `core/src/app/resolve.rs` | 读取文件 → 写入 `mcp:servers:{agent}` |
| 1.3 持久化进程池 | `mcp-client/src/lib.rs` | `call_tool` 复用 running process，不再每次 spawn |
| 1.4 健康检查 + 自动重启 | `mcp-client/src/lib.rs` | 定时 ping，失败重启（max 3） |
| 1.5 新增管理 API | `core/src/api/` | CRUD + start/stop/restart endpoints |
| 1.6 配置变更写回文件 | `core/src/app/` | API 修改后同步更新 `.toml` 文件 |

**验证：**
- `cargo build` 通过
- 手动配置 `mcp-servers.toml` → 重启 core → server 自动恢复
- 单个 server 处理 100 次 `call_tool` 无重新 spawn

### Phase 2: 推荐列表 + 客户端 UI（2 周）

**目标：** 一键启用体验。

| 任务 | 改动位置 | 说明 |
|------|----------|------|
| 2.1 嵌入 presets 数据 | `mcp-client/presets.toml` | 首批 8-10 个推荐 server |
| 2.2 Presets API | `core/src/api/` | GET /presets + POST /presets/{id}/install |
| 2.3 重构 McpPanel UI | `clients/weft_client/` | 推荐卡片 + 已配置列表 + 状态灯 |
| 2.4 自定义添加表单 | `clients/weft_client/` | stdio/http 双模式 + 测试连接 |
| 2.5 配置导入 | `core/src/api/` | 支持 `claude_desktop_config.json` 格式解析 |
| 2.6 WebSocket 状态推送 | `core/src/ws/` | server 状态变更实时通知客户端 |

**验证：**
- `flutter analyze` 通过
- 从推荐列表一键启用 filesystem server → 工具可用
- 导入 Claude Desktop 配置文件 → 所有 server 正确注册

### Phase 3: 工具自动发现 + Agent 集成（1.5 周）

**目标：** AI 自动感知并使用 MCP 工具。

| 任务 | 改动位置 | 说明 |
|------|----------|------|
| 3.1 工具注入 Agent context | `core/src/` (tool assembly) | Agent 工具列表合并 MCP tools |
| 3.2 工具缓存 + lock 文件 | `mcp-client/` | `mcp-servers.lock.json` 加速启动 |
| 3.3 工具变更推送 | `core/src/ws/` | server 启停时通知 Agent 重载工具 |
| 3.4 per-tool 权限控制 | `mcp-servers.toml` | `allowed_tools` / `denied_tools` 字段 |
| 3.5 Chat UI 工具气泡 | `clients/weft_client/` | 现有 `mcp:server:tool` 气泡完善 |
| 3.6 错误反馈优化 | 全链路 | server down 时工具调用返回友好错误 |

**验证：**
- Agent 对话中自动使用 MCP 工具（无需用户手动选择）
- Server 崩溃 → 自动重启 → 工具调用恢复
- 禁用某工具后 Agent 不再尝试调用

### Phase 4: 增强（后续迭代）

- OAuth 2.1 认证流程（MCP 2025 规范）
- `resources/list` + `resources/read` 支持
- `prompts/list` + `prompts/get` 支持
- Streamable HTTP transport 完整实现
- MCP server 沙箱隔离（限制文件系统访问范围）
- 推荐列表远程更新（从 registry 拉取）

---

## 7. 风险与决策记录

| 决策 | 选择 | 理由 |
|------|------|------|
| 配置文件格式 | TOML | 与 `config.toml` 风格一致，人类友好 |
| 配置文件位置 | `.weft/mcp-servers.toml` | 工作区级隔离，不与运行时契约冲突 |
| 进程管理层 | 在 WASM 包内（`mcp-client`） | 保持架构不变，进程操作需要 `process=true` 权限 |
| 工具命名空间 | `mcp:{server}:{tool}` | 与现有 tool_registry 命名一致 |
| 推荐列表存储 | 内嵌 toml + 可选远程更新 | Phase 2 先内嵌，后续加 registry |
| 标准格式兼容 | 只做导入，不改自有格式 | 自有格式更灵活（支持 enabled/health 等扩展） |

**已知风险：**
1. npx 首次运行下载慢 → 可在 preset 中标注预估时间，UI 展示进度
2. WASM 内 spawn 进程能力受 host runtime 限制 → 已有 `process=true` 权限，需测试边界
3. 多 agent 共享 server pool 的并发安全 → Phase 1 先单 agent，后续加锁
