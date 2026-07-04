# 更新日志

> 说明：这里记录 `WEFT-plug/docs/` 目录本身的整理与上传口径调整，内容尽量简短。

## 2026-06-14

### 同步 core / client / server 三仓库本轮变更

- 依据 `core`：完成 WEFT 全面重命名（包名 / 二进制 / 依赖 / 路径），新增运行时 loopback token 鉴权、包权限校验与熔断（`validate.rs` / `circuit_breaker.rs` / `permissions.rs`），`packages_runtime` 扩展动态安装与版本管理
- 依据 `client`（`weft_client`）：新增 ai-director / orchestration / apps-surface / chat-workspace 功能面，包导入与在线安装链路（catalog / importer / install provider），玻璃拟态 UI 组件；移除 system_service，改用 `weft_claw_api`
- 依据 `server`：从单体重构为分层架构（controller / service / model / dto / router），按业务线拆分 `internal/features`（cloud / device / llm_gateway / multimedia / packages / remote_relay / search），引入独立 **packages-server** 进程（extension-store 拆微服务第一步，共享 DB + SESSION_SECRET、反代同源）
- 刷新 `wiki/Developers/09-server-saas-evolution.md`：补充 packages-server 拆分拓扑

## 2026-06-05

### 新增 vNext/runtime 提升实施文档（10 篇）

- 新增 `architecture/vNext/runtime/improvement-plan-2026-06-overview.md`：本批提升计划总览 + 9 篇专项索引 + 状态总表
- 新增 9 篇专项：`21-b2`（official/installed 分裂）/ `22-d2`（loopback 鉴权）/ `23-b1`（manifest validator）/ `24-a2`（熔断器）/ `25-a3`（capability 解析）/ `26-b4`（扩权重批）/ `27-b5`（渐进披露·不做分析）/ `28-c1`（config 表单）/ `29-b3`（依赖 requires 校验）
- 代码/功能变更详见 `project_changelog/2026-06-05.md`

## 2026-05-26（晚）

### docs 子目录二次整理

- `architecture/`：移动 `design-v1.md` / `design-v2.md` / `overview.md` → `architecture/legacy/`，新增 `architecture/README.md` + `architecture/legacy/README.md`
- 清理 `architecture/` 下两个遗留空目录（`background/` / `current/`）
- `plans/`（顶层单文件目录）合并进 `superpowers/plans/`，删除空目录
- 新增 `superpowers/README.md` + `specs/README.md` + `plans/README.md`：每个文件一句话简介 + 状态标记（active / partial / done / historical / superseded）
- 新增 `brainstorms/README.md` / `comms/README.md` / `project_changelog/README.md`：单文件目录加索引

## 2026-05-26

### docs 目录整体重组（SaaS 化规划）

- 新增 `saas-features/`：SaaS 六层规划（AI 能力 / 云资源 / 知识库 / Agent / 行业 / 合规），含 24 个月路线图
- 新增 `server/`：服务端 SaaS 化规划，14 业务线架构 + 阿里云栈 + 包系统 ADR 对齐 + 权限体系 + AI 原生分发 + 5 个月迁移路径
- 新增 `client/`：客户端规划，手机端简化 + AI 询问列表 + NixOS 风格包隔离 + 首次离线包 + Sealed Foundation
- 新增 `wiki/Developers/08-server-codebase.md`：服务端代码地图（new-api 95k 行解读）
- 新增 `wiki/Developers/09-server-saas-evolution.md`：服务端 SaaS 演进路径
- 新增 `.omc/wiki/` 三页：`weft-cloud-saas` / `registry-server-saas` / `frontend-saas-nixos`
- 散文件归类：
  - `agent-guide.md` / `build-guide.md` → `guides/`
  - `weft-productization-requirements.md` → `legacy/`（部分被 saas-features 替代）
  - `git-hygiene-2026-04-24.md` / `git-tracked-artifact-cleanup-plan-2026-04-24.md` → `chores/`
- 重写 `README.md`：按业务/角色组织导航

### 来源说明

- 基于与决策层多轮讨论后产生的规划性文档
- 不引入实现代码变更
- 与 `architecture/vNext/adr/` 严格对齐

## 2026-05-23

### weft_client 实时执行步骤展示

- `core/src/api/mod.rs`：新增 `/api/stream/events` 端点，直接读 SQLite 绕过 WASM 锁，实现 send_message 执行期间的实时事件轮询
- `packages/official/agent-core`：`MAX_AGENT_TOOL_ROUNDS` 从 8 提升至 20，支持更复杂的多步任务
- `packages/official/weft-claw`：补充 `delete_session` / `reset_session` action
- `packages/official/session-events`：`latest_seq` 改为查全局 `MAX(seq)`，不再受 `limit` 参数影响
- `clients/weft_client`：Flutter 客户端改用增量事件轮询，工具调用实时显示 Running… / 完成状态，去除打字机动画延迟

## 2026-04-23

### docs 目录上传口径整理

- 明确 `docs/` 作为当前仓库内可直接上传的文档入口
- 将根目录说明调整为直接指向现有 `architecture/` 与 `superpowers/specs/`
- 保留蓝图、设计稿、实施计划三类主要可上传内容
- 补充更新规则，要求后续更新时优先读取 `core/` 与 `packages/weft-claw/` 的已 push 提交

### 来源说明

- `core`：按已 push 提交判断架构主线是否需要更新文档
- `packages/weft-claw`：按已 push 提交判断产品蓝图与声明是否需要更新文档
- `packages/weft-code`：当前仅作为声明/设计快照来源，不按独立 git 历史追踪
