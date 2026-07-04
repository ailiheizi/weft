# 客户端规划（Client）

> 路径：`docs/client/`
> 用途：桌面端 SaaS 化演进 + 手机端 + AI 询问列表 + NixOS 包隔离 + 首次离线包
> 编制日期：2026-05-26

---

## 0. 客户端边界

| 组件 | 跑在哪 | 本文档覆盖 |
|---|---|---|
| Core Runtime（Rust） | 用户桌面 | ✅ 包隔离 / GC / Foundation |
| Electron 前端（Vue 3） | 用户桌面 | ✅ AI 询问列表 / 配置同步 UI |
| 手机 App | 用户手机 | ✅ 简化版远程调用 |
| WASM 包 | 用户桌面 | ✅ NixOS 风格 store |

**不覆盖**：服务端（见 `../server/`）。

---

## 1. 文档清单

| 文件 | 内容 |
|---|---|
| **`README.md`** | 本文件 |
| `01-mobile-app.md` | 手机端简化方案 |
| `02-ai-suggestion-list.md` | AI 询问列表 UI |
| `03-nixos-package-isolation.md` | NixOS 风格包隔离 |
| `04-offline-bundle.md` | 首次离线安装包 |
| `05-sealed-foundation.md` | 不可更改的 AI 底层 |

---

## 2. 关键决策汇总

| 议题 | 决策 |
|---|---|
| 手机端能管理包吗 | ❌ 不能，只能用桌面已装能力 |
| 手机端远程方案 | 全走云中转（WebSocket 隧道），不做 NAT 穿透 |
| 手机端推送通知 | 暂不做 |
| AI 询问列表在哪 | 纯客户端（本地 SQLite/sled），服务端不存队列 |
| 包隔离方案 | NixOS 完整版（hash store + generation + GC） |
| 首次安装 | 离线包（Core + Foundation + 默认产品），0 配置 |
| Sealed Foundation | 锁定具体包列表，仅平台官方升级可变更 |
| 用户看到"包"吗 | ❌ UI 上只展示"功能"，隐藏所有包概念 |
| AI 修复策略 | 生成新 generation 试错，不动当前 active |

---

## 3. 架构图（SaaS 化后）

```
┌─────────────────────────────────────────────────────────────┐
│                    桌面客户端（Electron）                      │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  Vue 3 UI                                           │    │
│  │  ├─ AI 询问列表（建议队列 UI）                       │    │
│  │  ├─ 功能管理（隐藏包概念）                           │    │
│  │  ├─ 知识库 / Agent / 对话                           │    │
│  │  └─ 配置同步状态                                    │    │
│  └─────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  Core Runtime（Rust）                                │    │
│  │  ├─ Generation Manager（propose/verify/activate）    │    │
│  │  ├─ NixOS Store（hash 寻址 + 不可变）               │    │
│  │  ├─ Sealed Foundation（不可卸载的 AI 底层）          │    │
│  │  ├─ AI 修复引擎（依赖冲突自动修复）                  │    │
│  │  ├─ 建议队列（本地 SQLite）                         │    │
│  │  └─ 远程隧道客户端（连接 Server）                    │    │
│  └─────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  packages/store/（NixOS 风格）                       │    │
│  │  ├─ sha512-abc123.../  (agent-core@1.5.0)          │    │
│  │  ├─ sha512-def456.../  (web-to-pdf@1.2.0)         │    │
│  │  └─ ...                                            │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
         │                              ↑
         │ WebSocket 长连接              │ 手机请求转发
         ↓                              │
┌─────────────────────────┐    ┌────────┴────────┐
│   WEFT Cloud Server    │    │   手机 App       │
│   (remote_relay 模块)    │←───│  (纯调用端)      │
└─────────────────────────┘    └─────────────────┘
```

---

## 4. 手机端概要

### 能做
- AI 对话（走云 LLM 中转）
- 知识库检索（走云 KB API）
- Agent 调用（走云 Agent API）
- 扫码登录桌面端 / 临时登录其他电脑
- 查看账单 / 充值
- 查看 AI 询问列表（同步自桌面）
- 配置同步（查看 / 修改偏好）

### 不能做
- ❌ 管理包（安装 / 卸载 / 升级）
- ❌ 管理 Generation
- ❌ 修改 Profile / Policy
- ❌ 直接执行本地工具（fs / shell / git）

### 远程调桌面
- 手机 → Server（WebSocket）→ 桌面 Core（反向隧道）
- 只转发 channel 消息，不做复杂协议
- 桌面离线时：提示"桌面不在线"，仅可用云端服务

详见 `01-mobile-app.md`。

---

## 5. AI 询问列表概要

- 纯客户端本地队列（SQLite / sled）
- 服务端推荐 API 返回建议 → 客户端写入队列
- UI 呈现为多选框列表
- 用户可全选 / 部分选 / 拒绝 / 稍后
- 接受的项 → Core 走 Generation 流程
- 7 天 TTL 自动过期
- 成组建议（Bundle）支持

详见 `02-ai-suggestion-list.md`。

---

## 6. NixOS 包隔离概要

```
packages/
├── store/                    内容寻址存储（SHA-512 命名）
│   ├── sha512-abc123.../     不可变，永不修改
│   └── sha512-def456.../
├── generations/              Generation 切换
│   ├── 001/                  链接到一组特定 hash
│   ├── 002/
│   └── active → 002          当前激活
└── gc/                       垃圾回收
    └── gc.log                回收记录
```

- 安装 = 下载 hash 文件到 store + 创建新 generation
- 升级 = 下载新 hash + 新 generation（旧的不删）
- 回滚 = 切换 active 指针（秒级）
- GC = 清理无 generation 引用的旧 hash

详见 `03-nixos-package-isolation.md`。

---

## 7. 首次离线包概要

用户首次安装 WEFT 时下载一个"全离线包"：

```
weft-installer-v1.0.0-win64.exe
├── core-runtime.exe          Core Runtime 二进制
├── foundation/               Sealed Foundation 包
│   ├── agent-core@1.5.0.wasm
│   ├── tool-runtime-core@2.1.0.wasm
│   ├── skills@1.8.0.wasm
│   ├── mcp-client@0.9.0.wasm
│   └── prompt-system@1.0.0.wasm
├── product/                  默认产品包
│   ├── weft-claw@1.0.0.wasm
│   └── weft-code@1.0.0.wasm
├── foundation-manifest.toml  签名 + 版本锁定
└── default-lock.toml         预生成的 lock（首次启动直接用）
```

- 安装后 0 配置即可运行
- 首次启动时 Core 校验 foundation-manifest 签名
- 后续按需从服务端拉新包

详见 `04-offline-bundle.md`。

---

## 8. Sealed Foundation 概要

```toml
# foundation-manifest.toml
[foundation]
sealed = true
upgrade_channel = "official-stable"
signature = "..."

[[foundation.packages]]
name = "agent-core"
version = "1.5.0"
digest = "sha512-..."

[[foundation.packages]]
name = "tool-runtime-core"
version = "2.1.0"
digest = "sha512-..."

# ... 其他 foundation 包
```

规则：
- 用户不能通过普通 generation 操作改这些包
- 只能通过"客户端版本升级"或"平台推送的 foundation 升级"更新
- AI 修复时永远能依赖这些包做 fallback

详见 `05-sealed-foundation.md`。

---

## 9. 配置同步

桌面 ↔ 服务端 ↔ 手机 三端同步的配置项：

| 配置 | 同步方向 | 冲突策略 |
|---|---|---|
| 用户偏好（主题/语言/快捷键） | 双向 | 最后修改时间胜出 |
| API Key（BYOK） | 桌面→云→手机 | 桌面为主 |
| 模型偏好（默认档位） | 双向 | 最后修改 |
| AI 询问列表 | 桌面→云→手机（只读） | 桌面为主 |
| Evolved Skill | 桌面→云 | 桌面为主 |
| Lock 备份 | 桌面→云 | 桌面为主 |
| 会话历史 | 双向 | 合并（CRDT） |

---

## 10. 一键清理（GC）

用户在 UI 上看到：

```
┌──────────────────────────────────────┐
│  🧹 存储管理                          │
├──────────────────────────────────────┤
│  已用空间：2.3 GB                     │
│  可清理：800 MB                       │
│                                      │
│  · 旧版本功能（5 个）    600 MB       │
│  · AI 修复尝试残留       150 MB       │
│  · 过期缓存              50 MB        │
│                                      │
│  [一键清理]  [查看详情]               │
└──────────────────────────────────────┘
```

清理逻辑：
1. 列出所有 store 中的 hash
2. 扫描所有 generation（active + 最近 N 个保留）引用的 hash
3. 未被引用的 = 可清理
4. AI 修复尝试失败的 generation 也标记可清理
5. 用户确认后删除

---

## 11. 优先级

| 功能 | 优先级 | 时间 |
|---|---|---|
| NixOS store + generation 切换 | P0 | M1-M2 |
| Sealed Foundation | P0 | M1 |
| 首次离线包 | P0 | M2 |
| AI 询问列表 UI | P0 | M2-M3 |
| 配置同步 | P1 | M3-M4 |
| 手机端 App MVP | P1 | M4 |
| 一键清理 GC | P1 | M4 |
| AI 修复引擎 | P2 | M5-M6 |
| 会话历史 CRDT 同步 | P2 | M6 |

---

## 12. 与其他文档的关系

- 服务端配套见 [`../server/`](../server/)
- 包系统 ADR 见 `../architecture/vNext/adr/`
- Generation Manager 见 `../architecture/vNext/runtime/19-generation-manager.md`
- Lock 三层模型见 `../architecture/vNext/runtime/04-declaration-config-lock.md`
- SaaS 功能全景见 [`../saas-features/`](../saas-features/)
