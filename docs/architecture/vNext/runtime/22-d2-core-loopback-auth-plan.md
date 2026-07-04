# D2：Core 本地 loopback token 鉴权 — 方案

> 状态：调研完成，落点锁定；**代码未落地**（涉及 core 编译验证 + 多个易回归放行点，待确认执行）。
> 关联：[`06-profiles-and-policy.md`](../runtime/06-profiles-and-policy.md)（本地信任模型）、A1（host_fn 权限，同源"本地信任"范畴）。

## 1. 方向校正（重要）

原任务"API Key 安全存储（client flutter_secure_storage）"经实测是**伪需求**：
- core 的 `list_providers`/`get_provider`（`core/src/api/providers.rs:21,40`）**只返回 `key_count`，从不回显 key 明文**。
- client（`core_repository.dart:67-89`）拿不到已存 key，只在录入时 POST 一次；输入框已有 `obscureText`。
- → client 从不持久化 key，没有"明文落本地"问题可修。

**真正缺口**：core `/api/*` 全无鉴权（已确认无任何 auth middleware）。任何本机进程可：
- `GET /api/providers` 枚举 provider、`POST /api/providers` 覆盖配置
- `POST /api/apps/{name}/run`、`POST /v1/chat/completions` 白嫖已配置的 LLM 额度
- `POST /api/shutdown` 关闭 core、`PUT /api/packages/{name}/config` 改包配置

## 2. 方案：loopback token

### 2.1 token 生成与分发
- core 启动时（`main.rs`，AppState 组装处）生成随机 token（如 32 字节 hex），写入 `data-dir/runtime-token`（权限 600）。
- 已存在则复用（重启不失效，避免 client 每次重读）。
- client 启动时读同一路径（client 与 core 共享 data-dir，桌面同机）→ 注入 Dio。
  - 备选：core 在 `/api/health`（放行端点）响应里**不**返回 token（避免泄露）；client 读文件。

### 2.2 axum 鉴权 layer 落点
`core/src/api/mod.rs:399` 现为 `.layer(cors).with_state(state)`。在 cors 前插入 `axum::middleware::from_fn_with_state(state, auth_guard)`，校验 `Authorization: Bearer <token>`。

### 2.3 放行清单（关键，否则打挂 UI/WS/探活）
| 端点 | 原因 | 处理 |
|---|---|---|
| `/health`, `/api/health` | client 启动前探活、无 token 阶段 | **放行** |
| `/api/apps/{name}/ui[/*path]` | WebView 加载 weft-claw-ui 静态资源，`<script src>` 不带 header | **放行**（或同源 cookie） |
| `/packages/{package_name}/ui[/*path]` | 同上，包 UI 静态资源 | **放行** |
| `/ws/packages/{package_name}` | 浏览器 WebSocket 握手不能自定义 header | **token 走 query 参数**校验 |
| `/v1/chat/completions`, `/v1/models` | OpenAI 兼容，外部工具用 `Authorization: Bearer` | 复用同一 token（语义天然契合） |
| 其余 `/api/*` | 管理面 | **强制鉴权** |

### 2.4 client 改动面
- **只给连本地 core 的 Dio 加 token**：`apiClientProvider`（`client.dart`，baseUrl=127.0.0.1:3004）加 interceptor 注入 `Authorization`。
- **store_client 连远程 Registry，绝不能带这个 token** —— 确认 `store_client.dart` 用独立 Dio 实例（已确认 client.dart 注释提到 "core client + store client" 共享 AppErrorInterceptor 但 baseUrl 不同，需确保 token interceptor 只挂 core 实例）。
- 受影响调用面：`core_repository.dart` 全部、`weft_claw_api.dart` 全部、`stream/tokens`+`stream/events` 轮询（D1 新增的也走 core Dio，自动带上）。

## 3. 风险
| 风险 | 说明 | 缓解 |
|---|---|---|
| WebView 白屏 | UI 静态资源加鉴权 → `<script>` 401 | 放行 `*/ui[/*]`（见 2.3） |
| WebSocket 断连 | WS 握手无 header | token 走 query |
| store_client 误带 token | 远程请求泄露本地 token | interceptor 只挂 core Dio 实例 |
| 首次无 token 阶段 | client 读 token 前的探活 | 放行 health |
| 与 profile 模型重叠 | 本地信任语义应与 A1/profile 统一 | 标注：token 是"传输层门禁"，profile 是"能力授予"，两层不冲突可共存 |

## 4. 落地顺序（建议）
1. core：token 生成/读取 util + `auth_guard` middleware + 放行清单 + `mod.rs` 插 layer
2. core：`cargo build` + `cargo test`（build_router 的 test 模块需补 token 或放行）验证
3. client：core Dio 注入 interceptor，确认 store_client 不受影响
4. 联调：起 core + client，验证正常请求通过、无 token 请求 401、UI/WS 正常

## 5. 验证边界
本文档为方案；代码未写。落地需编译运行 core 实测（含 build_router 现有 test 的适配）。

<!-- D2-IMPL-RESULT-2026-06-05 -->
## 6. 实施结果（2026-06-05，已落地 + 运行时验证）

> §5 的"代码未写"已过时。D2 已完整落地、运行时验证、并修复了一次测试回归。

### 落地
- core：`AppState` 加 `data_dir` / `runtime_token` / `runtime_token_path`；`main.rs` 启动时 `resolve_data_dir` + `ensure_runtime_token`（生成/复用 `<data-dir>/runtime-token`，默认 `<repo>/data/runtime-token`）。
- `api/mod.rs`：`require_loopback_token` middleware（`from_fn_with_state`）接入 `build_router`。放行清单 `route_is_unprotected`：`/health`、`/api/health`、`/v1/models`、`/api/apps/*/ui[/*]`、`/packages/*/ui[/*]`；WebSocket `/ws/packages/*` 走 query `token=` 校验；其余 `/api/*` 强制 Bearer。
- `weft.rs`：`build_client()` 按 `WEFT_TOKEN` env → `data/runtime-token` → `runtime-token` 读 token 注入 Authorization 头（修复鉴权开启后 CLI 全线 401 的回归）。
- client：`client.dart` 的 `_LoopbackAuthInterceptor` 只挂 core Dio（127.0.0.1:3004），多路径查找 token 文件；store_client 独立 Dio 不带 token。

### 运行时验证（鉴权矩阵实测）
起 core 实测：`/api/health` 无 token → 200；`/api/providers` 无 token → 401；带正确 token → 200；错 token → 401。token 文件正常生成。

### ⚠️ 测试回归与修复（关键）
落地时**只验了"编译通过"，未跑完整测试套件** → 完整 `cargo test --lib` 暴露 **24 个 API handler 测试失败**（app_detail / generations / scenes / activation_plan 等全部 401）。
根因：`require_loopback_token` 的逻辑分支为"有 token 校验、否则落到 401"，导致 **`runtime_token: None` 也返回 401**；且 D2 落地时把多个测试 AppState 的 token 填成了 `Some("test-token")` 但测试请求不带 token。
修复（两处）：
1. middleware：`runtime_token: None` → **直接放行**（loopback auth 未配置 token = 未启用；利于 dev/test 与未配置部署，token 文件存在才"武装"鉴权）。
2. 4 个测试文件（app_detail/generations/mod/scenes）的 AppState `runtime_token` 改回 `None`（这些测试测业务，不应被鉴权干扰）。

### 最终验证
- core：`cargo test --lib` → **272 passed, 0 failed**（含修复后；另顺带修了 1 个**预先存在**的 `app::resolve` 测试 fixture bug —— `packages` Vec 缺 session-events DiscoveredPackage，与 D2 无关，git stash 验证确认非本次引入）。
- client：`dart analyze lib` → 0 error。

### 教训
"编译通过" ≠ "测试通过"。API 层改动（尤其全局 middleware）必须跑完整套件，针对性单测 + `--no-run` 会漏掉运行时回归。

