# 包市场（Package Marketplace）设计文档

> 状态：Draft
> 作者：Weft Team
> 日期：2026-07-03

---

## 概述

为 Weft 平台设计一个轻量级包市场，让用户可以在应用内发现、安装、管理社区和官方包。设计原则：**渐进式复杂度**——从最简单可行方案（GitHub 托管的 JSON registry）起步，逐步演进到完整的 registry 服务。

---

## 1. 用户体验目标

### 1.1 核心场景

| 场景 | 描述 |
|------|------|
| 浏览发现 | 用户在应用内按分类（工具 / AI 能力 / UI 应用）浏览可用包 |
| 一键安装 | 点击安装按钮，后台下载、验证、热加载，无需重启 |
| 包信息展示 | 名称、描述、版本、作者、大小、运行时类型、权限需求 |
| 已安装管理 | 启用/禁用/卸载/配置已安装包 |
| 更新通知 | 有新版本时在 UI 提示，支持一键更新 |

### 1.2 用户角色

- **最终用户**：浏览、安装、配置包
- **包开发者**：构建、打包、发布包到 registry
- **平台维护者**：审核、签名、管理 registry

---

## 2. 架构设计

### 2.1 整体架构

```
┌─────────────┐       ┌──────────────────┐       ┌─────────────────┐
│ Flutter UI  │──API──│  Weft Core       │──HTTP──│  Registry       │
│ (客户端)     │       │  (Rust Runtime)  │       │  (GitHub/Server)│
└─────────────┘       └──────────────────┘       └─────────────────┘
                              │
                      ┌───────┴────────┐
                      │ packages/      │
                      │   installed/   │
                      │   official/    │
                      │ index.toml     │
                      └────────────────┘
```

### 2.2 Registry（注册表）

**Phase 1 — 静态 JSON Registry（最小可行方案）**

- 一个 `registry.json` 文件托管在 GitHub 仓库（如 `weft-packages/registry`）
- 通过 GitHub Pages 或 raw.githubusercontent.com 提供 HTTPS 访问
- 每次发布新包时通过 PR 更新此文件
- Core 定期拉取并缓存到本地

**Phase 2 — REST API Registry Server**

- 独立的轻量 registry 服务（Rust/Go）
- 支持搜索、分页、分类筛选
- 包开发者通过 CLI 或 CI 发布
- 签名验证强制执行

### 2.3 包分发格式

沿用现有 `package.toml` 格式，打包为 `.weft.tar.gz`：

```
my-package-1.0.0.weft.tar.gz
├── package.toml          # 包元数据 + capability 声明
├── main.wasm             # WASM 二进制（wasm 类型包）
├── assets/               # 静态资源（可选）
├── config.schema.json    # 配置 schema（可选）
└── README.md             # 包说明（可选）
```

构建产物托管方式：
- Phase 1：GitHub Releases（每个包一个 repo 或 monorepo + tag）
- Phase 2：Registry server 自带 artifact storage

### 2.4 安装流程

```
用户点击安装
    │
    ▼
Core 从 registry 获取包详情（download_url, sha512, permissions）
    │
    ▼
下载 .weft.tar.gz artifact
    │
    ▼
校验 SHA-512 完整性
    │
    ▼
检查权限升级（B4 escalation check）
    │  ├── 无升级 → 继续
    │  └── 有升级 → 提示用户确认（approve_escalation）
    │
    ▼
解压到 packages/installed/{name}/
    │
    ▼
注册到 index.toml（current_source 指向新路径）
    │
    ▼
热加载（WasmPackageHost::load_package / ProcessManager）
    │
    ▼
安装完成，UI 刷新状态
```

### 2.5 更新机制

- Core 启动时 + 定期（可配置，默认 6h）拉取 registry index
- 比较本地 installed 版本与 registry 最新版本（semver）
- 有更新时在 UI 标记，用户手动触发更新
- 更新 = 下载新版 → 卸载旧版 → 安装新版（全量替换，简单可靠）
- 未来可优化为增量 diff 下载（低优先级）

### 2.6 依赖解析

- 基于现有 `requires` / `provides` capability 机制
- 安装时检查：包的 `requires` 是否被已安装包的 `provides` 满足
- 若缺少依赖：提示用户先安装依赖包（Phase 1 不自动解析传递依赖）
- Phase 3：拓扑排序 + 自动安装依赖链

---

## 3. Registry 设计

### 3.1 Phase 1：静态 JSON Registry

**仓库结构**（如 `github.com/user/weft-registry`）：

```
weft-registry/
├── registry.json         # 主索引
├── packages/
│   ├── web-search/
│   │   └── metadata.json # 单包详细元数据
│   ├── code-runner/
│   │   └── metadata.json
│   └── ...
└── README.md
```

**发布流程（开发者）**：

1. 开发者在自己仓库构建包，产出 `.weft.tar.gz`
2. 创建 GitHub Release，上传 artifact
3. 向 `weft-registry` 仓库提交 PR，添加/更新 `metadata.json`
4. 维护者审核合并 → `registry.json` 由 CI 自动聚合生成

**CI 聚合脚本**（GitHub Actions）：

```yaml
# 遍历 packages/*/metadata.json，合并生成 registry.json
# 校验格式、必填字段、download_url 可达性
```

### 3.2 Phase 2：REST API Registry

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/packages` | GET | 列表（支持 ?q=&category=&page=&limit=） |
| `/api/v1/packages/{name}` | GET | 包详情 |
| `/api/v1/packages/{name}/versions` | GET | 版本列表 |
| `/api/v1/packages` | POST | 发布新包（需认证） |
| `/api/v1/categories` | GET | 分类列表 |

### 3.3 包提交流程

**Phase 1（PR-based）**：

```
开发者构建包 → 上传到 GitHub Release → 提交 PR 到 registry 仓库
    → CI 校验格式 → 维护者审核 → 合并 → registry.json 更新
```

**Phase 2（CLI-based）**：

```bash
weft publish --registry https://registry.weft.dev
# 自动：打包 → 签名 → 上传 artifact → 注册元数据
```

---

## 4. 数据模型

### 4.1 Registry Index Schema（registry.json）

```json
{
  "version": 1,
  "updated_at": "2026-07-03T12:00:00Z",
  "packages": [
    {
      "name": "web-search",
      "display_name": "Web Search",
      "description": "搜索互联网内容的工具包",
      "category": "tool",
      "author": {
        "name": "Weft Team",
        "url": "https://github.com/weft-team"
      },
      "latest_version": "1.2.0",
      "runtime_type": "wasm",
      "icon_url": "https://...",
      "tags": ["search", "web", "utility"]
    }
  ]
}
```

### 4.2 Package Metadata Schema（metadata.json）

```json
{
  "name": "web-search",
  "display_name": "Web Search",
  "description": "搜索互联网内容的工具包",
  "long_description": "支持多引擎搜索...",
  "category": "tool",
  "author": {
    "name": "Weft Team",
    "email": "team@example.com",
    "url": "https://github.com/weft-team"
  },
  "repository": "https://github.com/weft-team/web-search",
  "license": "MIT",
  "runtime_type": "wasm",
  "icon_url": "https://...",
  "screenshots": [],
  "tags": ["search", "web"],
  "versions": [
    {
      "version": "1.2.0",
      "released_at": "2026-07-01T00:00:00Z",
      "download_url": "https://github.com/.../releases/download/v1.2.0/web-search-1.2.0.weft.tar.gz",
      "sha512": "abc123...",
      "size_bytes": 245760,
      "min_core_version": "0.5.0",
      "permissions": ["network", "filesystem_read"],
      "provides": ["weft_web.search"],
      "requires": [],
      "changelog": "- 新增多引擎支持\n- 修复超时问题"
    }
  ]
}
```

### 4.3 Installed Package State（本地）

沿用现有 `index.toml` 的 `[[package_sources]]` 结构，扩展字段：

```toml
[[package_sources]]
name = "web-search"
kind = "wasm"
package_kind = "tool"
current_source = "packages/installed/web-search"
trusted = true
source_authority = "weft-registry"

# 新增市场相关字段
installed_version = "1.2.0"
installed_at = "2026-07-03T12:00:00Z"
registry_url = "https://raw.githubusercontent.com/user/weft-registry/main/packages/web-search/metadata.json"
auto_update = false
enabled = true
```

### 4.4 Update Manifest（本地缓存）

```toml
# packages/marketplace-cache.toml
[registry]
last_fetched = "2026-07-03T12:00:00Z"
registry_url = "https://raw.githubusercontent.com/user/weft-registry/main/registry.json"
etag = "W/\"abc123\""

[[updates_available]]
name = "web-search"
installed_version = "1.2.0"
latest_version = "1.3.0"
download_url = "https://..."
sha512 = "..."
```

---

## 5. API 设计

扩展现有 `core/src/api/packages_runtime.rs`，新增市场相关端点：

### 5.1 浏览 Registry

```
GET /api/marketplace/packages
    ?q=search_term
    &category=tool|capability|app
    &page=1
    &limit=20

Response: {
  "packages": [...],  // registry index 中的包列表
  "total": 42,
  "page": 1
}
```

```
GET /api/marketplace/packages/{name}

Response: {
  ...metadata.json 完整内容,
  "installed": true,
  "installed_version": "1.2.0",
  "update_available": "1.3.0"
}
```

### 5.2 安装 / 卸载

```
POST /api/packages/install
Body: {
  "name": "web-search",
  "version": "1.2.0",              // 可选，默认 latest
  "store_base_url": "https://...", // 兼容现有字段
  "approve_escalation": false
}
```

```
POST /api/packages/uninstall
Body: { "name": "web-search" }
// 现有接口，已实现反向依赖检查
```

### 5.3 检查更新

```
GET /api/marketplace/updates

Response: {
  "updates": [
    {
      "name": "web-search",
      "installed_version": "1.2.0",
      "latest_version": "1.3.0",
      "changelog": "..."
    }
  ]
}
```

```
POST /api/marketplace/update
Body: { "name": "web-search", "version": "1.3.0" }
// 内部：卸载旧版 → 安装新版 → 热加载
```

### 5.4 已安装列表

```
GET /api/packages
// 现有接口，返回已加载包列表

GET /api/packages/{name}/config/schema
GET /api/packages/{name}/config
PUT /api/packages/{name}/config
// 现有接口，包配置管理
```

### 5.5 Registry 配置

```
GET /api/marketplace/registries
POST /api/marketplace/registries
Body: { "url": "https://...", "name": "community" }
// 支持多 registry 源
```

---

## 6. UI 设计

### 6.1 包浏览页

```
┌─────────────────────────────────────────────────────────┐
│  包市场                                    [搜索框 🔍]   │
├─────────────────────────────────────────────────────────┤
│  [全部] [工具] [AI 能力] [UI 应用]  ← 分类 Tab          │
├─────────────────────────────────────────────────────────┤
│  ┌──────────┐ ┌──────────┐ ┌──────────┐                │
│  │ 🔍       │ │ 🤖       │ │ 📝       │                │
│  │ Web      │ │ Code     │ │ Note     │                │
│  │ Search   │ │ Runner   │ │ Taker    │                │
│  │          │ │          │ │          │                │
│  │ v1.2.0   │ │ v2.0.1   │ │ v1.0.0   │                │
│  │ Weft Team│ │ Weft Team│ │ Community│                │
│  │          │ │          │ │          │                │
│  │[已安装 ✓]│ │[ 安装  ] │ │[ 安装  ] │                │
│  └──────────┘ └──────────┘ └──────────┘                │
│                                                         │
│  ┌──────────┐ ┌──────────┐                              │
│  │ ...      │ │ ...      │                              │
│  └──────────┘ └──────────┘                              │
└─────────────────────────────────────────────────────────┘
```

- 卡片式布局，GridView
- 每张卡片：图标、名称、简短描述、版本、作者、安装状态按钮
- 顶部分类 Tab + 搜索框
- 下拉刷新、分页加载

### 6.2 包详情页

```
┌─────────────────────────────────────────────────────────┐
│  ← 返回                                                 │
├─────────────────────────────────────────────────────────┤
│  🔍 Web Search                          [ 安装 v1.2.0 ] │
│  by Weft Team                                           │
│  ─────────────────────────────────────────────────────  │
│                                                         │
│  搜索互联网内容的工具包，支持多引擎搜索...               │
│                                                         │
│  ┌─ 信息 ─────────────────────────────────────────────┐ │
│  │ 版本: 1.2.0          大小: 240 KB                  │ │
│  │ 类型: WASM           许可: MIT                     │ │
│  │ 运行时: wasm         最低版本: Core 0.5.0          │ │
│  └────────────────────────────────────────────────────┘ │
│                                                         │
│  ┌─ 权限需求 ─────────────────────────────────────────┐ │
│  │ • 网络访问                                          │ │
│  │ • 文件系统读取                                      │ │
│  └────────────────────────────────────────────────────┘ │
│                                                         │
│  ┌─ 更新日志 ─────────────────────────────────────────┐ │
│  │ v1.2.0 - 新增多引擎支持，修复超时问题              │ │
│  │ v1.1.0 - 增加缓存机制                             │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

### 6.3 已安装管理页

```
┌─────────────────────────────────────────────────────────┐
│  已安装包                                  [检查更新]    │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │ 🔍 Web Search  v1.2.0     [启用 ●]  [⚙] [🗑]  │    │
│  │    工具 · Weft Team · 240 KB                    │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │ 🤖 Code Runner v2.0.1    [启用 ●]  [⚙] [🗑]   │    │
│  │    AI 能力 · Weft Team · 1.2 MB   ⬆ 2.1.0可用  │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

- 列表式布局
- 每行：图标、名称、版本、启用开关、配置按钮、卸载按钮
- 有更新时显示升级标记
- 配置按钮打开包配置面板（基于 config.schema.json 动态生成表单）

---

## 7. 实现步骤

### Phase 1：基础市场（MVP）

**目标**：能从 GitHub 发现并安装包

| 步骤 | 内容 | 涉及文件 |
|------|------|----------|
| 1.1 | 定义 registry.json / metadata.json schema | 新建 registry 仓库 |
| 1.2 | Core 实现 registry 拉取 + 本地缓存 | `core/src/marketplace/registry.rs` |
| 1.3 | 实现 `/api/marketplace/*` 端点 | `core/src/api/marketplace.rs` |
| 1.4 | 适配现有 install API 支持 registry 源 | `core/src/api/packages_runtime.rs` |
| 1.5 | 实现 update check 逻辑 | `core/src/marketplace/updater.rs` |
| 1.6 | 创建示例 registry + 2-3 个示例包 | registry 仓库 |
| 1.7 | CLI 命令支持（可选） | `weft marketplace list/install/update` |

**交付物**：
- GitHub 上的 registry 仓库 + CI
- Core 的 marketplace 模块
- 能通过 API 浏览和安装包

### Phase 2：Flutter UI

**目标**：用户可在应用内完成全流程

| 步骤 | 内容 |
|------|------|
| 2.1 | 包浏览页（GridView + 分类 + 搜索） |
| 2.2 | 包详情页（信息 + 权限 + changelog） |
| 2.3 | 安装/卸载交互（进度条 + 权限确认弹窗） |
| 2.4 | 已安装管理页（启用/禁用/配置/卸载） |
| 2.5 | 更新通知 badge + 更新流程 |

**交付物**：
- Flutter 包市场完整 UI
- 与 Core API 全流程打通

### Phase 3：成熟市场

**目标**：生产级包生态

| 步骤 | 内容 |
|------|------|
| 3.1 | 包签名强制验证（ed25519） |
| 3.2 | 自动更新（后台检查 + 用户确认策略） |
| 3.3 | 传递依赖自动解析 + 安装 |
| 3.4 | 发布 CLI（`weft publish`） |
| 3.5 | Registry server（搜索、排序、统计） |
| 3.6 | 开发者 portal（发布管理、统计面板） |
| 3.7 | 包评分/评论系统 |

---

## 8. 与现有系统的集成点

| 现有模块 | 集成方式 |
|----------|----------|
| `index.toml` | marketplace 安装的包写入 `[[package_sources]]`，增加 marketplace 字段 |
| `packages_runtime.rs` install API | 复用现有下载+校验+解压逻辑，marketplace 提供 `store_base_url` |
| `WasmPackageHost` | 安装后调用 `load_package()` 热加载，无需重启 |
| `permissions.rs` | 安装时权限升级检查，UI 展示权限清单 |
| `config.rs` | 包配置管理 UI 基于现有 schema/get/put API |
| `bridge.rs` | capability requires/provides 用于依赖检查 |

---

## 9. 安全考量

- **完整性**：所有下载强制 SHA-512 校验（已有）
- **签名**：Phase 1 可选（字段预留），Phase 3 强制 ed25519 签名验证
- **权限**：安装时展示权限清单，升级需用户确认（已有 B4 机制）
- **沙箱**：WASM 包天然沙箱隔离；service 包通过 ProcessManager 隔离
- **来源**：`source_authority` 标记包来源，可配置信任策略
- **审核**：Phase 1 通过 PR review 人工审核；Phase 2+ 自动化扫描

---

## 10. 开放问题

1. **多 registry 支持**：是否允许用户添加第三方 registry？建议 Phase 2 支持。
2. **离线模式**：本地缓存 registry index，已安装包离线可用。需要离线安装（从文件）能力吗？
3. **包命名冲突**：不同 registry 的同名包如何处理？建议 `{registry}/{name}` 命名空间。
4. **回滚机制**：更新失败时是否保留旧版本？建议保留一个版本的备份。
5. **Core 版本兼容**：`min_core_version` 字段控制，不兼容时 UI 灰显安装按钮。
