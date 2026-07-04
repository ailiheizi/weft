# 首次离线安装包（04）

> 父文档：[`README.md`](./README.md)
> 关联：[`05-sealed-foundation.md`](./05-sealed-foundation.md)、[`03-nixos-package-isolation.md`](./03-nixos-package-isolation.md)

---

## 0. 设计哲学

> **用户下载一个 .exe / .dmg / .deb，双击就能用，不需要联网下任何东西。**
>
> 把 Core + Foundation + 默认产品 全打进去，第一次启动 0 配置。

---

## 1. 离线包内容清单

```
weft-installer-v1.0.0-win64.exe   (约 200-400 MB)
├── core/
│   └── core-runtime.exe           Rust Core Runtime 二进制
├── frontend/
│   ├── Weft.exe                  Electron 主程序
│   ├── resources/                 Vue 3 打包产物
│   └── ...
├── packages/
│   ├── store/                     ← 预填 store
│   │   ├── sha512-<agent-core>/         agent-core@1.5.0
│   │   ├── sha512-<tool-runtime>/       tool-runtime-core@2.1.0
│   │   ├── sha512-<skills>/             skills@1.8.0
│   │   ├── sha512-<mcp-client>/         mcp-client@0.9.0
│   │   ├── sha512-<prompt-system>/      prompt-system@1.0.0
│   │   ├── sha512-<channel-core>/       channel-core@1.0.0
│   │   ├── sha512-<memory>/             memory@1.0.0
│   │   ├── sha512-<cron>/               cron@1.0.0
│   │   ├── sha512-<weft-claw>/         weft-claw@1.0.0
│   │   └── sha512-<weft-code>/         weft-code@1.0.0
│   └── generations/
│       ├── 001/
│       │   ├── lock.toml          预生成的 lock
│       │   ├── manifest.json
│       │   └── packages/          软链接到 store
│       └── active → 001
├── manifests/
│   ├── foundation-manifest.toml   Sealed Foundation 锁定（含平台签名）
│   ├── product-manifest.toml      默认产品包列表
│   └── installer-manifest.toml    安装包自身的版本/签名
├── public-keys/
│   └── weft-platform.pub         平台公钥（用于校验后续升级）
└── installer.exe                  安装器逻辑
```

---

## 2. 安装流程

### 2.1 用户视角

```
1. 下载 weft-installer-v1.0.0-win64.exe（200-400 MB）
2. 双击运行
3. 选择安装路径（默认 C:\Program Files\Weft 或 ~/Applications）
4. 点"安装"
5. 30 秒后完成
6. 启动 Weft
7. 出现欢迎引导：登录 / 注册（可选，离线也能用）
8. 立刻可以开始使用 AI 对话
```

**关键体验**：
- 首次启动**完全不需要联网**
- 不需要选择 / 配置任何包
- 默认产品（weft-claw 对话 / weft-code 代码）开箱可用

### 2.2 安装器逻辑

```rust
fn install(target_dir: &Path) -> Result<()> {
    // 1. 解压 core + frontend 到 target_dir/
    extract_core(target_dir)?;
    extract_frontend(target_dir)?;

    // 2. 创建用户数据目录
    let user_data = get_user_data_dir();
    fs::create_dir_all(&user_data.join("packages/store"))?;
    fs::create_dir_all(&user_data.join("packages/generations"))?;

    // 3. 拷贝预填的 store
    copy_dir(
        &target_dir.join("packages/store"),
        &user_data.join("packages/store"),
    )?;

    // 4. 拷贝预生成的 generation 001
    copy_dir(
        &target_dir.join("packages/generations/001"),
        &user_data.join("packages/generations/001"),
    )?;

    // 5. 设置 active 链接
    symlink("001", &user_data.join("packages/generations/active"))?;

    // 6. 校验 foundation-manifest 签名
    verify_foundation_signature(&target_dir.join("manifests/foundation-manifest.toml"))?;

    // 7. 注册系统级别（注册表 / Application 目录）
    register_with_os(target_dir)?;

    // 8. 创建快捷方式
    create_shortcuts()?;

    Ok(())
}
```

---

## 3. 离线包的构建

### 3.1 由平台 CI 构建

```yaml
# .github/workflows/build-installer.yml
name: Build Installer
on:
  push:
    tags: ['v*']

jobs:
  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build Core Runtime
        run: cargo build --release

      - name: Build Frontend
        run: npm run build --prefix clients/frontend

      - name: Fetch Sealed Foundation Packages
        run: ./scripts/fetch-foundation.sh

      - name: Generate Initial Generation
        run: ./scripts/generate-init-gen.sh

      - name: Sign Foundation Manifest
        run: ./scripts/sign-foundation.sh
        env:
          PLATFORM_PRIVATE_KEY: ${{ secrets.PLATFORM_KEY }}

      - name: Package Installer
        run: ./scripts/package-installer.sh win64

      - uses: actions/upload-artifact@v4
        with:
          name: weft-installer-win64
          path: weft-installer-*-win64.exe
```

### 3.2 三平台分发

| 平台 | 格式 | 大小预估 |
|---|---|---|
| Windows | .exe (NSIS) | 250-400 MB |
| macOS | .dmg + .pkg | 250-400 MB |
| Linux | .deb / .rpm / AppImage | 250-400 MB |

### 3.3 增量更新（远期）

后续小版本升级走差分包：
- 不重新下载整个 200MB
- 客户端用 bsdiff 应用补丁

---

## 4. 首次启动行为

```rust
// 客户端启动入口
async fn first_launch() -> Result<()> {
    // 1. 校验 Core Runtime + Frontend 签名
    verify_install()?;

    // 2. 校验 Foundation Manifest 签名
    let fm = load_foundation_manifest()?;
    fm.verify_signature(&PLATFORM_PUBLIC_KEY)?;

    // 3. 校验 store 中 foundation 包的 hash
    for pkg in &fm.packages {
        verify_package_in_store(&pkg.digest)?;
    }

    // 4. 检查 active generation 完整性
    let active_gen = load_active_generation()?;
    active_gen.verify()?;

    // 5. 启动 Core Runtime（端口 42617）
    let core = CoreRuntime::start().await?;

    // 6. 启动 Frontend（连接 Core）
    let frontend = Frontend::start(&core).await?;

    // 7. 显示欢迎引导（首次启动检测）
    if is_first_launch() {
        show_onboarding_wizard().await?;
        mark_first_launch_done()?;
    }

    Ok(())
}
```

---

## 5. 欢迎引导（Onboarding）

完全离线可用：

```
Step 1: 欢迎页
   "欢迎使用 Weft，AI 时代的个人工作台"
   [继续] [跳过]

Step 2: 登录 / 注册（可选）
   "登录账户可以：
   · 跨设备同步配置
   · 使用云端 AI 算力（消耗积分）
   · 加入 AI 询问列表
   ────
   或保持本地模式：
   · 仅使用本地能力（需自配 API Key）"
   [登录] [注册] [稍后]

Step 3: 选使用模式
   "你想以什么方式使用？
   ⚪ 安全模式（推荐）：AI 只在受限沙箱跑工具
   ⚪ 开发者模式：解锁文件 / shell / git 工具
   ⚪ 完全信任模式：所有能力开放
   注：以后可在设置中修改"
   [继续]

Step 4: 第一次对话
   "试试对 AI 说点什么吧"
   [打开对话]
```

---

## 6. 离线 vs 在线行为对比

| 行为 | 离线 | 在线 |
|---|---|---|
| Foundation 包加载 | ✅ | ✅ |
| 默认产品启动 | ✅ | ✅ |
| AI 对话（自配 API Key）| ✅ | — |
| AI 对话（平台积分）| ❌ | ✅ |
| 知识库（本地索引）| ✅ | ✅ |
| 知识库（云端同步）| ❌ | ✅ |
| 安装新包 | ❌ | ✅ |
| 升级 Foundation | ❌ | ✅ |
| AI 询问列表 | ⚠️ 仅本地建议 | ✅ |
| 多设备同步 | ❌ | ✅ |

---

## 7. 平台签名（Foundation Manifest）

```toml
# foundation-manifest.toml
schema_version = "1.0"

[foundation]
version = "1.0.0"
sealed = true
upgrade_channel = "official-stable"
issued_at = "2026-05-26T00:00:00Z"
expires_at = "2027-05-26T00:00:00Z"   # 一年有效，到期前升级

[[foundation.packages]]
name = "agent-core"
version = "1.5.0"
digest = "sha512-abc123..."

[[foundation.packages]]
name = "tool-runtime-core"
version = "2.1.0"
digest = "sha512-def456..."

# ... 其他包

[signature]
algorithm = "Ed25519"
key_id = "weft-platform-2026"
value = "..."
```

校验流程：
1. 客户端启动时加载 foundation-manifest.toml
2. 用内置的 `weft-platform.pub` 验签
3. 校验每个包的 hash 是否匹配 store
4. 任何一项失败 → 提示用户重新安装 / 修复

---

## 8. 数据持久性

| 数据 | 位置 | 安装包升级时 |
|---|---|---|
| Foundation 包 | `<install>/packages/store` | 覆盖 |
| 用户安装的包 | `<user_data>/packages/store` | 保留 |
| Generation 历史 | `<user_data>/packages/generations` | 保留 |
| 用户配置 | `<user_data>/config/` | 保留 |
| 知识库 | `<user_data>/kb/` | 保留 |
| AI 询问列表 | `<user_data>/suggestions.db` | 保留 |

---

## 9. 升级安装

新版本安装包：
```
weft-installer-v1.1.0-win64.exe
```

检测到已安装时：
1. 备份 `<user_data>/` 重要文件
2. 覆盖 `<install>/` 的 core + frontend + foundation
3. 校验新 foundation-manifest 签名
4. 升级现有 generation 中 foundation 包的引用
5. 用户数据完整保留

---

## 10. 验收清单

- [ ] Windows / macOS / Linux 三平台离线包能构建
- [ ] 安装后首次启动**完全离线**可用
- [ ] Foundation Manifest 签名校验通过
- [ ] 默认 generation 可激活并跑通对话
- [ ] 用户数据目录正确创建
- [ ] 卸载流程不删用户数据
- [ ] 升级安装保留用户数据
- [ ] 离线包大小 < 500MB
- [ ] 安装时间 < 60 秒
