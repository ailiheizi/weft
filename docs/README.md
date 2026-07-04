# WEFT 文档目录

这里是 `WEFT-plug` 仓库内的文档目录，沉淀架构主线、产品蓝图、SaaS 规划、设计稿与实施资料。

不是独立文档仓库，而是**源码仓库内的文档入口**。

> 仓库总览、组件入口与开发命令见根 [`../README.md`](../README.md)。本目录聚焦架构/产品/规划层面的细节。

---

## 📍 快速导航（按你是谁）

| 我要做什么 | 优先看 |
|---|---|
| 全局画面 | [`architecture/vNext/00-overview.md`](./architecture/vNext/00-overview.md) + [`wiki/Developers/00-architecture-overview.md`](./wiki/Developers/00-architecture-overview.md) |
| SaaS 商业化规划 / 给老板看 | [`saas-features/`](./saas-features/) |
| 服务端规划 / 接手 server 开发 | [`server/`](./server/) + [`wiki/Developers/08-server-codebase.md`](./wiki/Developers/08-server-codebase.md) |
| 客户端规划（手机端 / 包隔离 / 离线包） | [`client/`](./client/) |
| 现有客户端代码（Electron + Core + WASM） | [`wiki/Developers/`](./wiki/Developers/) |
| 架构决策 / 设计原则 | [`architecture/vNext/adr/`](./architecture/vNext/adr/) |
| 怎么构建 / 怎么跑 | [`guides/`](./guides/) |
| 当前活跃任务 | [`../.plans/`](../.plans/) |

---

## 📁 目录结构

```
docs/
├── README.md                ← 本文件，全局导航
├── CHANGELOG.md             ← 文档级变更
├── UPDATE_RULES.md          ← 文档维护规则
│
├── architecture/            稳定架构主线
│   ├── README.md            导航
│   ├── vNext/               ★ 当前主线（adr / foundations / runtime / products / workflows）
│   └── legacy/              早期设计（design-v1/v2/overview，已替代）
│
├── wiki/Developers/         开发者向（实际看代码的文档，含 server 代码地图）
│
├── saas-features/           🆕 SaaS 商业化规划（六层规划 + 路线图）
├── server/                  🆕 服务端架构规划（重构 / 阿里云 / 迁移）
├── client/                  🆕 客户端规划（手机端 / 询问列表 / NixOS 隔离）
│
├── superpowers/             产品设计稿 + 实施计划（带索引 + 状态标记）
│   ├── README.md
│   ├── specs/               设计稿
│   └── plans/               实施计划
│
├── guides/                  使用指南（agent-guide / build-guide）
├── comms/                   跨团队沟通文档
├── project_changelog/       项目变更日志（按日期）
│
├── brainstorms/             草稿性头脑风暴（默认不作正式上传）
├── legacy/                  早期文档（被新文档替代，保留作历史）
└── chores/                  仓库维护记录（git 清理等）
```

---

## 🏛 主线架构（architecture/vNext/）

当前主线，关键文档：

- [`vNext/00-overview.md`](./architecture/vNext/00-overview.md) — 主线入口
- [`vNext/foundations/03-package-model.md`](./architecture/vNext/foundations/03-package-model.md) — 包模型
- [`vNext/foundations/13-product-package-instance-model.md`](./architecture/vNext/foundations/13-product-package-instance-model.md) — Product/Package/Instance 关系
- [`vNext/runtime/04-declaration-config-lock.md`](./architecture/vNext/runtime/04-declaration-config-lock.md) — 三层文件模型（声明 / 配置 / Lock）
- [`vNext/runtime/05-binding-and-resolution.md`](./architecture/vNext/runtime/05-binding-and-resolution.md) — Binding 解析
- [`vNext/runtime/06-profiles-and-policy.md`](./architecture/vNext/runtime/06-profiles-and-policy.md) — Profile / Policy
- [`vNext/runtime/16-package-service.md`](./architecture/vNext/runtime/16-package-service.md) — Package Server 设计
- [`vNext/runtime/19-generation-manager.md`](./architecture/vNext/runtime/19-generation-manager.md) — Generation 状态机

**ADR**（不可违反的架构决策）：

- [`adr/0001`](./architecture/vNext/adr/0001-product-package-server-authority.md) — Package Server 权威边界
- [`adr/0002`](./architecture/vNext/adr/0002-package-server-review-publication-boundary.md) — 审核/发布边界
- [`adr/0003`](./architecture/vNext/adr/0003-package-server-client-experience-boundary.md) — 客户端体验边界

---

## 💼 SaaS 商业化（saas-features/）

为"积分制 AI + 云服务 SaaS"准备的六层规划。每章独立成档。

- [`saas-features/README.md`](./saas-features/README.md) — 总览 + 全景图 + 投资视角
- `01-ai-capability.md` — AI 能力层
- `02-cloud-resource.md` — 云资源层
- `03-knowledge-office.md` — 知识库 / 办公层
- `04-agent-automation.md` — Agent / 自动化层
- `05-vertical-industry.md` — 行业垂直层
- `06-business-compliance.md` — 商业 / 合规 / 增值层
- `07-priority-roadmap.md` — 优先级 + 24 个月路线图

---

## 🖥 服务端规划（server/）

服务端重构 + 阿里云栈 + 迁移路径。

- [`server/README.md`](./server/README.md) — 服务端规划总览 + 决策汇总
- `01-architecture.md` — 14 业务线 + 模块职责 + 依赖图 + DB schema
- `02-aliyun-stack.md` — 阿里云产品清单 + 三档成本
- `03-package-saas-alignment.md` — packages 模块对齐 ADR
- `04-permission-system.md` — 6 层权限体系
- `05-ai-native-distribution.md` — AI 原生包分发（NixOS + ai-manifest）
- `06-migration-plan.md` — 5 个月迁移路径

---

## 📱 客户端规划（client/）

桌面 + 手机 + 包系统演进。

- [`client/README.md`](./client/README.md) — 客户端规划总览
- `01-mobile-app.md` — 手机端简化方案
- `02-ai-suggestion-list.md` — AI 询问列表
- `03-nixos-package-isolation.md` — NixOS 风格包隔离
- `04-offline-bundle.md` — 首次离线安装包
- `05-sealed-foundation.md` — 不可更改的 AI 底层

---

## 📚 Wiki（wiki/Developers/）

开发者向的代码地图。

| 文件 | 内容 |
|---|---|
| `00-architecture-overview.md` | 全局架构鸟瞰 |
| `01-core-runtime.md` | Core Runtime（Rust）解读 |
| `02-plugin-system.md` | WASM 包系统解读 |
| `03-communication-layer.md` | HTTP / SSE / WebSocket 通信层 |
| `04-tool-skill-execution.md` | 工具与 Skill 执行 |
| `05-config-deployment.md` | 配置与部署 |
| `06-design-vs-implementation-gaps.md` | 设计 vs 实现差距 |
| `07-package-inventory.md` | 包清单 |
| `08-server-codebase.md` | **服务端代码地图（new-api 95k 行解读）** |
| `09-server-saas-evolution.md` | **服务端 SaaS 演进路径** |

LLM Wiki（`.omc/wiki/`）由 wiki tools 维护，包含：`agent-core` / `core-runtime` / `frontend` / `registry-server` / `weft-claw` / `skills` / `tool-runtime-core` / `tool-web` / `package-official-vs-installed` / `weft-cloud-saas` / `registry-server-saas` / `frontend-saas-nixos` 等。

---

## 📖 设计稿与实施计划（superpowers/）

### specs/ — 产品级设计稿

- `2026-04-10-weft-claw-package-blueprint-design.md`
- `2026-04-15-weft-code-design.md`
- `2026-04-26-openclaw-web-search-provider-design.md`
- ... 等

### plans/ — 实施计划

- `2026-04-09-vnext-phase-5-weft-claw-productization.md`
- `2026-04-13-vnext-product-package-instance-implementation.md`
- `2026-04-15-weft-code-implementation.md`
- ... 等

---

## 🧭 使用指南（guides/）

- [`guides/agent-guide.md`](./guides/agent-guide.md) — Agent / AI 协作者指南
- [`guides/build-guide.md`](./guides/build-guide.md) — 从源码构建 Weft

---

## 🗂 其他目录

| 目录 | 用途 |
|---|---|
| `comms/` | 跨团队沟通文档 |
| `plans/` | 顶层计划（与 superpowers/plans 区分） |
| `project_changelog/` | 项目变更日志（按日期） |
| `brainstorms/` | 头脑风暴草稿（不作正式上传） |
| `legacy/` | 早期文档（被新文档替代） |
| `chores/` | 仓库维护记录（git 清理等） |

---

## 🚀 当前活跃任务（`.plans/`）

如果你正在处理具体任务，先看对应 `../.plans/<任务名>/`，不是本目录。

---

## ✏️ 维护原则

1. **新增 SaaS 内容**：放 `saas-features/` / `server/` / `client/`，不要塞 `architecture/`
2. **稳定架构 / ADR**：放 `architecture/vNext/`
3. **设计稿 + 实施计划**：放 `superpowers/`
4. **使用指南**：放 `guides/`
5. **活跃任务执行**：放 `../.plans/`
6. **草稿 / 头脑风暴**：放 `brainstorms/`（不作上传）
7. **历史文档**：放 `legacy/`（保留但标记过时）
8. **仓库维护记录**：放 `chores/`
9. **更新前看** [`UPDATE_RULES.md`](./UPDATE_RULES.md)
10. **变更追加** [`CHANGELOG.md`](./CHANGELOG.md)
