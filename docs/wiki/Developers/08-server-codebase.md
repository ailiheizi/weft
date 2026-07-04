# 08 · 服务端代码地图（new-api 现状）

> 路径：`server/`
> 语言：Go 1.25 + Gin + GORM
> 来源：基于 [QuantumNous/new-api](https://github.com/QuantumNous/new-api) 改造
> 规模：590 个 Go 文件，约 95,000 行代码
> 端口：3000（默认）/ 3007（旧版）

---

## 1. 一句话定位

> **当前服务端是一个 AI API 中转 + 用户/积分管理平台**，由 38 家 AI 厂商适配 + 4 套支付 + 8 种 OAuth 组成；不是按业务领域组织，而是按技术分层（router / controller / service / model）。

---

## 2. 顶层目录结构

```
server/
├── main.go                            入口
├── go.mod / go.sum                    依赖
├── controller/        ~50 文件 / 23K 行   请求处理层
├── service/           ~40 文件 / 13.5K 行 业务逻辑层
├── model/             44 文件 / 11.5K 行  数据模型 + DB 操作
├── relay/             ~100 文件 / 36.7K 行 ★★★ AI 提供商适配
│   ├── channel/openai/
│   ├── channel/claude/
│   ├── channel/gemini/
│   ├── channel/deepseek/
│   ├── channel/dashscope/             阿里通义
│   ├── channel/azure/
│   ├── channel/aws/
│   ├── channel/baidu/
│   ├── channel/tencent/
│   └── ... (38 家)
├── middleware/        24 文件 / 2.6K 行  中间件
├── router/            6 文件 / 808 行    路由定义
├── common/            ~30 文件 / ~5K 行  共享工具
├── setting/           ~20 文件          配置管理
├── constant/          13 文件           常量
├── dto/               29 文件           数据传输对象
├── types/                               类型定义
├── i18n/                                国际化（en / zh）
├── oauth/                               OAuth 提供商
├── pkg/                                 内部包
│   ├── billingexpr/                     ★ 计费表达式
│   ├── cachex/                          缓存扩展
│   └── ionet/                           IO 网络
├── docker-compose.yml                   PostgreSQL + Redis + new-api
└── README.md
```

---

## 3. 三层架构

```
┌────────────────────────────────────────────┐
│  router/  路由定义 + 注册                    │
│      ↓ 转给                                  │
│  controller/  请求处理（参数解析、调用 service）│
│      ↓ 调                                    │
│  service/  业务逻辑（计费、渠道、OAuth）       │
│      ↓ 调                                    │
│  model/  数据 CRUD（GORM）                    │
└────────────────────────────────────────────┘
            ↓ 横切
┌────────────────────────────────────────────┐
│  middleware/  认证 / 限流 / 日志 / 压缩       │
└────────────────────────────────────────────┘
            ↓
┌────────────────────────────────────────────┐
│  relay/  AI 厂商适配（38 家）                  │
└────────────────────────────────────────────┘
```

---

## 4. 核心数据模型（model/）

| 表 | 文件 | 用途 |
|---|---|---|
| `User` | model/user.go | 用户 + 多 OAuth 绑定（含 Steam）|
| `Token` | model/token.go | API Key（含模型限制 / IP 白名单）|
| `Channel` | model/channel.go | AI 渠道（多 Key / 健康检查）|
| `SubscriptionPlan` | model/subscription_plan.go | 订阅计划 |
| `UserSubscription` | model/user_subscription.go | 用户订阅 |
| `TopUp` | model/topup.go | 充值记录（4 套支付）|
| `Redemption` | model/redemption.go | 兑换码 |
| `Option` | model/option.go | 系统配置（key-value）|
| `AuditLog` | model/audit_log.go | 审计日志 |
| `Log` | model/log.go | 调用日志（计费用） |
| `Task` | model/task.go | 异步任务 |
| `MidjourneyTask` | model/midjourney_task.go | MJ 任务（特殊适配） |
| `LoginEvent` | model/login_event.go | 登录历史 |

---

## 5. 用户表（User）字段拆解

```go
// model/user.go
type User struct {
    Id              int
    Username        string
    Password        string
    DisplayName     string
    Role            int       // Root / Admin / User
    Status          int

    // 多 OAuth 绑定
    Email           string
    GitHubId        string
    DiscordId       string
    OidcId          string
    WeChatId        string
    TelegramId      string
    LinuxDOId       string
    SteamId         string    ★ 已实现 Steam OpenID 2.0

    // 系统管理 token
    AccessToken     string

    // 配额
    Quota           int       // 总配额
    UsedQuota       int       // 已用
    RequestCount    int

    // 邀请系统
    Group           string
    AffCode         string
    AffCount        int
    AffQuota        int
    AffHistoryQuota int
    InviterId       int

    // 其他
    Setting         string    // JSON 个人设置
    Remark          string
    StripeCustomer  string

    CreatedAt       int64
    LastLoginAt     int64
}
```

---

## 6. 路由清单（100+ endpoint）

详细完整列表见 [`docs/server/`](../../server/) 中的迁移路径文档。这里给关键分组：

### 6.1 用户与认证
```
POST   /api/user/register
POST   /api/user/login
POST   /api/user/login/2fa
GET    /api/user/logout
GET    /api/user/self
PUT    /api/user/self
GET    /api/user/token
GET    /api/user/checkin
POST   /api/user/checkin
GET    /api/user/login_history
```

### 6.2 OAuth（含 Steam）
```
GET  /api/oauth/state
GET  /api/oauth/wechat
GET  /api/oauth/telegram/login
GET  /api/oauth/steam/login              ★ Steam OpenID 2.0
POST /api/oauth/steam/sdk/login          ★ Steam SDK
GET  /api/oauth/:provider                 GitHub / Discord / OIDC / LinuxDO
```

### 6.3 Passkey / WebAuthn
```
POST   /api/user/passkey/login/begin
POST   /api/user/passkey/login/finish
POST   /api/user/passkey/register/begin
POST   /api/user/passkey/register/finish
POST   /api/user/passkey/verify/begin
POST   /api/user/passkey/verify/finish
DELETE /api/user/passkey
```

### 6.4 充值与支付
```
GET  /api/user/topup/info
GET  /api/user/topup/self
POST /api/user/topup
POST /api/user/stripe/pay
POST /api/user/creem/pay
POST /api/user/waffo/pay
POST /api/user/pay                       Epay
POST /api/stripe/webhook
POST /api/creem/webhook
POST /api/waffo/webhook
```

### 6.5 AI 中转（OpenAI 兼容）
```
GET  /v1/models
POST /v1/chat/completions
POST /v1/completions
POST /v1/embeddings
POST /v1/images/generations
POST /v1/audio/transcriptions
POST /v1/audio/speech
GET  /v1/realtime                        WebSocket
```

### 6.6 系统
```
GET  /api/setup
GET  /api/status
GET  /api/notice
GET  /api/about
GET  /api/pricing
GET  /api/rankings
GET  /api/models
GET  /api/perf-metrics
```

---

## 7. 计费系统（核心机制）

### 7.1 三段式

```
请求到达
   ↓
PreConsume（预消费）
   ├─ 检查钱包/订阅额度
   ├─ 预先扣费
   └─ 保存预消费记录
   ↓
转发到上游 AI
   ↓
获得实际消耗
   ↓
Settle（结算）
   ├─ 计算差额（实际 - 预消费）
   ├─ 补扣或退款
   └─ 记录日志
   ↓
发送额度通知
```

### 7.2 计费来源

- **Wallet**：用户充值的余额
- **Subscription**：订阅计划的月度/年度额度

### 7.3 关键文件

| 文件 | 作用 |
|---|---|
| `service/billing/preconsume.go` | 预扣 |
| `service/billing/settle.go` | 结算 |
| `service/billing/refund.go` | 退款 |
| `pkg/billingexpr/` | 计费表达式（动态定价 / 分层） |

---

## 8. AI 提供商适配（relay/）

### 8.1 已支持厂商

| 厂商 | 用途 |
|---|---|
| OpenAI | GPT-4 / GPT-3.5 / o1 |
| Anthropic | Claude |
| Google | Gemini |
| AWS | Bedrock |
| Azure | OpenAI Azure |
| Mistral | Mistral / Mixtral |
| Cohere | Cohere |
| Perplexity | Perplexity |
| 百度 | Ernie |
| 阿里 | Qwen / DashScope |
| 腾讯 | Hunyuan |
| 讯飞 | Spark |
| 智谱 | GLM |
| 零一万物 | Yi |
| 月之暗面 | Kimi |
| 深度求索 | DeepSeek |
| Ollama | 本地推理 |
| Replicate | 图像生成 |
| Jina | 搜索 |
| Dify | AI 工作流 |
| Coze | 字节跳动 |
| ... | （共 38 家）|

### 8.2 适配器结构

```
relay/channel/{provider}/
├── adapter.go              请求/响应转换
├── stream.go               流式处理
├── auth.go                 认证（如果特殊）
├── model.go                模型映射
└── ...
```

### 8.3 通道选择逻辑

```
service/channel/
├── distribute.go           负载均衡
├── health.go               健康检查
├── failover.go             自动降级
└── multi_key.go            多 Key 轮询
```

---

## 9. 中间件（middleware/）

| 文件 | 作用 |
|---|---|
| `auth.go` | JWT / AccessToken 认证 |
| `rate-limit.go` | 全局 / 关键操作限流 |
| `model-rate-limit.go` | 按模型限流 |
| `cors.go` | CORS |
| `logger.go` | 请求日志 |
| `cache.go` | 响应缓存 |
| `gzip.go` | 压缩 |
| `distributor.go` | 渠道分发 |
| `performance.go` | 性能监控 |
| `stats.go` | 统计 |
| `i18n.go` | 国际化 |
| `admin_audit.go` | 管理员审计 |
| `turnstile-check.go` | Cloudflare Turnstile |
| `email-verification-rate-limit.go` | 邮件限流 |
| `secure_verification.go` | 安全验证 |
| `header_nav.go` | 模块权限 |

---

## 10. 数据库支持

| 数据库 | 版本 | 默认 |
|---|---|---|
| SQLite | — | 开发用 |
| MySQL | ≥ 5.7.8 | — |
| PostgreSQL | ≥ 9.6 | 生产推荐 |

ORM：GORM v2，跨三种数据库兼容。

缓存：
- Redis（分布式）
- 内存（本地，可选）

---

## 11. 配置来源

```
环境变量 (.env)         系统级配置（DSN / 密钥 / OAuth）
   ↓
options 表（DB）        运行时可改的配置（运营调整）
   ↓
setting/ 代码常量       配置默认值
```

关键环境变量：

```bash
SQL_DSN=postgres://...
LOG_SQL_DSN=...
REDIS_CONN_STRING=redis://...
SESSION_SECRET=...
CRYPTO_SECRET=...
STRIPE_API_SECRET=...
STRIPE_WEBHOOK_SECRET=...
GITHUB_CLIENT_ID=...
DISCORD_CLIENT_ID=...
SMTP_SERVER=...
GLOBAL_API_RATE_LIMIT=180
GLOBAL_WEB_RATE_LIMIT=60
```

---

## 12. Docker Compose 部署

```yaml
services:
  postgres: postgres:16-alpine
  redis: redis:7-alpine
  new-api: calciumion/new-api:latest
  security-checker: alpine
```

应用端口 3000，日志 `/app/logs`。

---

## 13. 关键依赖（go.mod）

| 库 | 用途 |
|---|---|
| `gin-gonic/gin` | Web 框架 |
| `gorm.io/gorm` | ORM |
| `go-redis/redis` | Redis |
| `golang-jwt/jwt` | JWT |
| `go-webauthn/webauthn` | Passkey |
| `stripe/stripe-go` | Stripe 支付 |
| `tiktoken-go/tokenizer` | Token 计数 |
| `nicksnyder/go-i18n` | 国际化 |
| `gin-contrib/sessions` | 会话管理 |
| `shopspring/decimal` | 精确小数 |
| `samber/lo` | 函数工具 |
| `bytedance/gopkg` | 字节跳动工具库 |

---

## 14. 现状评估（每模块）

| 模块 | 复杂度 | SaaS 化处置建议 |
|---|---|---|
| **Router** | 低 | 重组：按业务线自注册 |
| **Middleware** | 中 | 部分重写：authz 模块统一 |
| **Controller** | 中 | 拆分：按业务领域进 internal/{module}/api/ |
| **Service** | 高 | 拆分：按业务领域进 internal/{module}/service/ |
| **Model** | 高 | 拆分 + schema 隔离：按业务进 internal/{module}/domain/ |
| **Relay** | 高 | **保留**：跟 new-api 上游 |
| **Setting** | 中 | 重组：每业务线自带 config |
| **Common** | 低 | 部分迁移到 shared/ |
| **OAuth** | 中 | 迁移到 internal/identity/provider/ |
| **billingexpr** | 中 | 保留位置不动 |

---

## 15. SaaS 化演进路径

详细演进见：
- [`09-server-saas-evolution.md`](./09-server-saas-evolution.md)
- [`../../server/06-migration-plan.md`](../../server/06-migration-plan.md)

---

## 16. 入口文件指引

| 想找什么 | 看哪 |
|---|---|
| 程序入口 | `main.go` |
| 路由总注册 | `router/router.go` |
| AI 中转入口 | `controller/relay.go` + `relay/relay-text.go` |
| 用户登录逻辑 | `controller/auth.go` + `service/auth/` |
| Steam OAuth | `oauth/steam.go` + `controller/auth.go` |
| 计费三段式 | `service/billing/` + `pkg/billingexpr/` |
| 渠道选择 | `service/channel/` + `middleware/distributor.go` |
| 数据库初始化 | `model/main.go` |
| 配置加载 | `setting/main.go` |

---

## 17. 调试与诊断

### 17.1 启动

```bash
# 开发模式（SQLite）
cd server && go run main.go

# 生产模式（PostgreSQL）
SQL_DSN="postgres://..." REDIS_CONN_STRING="..." go run main.go
```

### 17.2 日志

- 请求日志：自动写 `logs/`
- 计费日志：写 DB（`Log` 表）
- 审计日志：写 DB（`AuditLog` 表）

### 17.3 性能

- `/api/perf-metrics`：Prometheus 格式指标

---

## 18. 学习路径建议

1. 先读 `main.go` 看启动顺序
2. 读 `router/router.go` 看所有路由
3. 选一个简单 endpoint（如 `/api/status`）跟踪 controller → service → model
4. 看 `service/billing/preconsume.go` 理解计费
5. 看 `relay/channel/openai/adapter.go` 理解一家 AI 适配
6. 看 `oauth/steam.go` 理解 Steam 登录
7. 读 [`09-server-saas-evolution.md`](./09-server-saas-evolution.md) 理解 SaaS 化方向

---

## 19. 与上游 new-api 的关系

- 我们是 fork，不直接 PR 回去
- 定期 rebase 拿新模型适配 / 安全 fix
- 自定义部分：Steam OAuth / 部分 OAuth 提供商扩展
- 完整规划：[`docs/server/`](../../server/)
