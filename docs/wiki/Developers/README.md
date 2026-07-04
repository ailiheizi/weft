# Wiki — Developers

面向**实际接手代码的开发者**的导航。每页都是某个模块/层的代码地图与解读。

## 索引

| 文件 | 内容 |
|---|---|
| [`00-architecture-overview.md`](./00-architecture-overview.md) | 全局架构鸟瞰 |
| [`01-core-runtime.md`](./01-core-runtime.md) | Core Runtime（Rust）解读 |
| [`02-plugin-system.md`](./02-plugin-system.md) | WASM 包系统解读 |
| [`03-communication-layer.md`](./03-communication-layer.md) | HTTP / SSE / WebSocket 通信层 |
| [`04-tool-skill-execution.md`](./04-tool-skill-execution.md) | 工具与 Skill 执行 |
| [`05-config-deployment.md`](./05-config-deployment.md) | 配置与部署 |
| [`06-design-vs-implementation-gaps.md`](./06-design-vs-implementation-gaps.md) | 设计 vs 实现差距 |
| [`07-package-inventory.md`](./07-package-inventory.md) | 包清单 |
| [`08-server-codebase.md`](./08-server-codebase.md) | **服务端代码地图（new-api 95k 行解读）** |
| [`09-server-saas-evolution.md`](./09-server-saas-evolution.md) | **服务端 SaaS 演进路径** |

## 阅读建议

### 新人入门
1. `00-architecture-overview.md` 看全局
2. `01-core-runtime.md` 理解 Rust 后端核心
3. `02-plugin-system.md` 理解 WASM 包系统

### 接手服务端开发
1. `08-server-codebase.md` 现状盘点
2. `09-server-saas-evolution.md` 演进路径
3. 配套：[`../../server/`](../../server/) 完整 SaaS 规划

### 接手客户端开发
1. `01-core-runtime.md` Core
2. `02-plugin-system.md` 包系统
3. `03-communication-layer.md` 通信
4. 配套：[`../../client/`](../../client/) 客户端规划

### 排查设计与实现差距
- `06-design-vs-implementation-gaps.md`

## 与 LLM Wiki 的关系

`.omc/wiki/` 是另一层级的知识库，由 wiki tools 维护（11+ 页），覆盖 `agent-core` / `core-runtime` / `frontend` / `registry-server` / `weft-claw` / `skills` / `tool-runtime-core` / `tool-web` / `package-official-vs-installed` / `weft-cloud-saas` / `registry-server-saas` / `frontend-saas-nixos` 等。

| 用途 | 看哪个 |
|---|---|
| 工程师**人工阅读**理解代码 | 本目录（`wiki/Developers/`）|
| LLM 上下文 / 跨会话知识检索 | `.omc/wiki/`（用 `wiki_query` 工具）|
