# NixOS 风格包隔离（03）

> 父文档：[`README.md`](./README.md)
> 关联：服务端 [`../server/03-package-saas-alignment.md`](../server/03-package-saas-alignment.md)

---

## 0. 设计哲学

> **每次"安装"都是一次新生成；旧的永不修改。**
>
> 这条原则带来：原子回滚、零依赖冲突、可复现、AI 修复可试错（错了秒回）。

---

## 1. 借鉴 NixOS 的什么

| NixOS 概念 | WEFT 对应 |
|---|---|
| `/nix/store/<hash>-name/` | `packages/store/sha512-<hash>/` |
| `/nix/var/nix/profiles/system-N` | `packages/generations/<n>/` |
| `nix-env --rollback` | Generation Manager 切 active 指针 |
| `nix-collect-garbage` | 客户端 GC 模块 |
| Pure functions（包不可变）| WASM artifact 不可变（hash 锁定）|
| Build sandbox | WASM 沙箱本身就有 |

---

## 2. 目录结构

```
<user_data>/weft/
├── packages/
│   ├── store/                        ← 内容寻址存储（不可变）
│   │   ├── sha512-abc123.../         agent-core@1.5.0
│   │   │   ├── package.toml
│   │   │   ├── ai-manifest.toml
│   │   │   ├── manifest.json
│   │   │   └── pkg.wasm
│   │   ├── sha512-def456.../         web-to-pdf@1.2.0
│   │   ├── sha512-789xyz.../         web-to-pdf@1.3.0  (新版本同时存在)
│   │   └── ...
│   ├── generations/                   ← Generation 切换
│   │   ├── 001/                      Gen 1：链接到一组特定 hash
│   │   │   ├── lock.toml             该 generation 的 lock 快照
│   │   │   ├── manifest.json         该 generation 的元数据
│   │   │   └── packages/             软链接到 store 内的 hash 目录
│   │   │       ├── agent-core → ../../store/sha512-abc123.../
│   │   │       └── web-to-pdf → ../../store/sha512-def456.../
│   │   ├── 002/                      Gen 2：换了 web-to-pdf 到 v1.3.0
│   │   └── active → 002              ← 当前激活
│   └── gc/
│       └── gc.log                    回收记录
├── instances/                         产品实例（指向 generation）
│   ├── weft-claw/
│   │   ├── config.toml
│   │   └── active-gen → ../../packages/generations/active
│   └── weft-code/
└── installer-cache/                   下载缓存（短期）
```

---

## 3. 内容寻址存储（store）

### 3.1 命名规则

所有包目录命名 = SHA-512 of package artifact：

```
sha512-abc123def...  (64 字节哈希，前 16 字节作目录名简化)
```

### 3.2 不可变性

- 一旦写入，目录设为只读
- 任何"修改"实际上是创建新 hash 目录
- 即使 v1.0 → v1.0 重发，只要内容变了 hash 就变

### 3.3 完整性校验

每次加载包时校验：
1. 计算实际 SHA-512
2. 对比目录名
3. 不匹配则报错（可能磁盘损坏 / 被篡改）
4. 自动重新从云端拉取

### 3.4 重复利用

- 升级时只下载新 hash，旧 hash 保留（用于回滚）
- 多个 generation 引用同一 hash → 仅占用一份磁盘
- 同一包多版本可以并存（不冲突）

---

## 4. Generation Manager

### 4.1 状态机（来自 docs/architecture/vNext/runtime/19-generation-manager.md）

```
draft → candidate → verified → active
                  ↓
                failed → archived

rollback = activate(previous_verified)
```

### 4.2 Generation 内容

每个 generation 目录：

```
generations/N/
├── lock.toml              ← 完整 lock 快照（含 binding_set_id, closure_id 等）
├── manifest.json          ← 元数据（创建时间、来源、scene、profile）
├── packages/              ← 软链接到 store
│   ├── <pkg-A> → ../../store/sha512-.../
│   └── <pkg-B> → ../../store/sha512-.../
└── meta/
    ├── trust_level        ← 该 generation 的信任级别
    ├── dev_unsealed       ← 是否包含开发模式解锁
    ├── parent_generation  ← 父 generation（用于回滚）
    └── creation_source    ← cli / gui / ai / import
```

### 4.3 操作流程

#### Propose（提议变更）

```
输入：scene + declaration + config + 想要变更的内容
   ↓
1. 计算目标 closure（要装哪些 hash）
2. 检查 store 中缺失的 hash → 下载
3. 创建新 generation 目录（draft 状态）
4. 写入软链接
5. 生成 candidate lock
   ↓
输出：candidate generation id
```

#### Verify（验证）

```
1. 校验所有 hash 完整性
2. 校验 binding 合法性
3. 校验 profile / policy 约束
4. 试加载 WASM（boot check）
5. 通过 → state = verified
   失败 → state = failed → archive
```

#### Activate（激活）

```
原子操作：
1. 写 active 指针 → new gen
2. 更新 root lock.toml 镜像
3. 通知运行中的 instance 重新加载
4. 旧 active gen 保留（用于回滚）
```

#### Rollback（回滚）

```
1. 从 archived/verified 列表中找前一个
2. 直接 activate 该 gen
3. 不重新 resolve，直接用历史 lock
4. 秒级完成
```

---

## 5. AI 修复链路（错了秒回）

### 5.1 流程

```
用户装了 community 包 X → 依赖冲突 → 系统降级
   ↓
AI 修复引擎被触发（客户端本地 + 服务端协助）
   ↓
1. 不动当前 active generation
2. AI 生成新 generation 候选：
   方案 A：升级冲突包到最新
   方案 B：降级冲突包
   方案 C：移除包 X，提示用户
3. 每个方案走完整 propose → verify
4. 只有 verify 通过的方案才推到"AI 询问列表"
5. 用户选择一个方案 → activate 新 generation
6. 失败 → 保持当前 active 不变（即在原始问题状态）
```

### 5.2 服务端协助

- 提供 install-plan API 帮 AI 评估方案
- 提供 known_fixes 库（已知冲突解法）
- 不直接修复（Core 是唯一权威）

详见 [`../server/05-ai-native-distribution.md`](../server/05-ai-native-distribution.md) §6。

---

## 6. 垃圾回收（GC）

### 6.1 何时触发

- 用户手动点"一键清理"
- 后台定期（每周）扫描
- store 占用超过阈值时提醒

### 6.2 GC 算法

```rust
fn collect_garbage() -> GcReport {
    // 1. 列出 store 内所有 hash
    let all = list_store_hashes();

    // 2. 列出活跃 generation（active + 最近 N 个）
    let live_gens = list_recent_generations(KEEP_RECENT);

    // 3. 计算引用集
    let referenced: HashSet = live_gens
        .iter()
        .flat_map(|g| g.referenced_hashes())
        .collect();

    // 4. 未引用的 = 可回收
    let garbage: Vec<_> = all.into_iter()
        .filter(|h| !referenced.contains(h))
        .collect();

    // 5. 用户确认 → 删除
    GcReport { garbage, freed_bytes: ... }
}
```

### 6.3 保留策略

```toml
# user config
[gc]
keep_recent_generations = 10        # 至少保留最近 10 个 generation
keep_failed_repairs_days = 7        # AI 修复失败的 generation 保留 7 天
auto_gc = true                      # 自动 GC 开关
auto_gc_when_disk_above = "5GB"     # 触发阈值
```

### 6.4 GC 日志

```
gc/gc.log
2026-05-26T10:00:00 manual_trigger removed=12 freed=850MB kept_generations=10
2026-05-27T03:00:00 auto_disk_threshold removed=3 freed=120MB
```

---

## 7. 一键清理 UI

```
┌──────────────────────────────────────┐
│  🧹 存储管理                          │
├──────────────────────────────────────┤
│  已用空间：2.3 GB                     │
│  ├─ 当前激活的功能：1.5 GB             │
│  ├─ 历史版本（用于回滚）：500 MB        │
│  └─ 可清理：300 MB                    │
│                                      │
│  详细：                               │
│  · 旧版本"网页存 PDF" v1.2 → 60 MB    │
│  · 旧版本"AI 工具集" v2.0 → 200 MB    │
│  · AI 修复尝试残留（3 个）→ 30 MB     │
│  · 过期缓存 → 10 MB                   │
│                                      │
│  设置：                               │
│  ☑ 保留最近 10 个版本                 │
│  ☑ 磁盘 > 5GB 时自动清理              │
│                                      │
│  [一键清理]  [详细查看]               │
└──────────────────────────────────────┘
```

---

## 8. 客户端调用示例

```rust
// 在 Rust Core Runtime 内
let store = Store::open(&user_data.join("packages/store"))?;
let gens = GenerationManager::open(&user_data.join("packages/generations"))?;

// 安装新包
let candidate = gens.propose(ProposeInput {
    scene: "default",
    add: vec![("web-to-pdf", "1.2.0")],
    profile: Profile::Safe,
}).await?;

// 验证
let verify = gens.verify(&candidate).await?;
if !verify.passed {
    return Err(verify.errors.into());
}

// 激活
gens.activate(&candidate).await?;

// 回滚
gens.rollback_to_previous().await?;

// GC
let report = gens.collect_garbage(GcOptions {
    keep_recent: 10,
    dry_run: false,
}).await?;
println!("Freed {} MB", report.freed_bytes / 1024 / 1024);
```

---

## 9. 与首次离线包的关系

首次安装时（详见 [`04-offline-bundle.md`](./04-offline-bundle.md)）：
- store/ 预填 sealed foundation 的所有 hash
- generations/001/ 是预生成的初始 generation（含默认产品）
- active → 001
- 用户首次启动直接可用，无需联网

---

## 10. 验收清单

- [ ] store/ 内容寻址存储跑通（写入 / 读取 / 校验）
- [ ] 同一包多版本并存
- [ ] generations/ 软链接结构正确
- [ ] propose / verify / activate / rollback 四个操作完整
- [ ] active 指针原子切换
- [ ] GC 算法正确（不删活跃引用）
- [ ] 一键清理 UI 完成
- [ ] 自动 GC 配置可生效
- [ ] AI 修复时不动当前 active
- [ ] 性能：generation 切换 < 1 秒
