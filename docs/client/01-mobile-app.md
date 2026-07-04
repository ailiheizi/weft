# 手机端简化方案（01）

> 父文档：[`README.md`](./README.md)
> 关联：服务端 `remote_relay` 模块（`../server/01-architecture.md`）

---

## 0. 设计哲学

> **手机端 = 调用端，不是控制端**
>
> 手机不接触包系统、不管理 generation、不修改配置。它只是用户在外面时调用云上服务（或家里桌面）的"瘦终端"。

---

## 1. 能力清单

### 1.1 手机端可做

| 功能 | 实现方式 |
|---|---|
| AI 对话 | 直连云端 LLM Gateway |
| 知识库检索 | 直连云端 KB API |
| Agent 调用（云端） | 直连云端 Agent API |
| Agent 调用（家里桌面）| 走 remote_relay 转发到桌面 Core |
| 查看充值 / 账单 | 直连云端 billing API |
| 查看 AI 询问列表 | 同步桌面的本地队列（只读） |
| 配置同步（查看 / 改偏好） | 通过 device 模块同步 |
| 扫码登录桌面端 | 桌面显示二维码，手机扫码绑定 |
| 临时登录其他电脑 | 类似扫码，会话级授权 |

### 1.2 手机端不可做

| 禁止 | 原因 |
|---|---|
| 安装 / 卸载 / 升级包 | 包是桌面级资源，跨设备管理会乱 |
| 切换 Generation | 同上 |
| 修改 Profile / Policy | 客户端 Core 自治范围 |
| 直接执行 fs / shell / git 工具 | 无沙箱环境 |
| 充值 / 支付（远期可放开）| 移动支付合规复杂，先禁 |
| 后台管理（Admin Panel）| 设备类型决定 |

---

## 2. 架构

### 2.1 两种调用路径

```
┌─────────────────────────────────────────────────────┐
│                  手机 App（React Native / Flutter）  │
└──────────────┬──────────────────────────────────────┘
               │
               ↓
        ┌──────────────┐
        │ WEFT Server  │
        └──────┬────────┘
               │
       ┌───────┴───────┐
       ↓               ↓
┌──────────────┐  ┌────────────────────┐
│ 云端 LLM/KB/  │  │ remote_relay 模块   │
│ Agent 直接服务 │  │  （反向 WebSocket）  │
└──────────────┘  └─────────┬──────────┘
                            │ 长连接
                            ↓
                    ┌────────────────┐
                    │  家里的桌面 Core │
                    └────────────────┘
```

**路径 A（云端调用）**：手机 → Server → 上游 AI → 返回
**路径 B（远程桌面）**：手机 → Server → 桌面 Core 反向隧道 → 桌面执行 → 返回

### 2.2 桌面端反向隧道

桌面 Core Runtime 启动时主动连云：

```rust
// 桌面 Core 启动时
let tunnel = ws_connect("wss://weft.cloud/api/relay/desktop").await?;
tunnel.authenticate(device_id, jwt).await?;

// 等待来自云的请求
while let Some(req) = tunnel.recv().await {
    let resp = handle_local(req).await;
    tunnel.send(resp).await?;
}
```

服务端 remote_relay 模块维护"用户 → 设备 → 隧道"映射。

---

## 3. 扫码登录 / 临时登录

### 3.1 桌面 → 手机扫码登录桌面

桌面 Web UI 显示二维码，手机扫码后：

```
1. 桌面生成 pairing_code（6 位数字 + 二维码）
2. 桌面调用 POST /api/device/pairing-request 注册到云
3. 云返回 pairing_id，桌面在二维码中编码
4. 手机扫码 → POST /api/device/pairing-confirm（带 pairing_id + 手机 device_id）
5. 云校验后建立绑定关系
6. 桌面收到 WebSocket 推送：绑定成功
```

### 3.2 手机扫码临时登录其他电脑

朋友家电脑临时用：

```
1. 朋友电脑显示二维码（仅会话级 token，1 小时有效）
2. 你手机扫码 → 朋友电脑获得你账号的临时会话
3. 退出后自动失效，无持久绑定
4. 朋友电脑无法访问你的"已装功能"，只能用云端服务
```

### 3.3 会话级 vs 持久绑定

| 类型 | 时长 | 权限 |
|---|---|---|
| **持久绑定**（自家手机/电脑） | 永久（可吊销）| 完整设备权限 |
| **会话级**（朋友电脑）| 1 小时 | 仅云端服务，不能访问家里桌面 |

---

## 4. 实现技术选型

### 4.1 客户端框架

| 候选 | 优点 | 缺点 | 推荐 |
|---|---|---|---|
| React Native | 跨端 + 生态好 | 性能 / 包体积一般 | ✅ 启动期 |
| Flutter | 性能好 + UI 一致 | Dart 生态相对小 | 备选 |
| 原生 iOS + Android | 性能最好 | 双倍工作量 | M12+ 考虑 |

**建议**：M4 用 React Native 出 MVP，M12+ 看用户规模决定要不要原生。

### 4.2 通信协议

| 通道 | 协议 | 用途 |
|---|---|---|
| API 调用 | HTTPS + JSON | 普通业务 |
| 流式输出 | SSE | AI 对话流 |
| 远程隧道 | WebSocket | 手机 → 桌面 |
| 推送（远期）| MPS | 离线消息 |

### 4.3 离线策略

| 场景 | 处理 |
|---|---|
| 手机无网 | 提示，离线缓存最近会话 |
| 桌面离线 | 路径 A 仍可用，路径 B 提示桌面不在线 |
| Server 不可用 | 本地缓存 + 重试 |

---

## 5. 不做的功能（明确）

| 功能 | 不做原因 |
|---|---|
| **NAT 穿透 / WebRTC** | 复杂、不稳定；纯中转够用 |
| **推送通知** | iOS APNs + 国内厂商通道维护成本高，远期再做 |
| **离线消息队列** | 推送没做，无意义 |
| **后台静默任务** | iOS 限制多，体验差 |
| **包管理 UI** | 见决策 |
| **Profile 切换** | 仅桌面 Core 自治 |
| **CRDT 实时协同** | 远期 |

---

## 6. 数据库（device 模块用）

```sql
CREATE TABLE device.devices (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL,
    device_type     TEXT NOT NULL,          -- desktop / mobile / web / cli
    name            TEXT,
    fingerprint     TEXT UNIQUE,
    platform        TEXT,                   -- ios / android / windows / mac / linux
    last_seen_at    TIMESTAMPTZ,
    capabilities    JSONB,
    status          TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE device.pairings (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL,
    desktop_id      BIGINT NOT NULL,
    mobile_id       BIGINT NOT NULL,
    pairing_type    TEXT NOT NULL,          -- persistent / session
    expires_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE relay.tunnels (
    id              BIGSERIAL PRIMARY KEY,
    device_id       BIGINT NOT NULL,
    server_node     TEXT NOT NULL,          -- 哪个服务节点持有此隧道
    connected_at    TIMESTAMPTZ NOT NULL,
    last_heartbeat  TIMESTAMPTZ NOT NULL,
    status          TEXT NOT NULL           -- active / disconnected
);
```

---

## 7. 安全考虑

| 风险 | 措施 |
|---|---|
| 隧道劫持 | TLS + JWT + 设备指纹三因素 |
| 中间人 | 强制 HTTPS + cert pinning |
| 临时会话泄露 | 短 TTL + 一次性 token |
| 手机丢失 | 远程吊销设备（其他设备登录后操作）|
| 隧道流量被截 | 端到端加密（远期）|

---

## 8. 验收清单

- [ ] React Native 项目搭起来（iOS + Android 都能编）
- [ ] 登录 / 注册 / 充值流程完成
- [ ] AI 对话（云端路径 A）跑通
- [ ] 知识库检索 UI 完成
- [ ] 扫码绑定桌面流程完成
- [ ] 远程调桌面（路径 B）跑通
- [ ] 临时会话（朋友电脑）流程完成
- [ ] 配置同步 UI 完成
- [ ] 桌面离线时降级体验合理
