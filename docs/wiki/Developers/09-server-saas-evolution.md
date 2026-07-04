# 09 · 服务端 SaaS 演进路径

> 路径：从 [`08-server-codebase.md`](./08-server-codebase.md) 现状演进到完整 SaaS
> 参考：[`docs/server/`](../../server/) 完整规划

---

## 1. 演进总览

```
当前 (new-api)                目标 (WEFT Cloud Server)
─────────────                 ──────────────────────────
按技术分层组织              →   按 14 业务线组织
38 家 AI 中转               →   保留 + 增强（路由策略 / 缓存退还）
8 种 OAuth                  →   保留 + 扩展（Apple / Google / 飞书 / 钉钉等）
4 套支付                    →   保留 + 增加支付宝 / 微信支付
SQLite/MySQL/PG             →   阿里云 RDS PG（统一）
Redis 自建                  →   阿里云 Tair
本地文件                    →   阿里云 OSS
（无）                      →   函数计算 FC（用户云函数）
（无）                      →   知识库（DashVector + IMM）
（无）                      →   Agent 编排 / 包仓库 / Workflow
（无）                      →   远程隧道（手机调桌面）
（无）                      →   设备管理 + 多端同步
（无）                      →   多模态生成（图像/视频/音频/数字人）
（无）                      →   团队工作区 + 6 档 RBAC
（无）                      →   统一权限模块（authz）
```

---

## 2. 14 业务线对应关系

```
旧 server/                       新 server/internal/{业务线}/
────────────────────────         ────────────────────────────
controller/auth.go         →     identity/api/auth_handler.go
controller/user.go         →     identity/api/user_handler.go
oauth/                     →     identity/provider/
model/user.go              →     identity/domain/user.go
controller/topup.go        →     billing/api/topup_handler.go
service/billing/           →     billing/service/
pkg/billingexpr/           →     billing/service/billingexpr/ (位置保留)
controller/relay.go        →     llm_gateway/api/chat_handler.go
relay/                     →     llm_gateway/relay/ (整体迁入)
service/channel/           →     llm_gateway/channel/
middleware/auth.go         →     shared/auth/
middleware/rate-limit.go   →     shared/middleware/
（新建）                    →     shared/authz/  ★ 统一权限
（新建）                    →     shared/aliyun/  ★ 阿里云 SDK
（新建）                    →     search/
（新建）                    →     knowledge/
（新建）                    →     cloud/
（新建）                    →     agent/
（新建）                    →     workflow/
（新建）                    →     multimedia/
（新建）                    →     tenant/
（新建）                    →     admin/
（新建）                    →     packages/  ★ 包仓库（严格对齐 ADR）
（新建）                    →     remote_relay/
（新建）                    →     device/
```

---

## 3. 演进的核心设计原则

### 3.1 业务领域闭环

每个业务线内部 4 层：

```
business_line/
├── api/        HTTP handler
├── service/    业务逻辑
├── repo/       DB 访问
└── domain/     模型 / 枚举
```

**业务线之间不直接 import**，通过接口抽象 + DI。

### 3.2 共享内核（shared/）

只放真正"所有业务都用"的：

```
shared/
├── db/             GORM
├── cache/          Redis (Tair)
├── auth/           JWT / API Key
├── authz/      ★  统一权限（6 层）
├── middleware/     限流 / CORS / 日志
├── errors/         统一错误码
├── response/       统一响应
├── i18n/
├── ratelimit/
└── aliyun/     ★  阿里云 SDK 封装
```

### 3.3 数据库 schema 隔离

每业务线一个 PostgreSQL schema：

```sql
identity.users
billing.topups
llm.channels
kb.documents
agent.tasks
pkg.packages
device.devices
...
```

跨 schema **禁止外键**，只在应用层关联。

### 3.4 阿里云为主

所有基础设施托管：
- ECS / ACK / FC / ECI
- RDS PG / Tair / OSS / DashVector / OpenSearch / RocketMQ
- DashScope（AI 兜底）/ IMM（解析）/ 智能语音
- WAF / DDoS / 内容安全 / KMS

详见 [`docs/server/02-aliyun-stack.md`](../../server/02-aliyun-stack.md)。

---

## 4. 5 个月迁移路径概要

详细见 [`docs/server/06-migration-plan.md`](../../server/06-migration-plan.md)。

| 月 | 重点 |
|---|---|
| M1 | 立骨架 + 阿里云接通 + relay 拷贝 |
| M2 | 迁移 identity / billing / llm_gateway 三条核心线 |
| M3 | 新建 search / cloud / knowledge / multimedia 四条线 + MVP 公测 |
| M4 | 新建 agent / workflow / packages / device / remote_relay 五条线 + 手机端 |
| M5 | 完善 tenant / admin / authz + 老代码清理 + 商业化上线 |

---

## 5. 双跑切流策略

### 5.1 路由分发器

迁移期间，新结构未实现的 endpoint 转发到老代码：

```go
// 新结构 (cmd/server/main.go)
r := gin.New()

// 已迁移：走新结构
identity.RegisterRoutes(r)
billing.RegisterRoutes(r)

// 未迁移：转发到老 controller
r.Any("/v1/*path", legacyRelay.Handler)
r.Any("/api/midjourney/*", legacyMJ.Handler)
```

### 5.2 灰度切换

```yaml
# config.yaml
migration:
  identity:
    use_new: true       # 100% 新结构
  billing:
    use_new: 0.1        # 10% 灰度
  llm_gateway:
    use_new: false      # 仍走老代码
```

---

## 6. 包系统的特殊处理

`packages/` 业务线**严格对齐 ADR 0001/0002/0003**：

- Server 是元数据/候选权威
- Core 是运行时权威
- 所有响应必带 `advisory: true` meta
- 禁止字段：`runtime_lock` / `active_generation` / `final_resolved_bindings` 等
- 完整 candidate 生命周期保留（draft → quarantined → reviewable → approved → published）
- 自动扫描代替人工审核
- Lock 备份明文存储（不解析）

### 6.1 packages-server 拆分（2026-06，拆微服务第一步）

`cmd/packages-server` 已落地为独立进程，把 `packages` 业务线从单体里抽出单独运行：

- **共享 DB**：与主 server 用同一个 `SQL_DSN`，靠 `pkg_*` 表前缀隔离行，暂不分库
- **共享会话**：复用主 server 的 cookie session store，任何持有相同 `SESSION_SECRET` 的进程都能本地校验签名 cookie，无需中心查表；API-key / access-token 路径在第一阶段 fail closed（暂不支持）
- **同源反代**：由 nginx/caddy 把 `/api/packages/*` 路由到本进程、其余给主 server，浏览器看到单一 origin，`SameSite=Strict` cookie 正常发送，前端 `api.ts`（baseURL=""）无需改动
- **硬性前提**：`SESSION_SECRET` 必须与主 server 完全一致、`SQL_DSN` 必须指向同库，否则 cookie 无法解码、请求全部 401

意义：`packages` 生产代码不依赖 `core/*` / `controller/*`，仅用 `pkg_*` 表并通过注入获取 DB / 鉴权 / 计费依赖，是单体中最接近“可独立服务”的业务线，因此被选为拆分起点。

详见：
- [`docs/server/03-package-saas-alignment.md`](../../server/03-package-saas-alignment.md)
- [`docs/architecture/vNext/adr/`](../../architecture/vNext/adr/)

---

## 7. 权限体系演进

### 当前 new-api

- 单层：身份认证（JWT + 角色 Root/Admin/User）
- 资源粒度：用户级（user_id 字段）

### SaaS 完整版（6 层）

```
① 身份认证（who you are）
② 套餐准入（subscription gate）
③ 团队 RBAC（role in team）
④ 资源所有权（visibility）
⑤ 设备权限（device capability）
⑥ Profile（客户端 Core 自治）
```

详见 [`docs/server/04-permission-system.md`](../../server/04-permission-system.md)。

---

## 8. 团队 / 工作区演进

### 当前

- `User.Group`（字符串字段，扁平分组）

### 演进到

```
tenant.workspaces           工作区
tenant.workspace_members    成员 + 6 档角色（Owner/Admin/Editor/Viewer/Billing/Guest）
```

每个业务资源带 `tenant_id` + `owner_user_id` + `visibility` + `shared_with`。

---

## 9. 计费维度扩展

### 当前

- 单一维度：LLM token 计费

### 演进到

多维度计量：

```
billing.meter_records 字段：
  meter_type    -- llm_token / kb_index / fc_invoke / image_gen / tts_seconds / ...
  amount        -- 实际消耗
  rc_consumed   -- 折成 RC 多少
  reason        -- 调用 ID / 请求 ID
```

每个业务线在自己的处理器里调 `billing.Meter` 接口扣减。

---

## 10. AI 中转的演进

### 保留

- 38 家厂商适配（relay/）
- OpenAI 兼容协议
- 流式输出
- 多 Key 轮询

### 增强

- **路由策略层**：质量 / 价格 / 速度优先
- **自动降级**：A 厂商挂了切 B
- **Prompt 缓存**：命中缓存退还 50% RC
- **BYOK**：用户带自己的 Key（5% 路由费）
- **DashScope 优先**：国内合规默认走阿里通义

详见 [`docs/server/01-architecture.md`](../../server/01-architecture.md) §3。

---

## 11. 对外契约不破坏

迁移过程中保证：

| 契约 | 不变 |
|---|---|
| `/v1/*` OpenAI 兼容 endpoint | ✅ |
| API Key 格式 | ✅ |
| JWT 格式 | ✅ |
| Stripe / Creem / Waffo / Epay webhook | ✅ |
| 现有用户数据库 | ✅ 不动表，只加 schema |
| 用户 OAuth 绑定 | ✅ |
| Steam OAuth | ✅ |

---

## 12. 关键 ADR

新增 ADR（写在 `docs/server/adr/`）：

| ADR | 决策 |
|---|---|
| 001 | 数据库按业务 schema 隔离 |
| 002 | relay/ 跟 new-api 上游 |
| 003 | 阿里云作为首选基础设施 |
| 004 | 单体起步，后续微服务化 |
| 005 | 业务线之间通过接口抽象 + DI |
| 006 | packages 模块严格对齐 ADR 0001/0002/0003 |
| 007 | 不做人工包审核，自动扫描 + 信任标签 |
| 008 | NixOS 风格包隔离（客户端） |
| 009 | Sealed Foundation 平台签名维护 |

---

## 13. 学习路径

如果你接手 SaaS 演进工作：

1. 先看 [`08-server-codebase.md`](./08-server-codebase.md) 理解 new-api 现状
2. 读 [`docs/server/01-architecture.md`](../../server/01-architecture.md) 理解目标
3. 读 [`docs/server/02-aliyun-stack.md`](../../server/02-aliyun-stack.md) 理解阿里云栈
4. 读 [`docs/server/03-package-saas-alignment.md`](../../server/03-package-saas-alignment.md) 理解包系统约束
5. 读 [`docs/server/04-permission-system.md`](../../server/04-permission-system.md) 理解权限
6. 读 [`docs/server/06-migration-plan.md`](../../server/06-migration-plan.md) 理解节奏
7. 读 [`docs/architecture/vNext/adr/`](../../architecture/vNext/adr/) 理解硬约束

---

## 14. 维护原则

- 不破坏 new-api 上游兼容（只在 relay/ 同步）
- 新功能必须在 internal/{业务线}/ 内
- 业务线之间禁止直接 import
- 数据库 schema 不跨业务关联（外键禁止）
- 所有外部依赖走 shared/aliyun/ 抽象
- ADR 0001/0002/0003 是包模块的硬约束，违反 = 阻塞合并

---

## 15. 远期演进

- 微服务化（按业务线拆服务）
- 海外区（Singapore / US / EU）
- 私有化部署 + 白标 OEM
- 行业垂直版（Code / Studio / Legal / Edu / Commerce）
- 完整 SOC 2 / GDPR 合规
