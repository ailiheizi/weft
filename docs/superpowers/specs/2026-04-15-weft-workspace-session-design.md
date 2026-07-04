# WEFT Workspace Session Design

**Date:** 2026-04-15

## Goal

为 WEFT Desktop 增加真正的桌面工作区层，让顶部从“session tabs”变为“workspace tabs”，左侧 session 列表只显示当前 workspace 下的会话，并让 terminal 在创建时继承当前 workspace 根目录。

这次不是视觉改名，而是机制改造：形成 `workspace -> sessions -> terminals` 的正式模型。

## Current State

- 顶部 tab 目前直接渲染所有 chat sessions。
- 左侧 sidebar 直接基于全局 `chatStore.sessions` 列表，没有工作区过滤。
- `ChatSession` 没有 `workspaceId`。
- `agent-core` 的 `SessionRecord` 没有 workspace 元数据。
- terminal 会话已经绑定到 session，但 PTY 创建时没有 `cwd` 入参，默认从用户主目录启动。
- WEFT-core 已有 capability 级别的 `workspace_root/cwd` 能力，但桌面端没有将其接成工作区产品机制。

## Target Mechanism

### 1. Desktop Workspace Layer

新增 `DesktopWorkspace`：

- `id`
- `name`
- `rootPath`
- `createdAt`
- `updatedAt`
- `lastActiveSessionId`

桌面端始终存在一个默认工作区：

- `name`: `Global`
- `rootPath`: `null`

该默认工作区承接所有旧 session，避免历史数据丢失。

### 2. Session Ownership

`ChatSession` 新增：

- `workspaceId`

所有 session 必须归属于某个 workspace。创建 session 时直接归属当前激活 workspace，不保留“无工作区 session”的兼容状态。

### 3. Workspace UI

顶部 `ChatPanel` 改为 workspace tabs：

- tab 切换的是 workspace，不是 session
- 顶部右侧原 `+` 改为文件夹语义按钮
- 点击后调用系统目录选择器，创建并切到新 workspace

左侧 `Sidebar` 保持 session 列表职责，但只显示当前 workspace 的 sessions。

### 4. Terminal Behavior

terminal 仍然按 session 维护多终端并排状态，不改现有 split 模型。

变化点只有一条：

- `terminal:create` 支持 `cwd`
- 新 terminal 默认使用当前 session 所属 workspace 的 `rootPath`
- 如果 workspace 没有 `rootPath`，回退到当前默认主目录

已打开 terminal 不支持在运行中切换工作区根目录。这与 VS Code 和主流 agent 工具的实际约束一致。

### 5. Persistence

工作区列表使用桌面本地存储持久化。

session 的 `workspaceId/workspaceRoot` 进入 agent-core session 记录，使：

- renderer reload 后不丢
- main 进程 list/create session 时有权威元数据
- 后续 direct/delegate/tool 调用若需要 workspace root，可以沿 session 元数据继续向 core 能力透传

## Data Model Changes

### Shared Types

- 新增 `DesktopWorkspace`
- `ChatSession` 新增 `workspaceId`

### Renderer Store

`chatStore` 新增：

- `workspaces`
- `activeWorkspaceId`
- `activeWorkspace`
- `filteredSessions`
- `createWorkspace`
- `switchWorkspace`
- `ensureDefaultWorkspace`
- `openWorkspaceFolder`

### Main/API

`WeftCoreSessionSummary` / `WeftCoreSessionConfig` / session store row 新增：

- `workspaceId`
- `workspaceRoot`

### Agent Core

`SessionRecord` / `CreateSessionInput` 新增：

- `workspace_id`
- `workspace_root`

旧记录读取时默认填充：

- `workspace_id = "workspace-global"`
- `workspace_root = ""`

## Migration

迁移规则简单且不可选：

1. 初始化工作区列表时确保存在默认 `Global` 工作区
2. 所有旧 session 若无 `workspaceId`，统一归到默认工作区
3. 旧 session 若无 `workspaceRoot`，按空字符串处理
4. 不做双轨兼容，不保留“全局 session”与“workspace session”并存的逻辑分叉

## Testing

### Contract Tests

- `weft/scripts/test-workspace-session-contract.ts`
- `weft/scripts/test-terminal-workspace-cwd-contract.ts`

### Verification Gates

- `npx tsx weft/scripts/test-workspace-session-contract.ts`
- `npx tsx weft/scripts/test-terminal-workspace-cwd-contract.ts`
- `npm run typecheck:ui` in `weft/`

## Scope Boundaries

本次只做工作区机制接入：

- 顶部 workspace tab
- sidebar session 过滤
- session 归属持久化
- terminal cwd 继承

不在本次范围内：

- 多根工作区 `.code-workspace` 语义
- 已打开 terminal 动态迁移 cwd
- tool 调用链更深层的 workspace root 透传细化
- memory runtime 改动

## Recommended Implementation Order

1. shared/main/agent-core 数据结构补齐
2. renderer store 加入 workspace 层与迁移
3. terminal IPC 加 `cwd`
4. ChatPanel/Sidebar 切到 workspace 视图
5. 回归测试与 UI typecheck
