# 不可更改的 AI 底层 / Sealed Foundation（05）

> 父文档：[`README.md`](./README.md)
> 关联：[`03-nixos-package-isolation.md`](./03-nixos-package-isolation.md)、[`04-offline-bundle.md`](./04-offline-bundle.md)

---

## 0. 设计哲学

> **AI 修包炸了，平台能兜底；用户手贱删了关键包，系统还能跑。**
>
> 这就是 Sealed Foundation 的存在意义：**一组永远在的、平台签名的、不可被普通操作修改的核心包**。

---

## 1. Sealed Foundation 是什么

### 1.1 一句话

由平台维护并签名的**一组核心 WASM 包**，用户的常规 generation 操作不能改它们，只能通过"平台官方升级"变更。

### 1.2 包含哪些包（建议）

| 包 | 作用 | 必要性 |
|---|---|---|
| `agent-core` | Agent turn loop + 工具调用编排 | 没它不能跑 Agent |
| `tool-runtime-core` | 工具执行引擎 | 没它工具调用全废 |
| `skills` | 内置技能（fs/shell/git/web）| 基础能力 |
| `mcp-client` | MCP 工具集成 | 接外部生态必需 |
| `prompt-system` | Agent prompt 渲染 | 没它 prompt 退化 |
| `channel-core` | 通道 / 事件流 | 数据传输基础 |
| `memory` / `memory-runtime` | 记忆系统 | 上下文管理 |
| `cron` | 定时触发 | 自动化基础 |

合计 ~8 个包，平台**永远维护这套清单**。

### 1.3 不包含哪些

- 产品包（weft-claw / weft-code）— 可被用户卸载
- Feature 包（web-to-pdf 等）— 用户自由装卸
- 用户上传的 community 包

---

## 2. Foundation Manifest 设计

### 2.1 数据结构

```toml
# foundation-manifest.toml （平台维护，随客户端发布）
schema_version = "1.0"

[foundation]
version = "1.0.0"
sealed = true
upgrade_channel = "official-stable"     # stable / beta / canary
issued_at = "2026-05-26T00:00:00Z"
expires_at = "2027-05-26T00:00:00Z"

[[foundation.packages]]
name = "agent-core"
version = "1.5.0"
digest = "sha512-abc123..."
required = true                         # 必装，不可关
upgrade_policy = "always"               # always / on-major / manual

[[foundation.packages]]
name = "tool-runtime-core"
version = "2.1.0"
digest = "sha512-def456..."
required = true
upgrade_policy = "on-major"

# ... 其他 6-7 个包

[upgrade]
check_url = "https://weft.cloud/api/foundation/latest"
check_interval_hours = 24
auto_apply = false                      # 是否自动应用升级（默认问用户）

[signature]
algorithm = "Ed25519"
key_id = "weft-platform-2026"
value = "..."                           # 整个 manifest 内容的签名
```

### 2.2 签名机制

```
平台私钥（保管在 KMS 离线 HSM）
   ↓ sign
foundation-manifest.toml
   ↓ ship with installer
客户端用内置 public_key 验签
```

**关键**：private key 物理离线，签名时人工操作，最大化保护。

---

## 3. 客户端的强制约束

### 3.1 启动校验

```rust
async fn boot_with_sealed_foundation() -> Result<()> {
    // 1. 加载 foundation manifest
    let fm = load_foundation_manifest()?;

    // 2. 验签
    fm.verify_signature(PLATFORM_PUBLIC_KEY)?;

    // 3. 校验过期
    if fm.expires_at < now() {
        warn("Foundation manifest expired, please update");
    }

    // 4. 校验 store 中 foundation 包都存在
    for pkg in &fm.foundation.packages {
        ensure_package_in_store(&pkg.digest)?;
    }

    // 5. 校验当前 active generation 引用了所有 required 包
    let active = load_active_generation()?;
    for pkg in fm.foundation.packages.iter().filter(|p| p.required) {
        if !active.references(&pkg.digest) {
            return Err("Active generation missing required foundation package");
        }
    }

    Ok(())
}
```

### 3.2 Generation 操作约束

```rust
fn validate_proposed_generation(gen: &CandidateGen, fm: &FoundationManifest) -> Result<()> {
    // 1. 所有 required foundation 包必须存在
    for pkg in fm.foundation.packages.iter().filter(|p| p.required) {
        if !gen.has_package(&pkg.name, &pkg.version) {
            return Err(format!("Cannot remove required foundation package: {}", pkg.name));
        }
    }

    // 2. Foundation 包的版本必须严格匹配（不能用其他版本替代）
    for pkg in &fm.foundation.packages {
        if let Some(installed) = gen.find_package(&pkg.name) {
            if installed.version != pkg.version {
                return Err(format!(
                    "Foundation package {} must be version {}, not {}",
                    pkg.name, pkg.version, installed.version
                ));
            }
        }
    }

    Ok(())
}
```

### 3.3 UI 上的体现

| 场景 | 行为 |
|---|---|
| 用户点击 foundation 包的"卸载"按钮 | 按钮置灰，提示"系统核心组件不可卸载" |
| 用户尝试在 generation 中改 foundation 包版本 | 拒绝并提示原因 |
| 用户从 catalog 看 foundation 包 | 标记为"系统核心，由平台维护" |

---

## 4. Foundation 升级流程

### 4.1 升级的两种入口

| 入口 | 触发 |
|---|---|
| 客户端版本升级（installer 升级） | 用户主动下新版本 |
| 在线 Foundation 升级 | 客户端检查到新 manifest 时提示 |

### 4.2 在线升级流程

```
1. 客户端定期请求：GET /api/foundation/latest?channel=stable
   ↓
2. 服务端返回最新 manifest 元数据 + 签名
   ↓
3. 客户端验签 → 比较版本
   ↓
4. 有新版本 → 推送到 AI 询问列表
   "🔄 系统核心可升级到 v1.1.0
    更新内容：修复 X / 优化 Y / 新增 Z
    大小：50 MB"
   [立即升级] [稍后] [跳过此版本]
   ↓
5. 用户同意 → 客户端下载新 manifest 中的所有 foundation 包到 store
   ↓
6. 创建一个新 generation（升级后的 foundation + 原有用户包）
   ↓
7. verify → activate
   ↓
8. 失败 → 自动回滚到上一个 generation
```

### 4.3 升级失败兜底

```
如果新 foundation 升级失败：
1. 当前 active generation 不动（仍是旧 foundation）
2. 告知用户：升级失败原因
3. 提供"重试"或"忽略此版本"选项
4. 服务端记录失败事件用于改进
```

---

## 5. AI 修复对 Sealed Foundation 的依赖

### 5.1 修复时的"安全锚"

当 AI 修复依赖冲突时：

```
用户装的 community 包 X 与某个 feature 冲突
   ↓
AI 修复策略：
  ┌─ 方案 A：升级冲突包 → 试 verify
  ├─ 方案 B：降级冲突包 → 试 verify
  └─ 方案 C：移除包 X → 试 verify

每个方案的关键约束：
  ✓ Sealed Foundation 永远不动
  ✓ AI 不能"为了让 X 跑通而降级 agent-core"
```

这保证了：
- AI 不能误删核心包
- AI 修复永远有 fallback
- 即使所有 feature 包都坏了，foundation 仍能让最小系统跑

### 5.2 极端兜底

如果用户 generation 全坏了：
```
foundation-only mode
└─ 客户端检测到 active generation 不可用
└─ 自动切到 foundation-only generation（仅含 foundation 包）
└─ 系统能跑，但只有最基础能力
└─ 引导用户从 AI 询问列表恢复
```

---

## 6. 平台维护 Foundation 的责任

### 6.1 平台必须做的

- 永远维护 foundation manifest 最新版本
- 关键 bug fix 48 小时内推送
- 每年至少一次 minor 升级（功能演进）
- 私钥安全（HSM + 多签）
- 升级失败的事件追踪和修复

### 6.2 平台禁止做的

- 不能未经用户同意推送 breaking change
- 不能在 foundation 中加入"广告/统计" 类不必要功能
- 不能因为商业原因下架 foundation 包
- 不能给某些用户推送不同版本（除非明确灰度协议）

### 6.3 Foundation 的稳定性承诺

向用户做出的承诺：

> 1. Foundation 包永久免费
> 2. 任何 foundation 升级都向后兼容（用户已有 generation 升级后仍能跑）
> 3. 升级失败可一键回退
> 4. 离线时已装 foundation 永久可用

---

## 7. 与商业模式的关系

Foundation 是免费的，但它撑起了：
- 用户能用 AI（产生消费）
- 用户能装第三方包（推动包市场）
- AI 修复可用（提升体验和留存）

商业上是"基础设施补贴"——平台不直接收 foundation 的钱，但通过它锁定用户、推动消费。

---

## 8. 数据库 schema（服务端）

```sql
-- 服务端维护的 foundation manifests
CREATE TABLE pkg.foundation_manifests (
    id              BIGSERIAL PRIMARY KEY,
    version         TEXT UNIQUE NOT NULL,
    channel         TEXT NOT NULL,           -- stable / beta / canary
    manifest_json   JSONB NOT NULL,
    manifest_bytes  BYTEA NOT NULL,          -- 用于签名的原始字节
    signature       BYTEA NOT NULL,
    issued_at       TIMESTAMPTZ NOT NULL,
    expires_at      TIMESTAMPTZ NOT NULL,
    status          TEXT NOT NULL,           -- draft / published / yanked
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 客户端升级事件追踪
CREATE TABLE pkg.foundation_upgrade_events (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL,
    device_id       BIGINT NOT NULL,
    from_version    TEXT NOT NULL,
    to_version      TEXT NOT NULL,
    result          TEXT NOT NULL,           -- success / failed / rolled_back
    error_message   TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

---

## 9. 验收清单

- [ ] foundation-manifest.toml schema 完整
- [ ] 平台签名 + 客户端验签跑通
- [ ] 客户端启动时 foundation 完整性校验
- [ ] Generation 操作禁止删除 required foundation 包（单测）
- [ ] Generation 操作禁止改 foundation 包版本（单测）
- [ ] 在线升级流程跑通端到端
- [ ] 升级失败自动回滚
- [ ] foundation-only 兜底模式可启动
- [ ] AI 修复尝试不会改 foundation（行为测试）

---

## 10. 与现有 docs 的关系

- 兼容 `../architecture/vNext/runtime/04-declaration-config-lock.md`：Sealed Foundation 是 lock 的一个特殊层级
- 兼容 `../architecture/vNext/runtime/06-profiles-and-policy.md`：Profile 不能解锁 foundation（即使 trusted）
- 兼容 `../architecture/vNext/runtime/19-generation-manager.md`：升级走标准 generation 流程
