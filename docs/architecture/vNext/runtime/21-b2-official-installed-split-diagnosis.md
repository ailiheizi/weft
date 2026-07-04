# B2：official/installed 包分裂 — 诊断与修复方案

> ⚠️ **勘误（2026-06-04，经运行时验证修正）**
> 本文 §2/§4 中将 channels/cron/mcp-client/prompt-system/workflow-orchestrator/team-task-board/team-runtime/workflow-template-devteam 共 **8 个包判定为「STUB、无 .wasm 产物、未加载」是错误的**。
> 原因：原诊断只检查包根目录有无 `package.wasm`，**漏看了 `resolve_entry_path`（mod.rs:471）的 fallback 机制** —— 它会去 `target/wasm32-wasip1/{debug,release}/` 找编译产物，找不到且有 Cargo.toml 还会现场 `cargo build`。
> **实测（启动日志）：这 8 个包全部有 target/ 产物且被成功加载**（`Loaded WASM package 'channel-core' from .../target/wasm32-wasip1/debug/...`）。mcp-client/prompt-system 也**已装载**，wiki 的 P0「未装载」记录同样需修正。
> **唯一的真 stub 是 `packages/weft-code`**（无 Cargo、无 wasm、无法 fallback 编译），且 index.toml L299 仍引用它 —— 由 B1 validator 抓到（`entry-not-found`）。
> 下文 §2/§4 的 STUB 标注与「8 个 stub」表述保留作诊断演进记录，但以本勘误为准。

> 状态：诊断完成（铁证锁定）；index.toml 已做第 1 项修复；lock 重生成与孤儿归档待执行。
> 关联：[`04-declaration-config-lock.md`](./04-declaration-config-lock.md)、wiki `package-official-vs-installed`、`docs/wiki/Developers/06-design-vs-implementation-gaps.md`
> 决策基线：用户已定「只做 index 数据对齐，不编译 stub」。

---

## 1. 根因：三层 desync（index / lock / 文件系统 互不自洽）

| 层 | 作用 | 现状问题 |
|---|---|---|
| `packages/index.toml` | 包注册表，运行时 `discover_runtime_packages` 用 | tool-* 的 `current_source` 指 `installed/`、其余指 `official/`，**规则不统一**；`memory-store` 的 `kind` 标错 |
| `packages/weft-claw/lock.toml` | 激活快照，驱动 generation digest 校验 | **几乎全部 `source` 指 `official/`**，且记录的 wasm digest 多为**幻影**（official 目录下无 .wasm） |
| 文件系统 | 真实 `.wasm` 产物 | 大多在 `installed/`；8 个 official 包只有 Rust 源码无 .wasm |

**结论**：「分裂」不是单一 index 数据问题，而是注册表、激活快照、产物三者互不对齐。其中 `lock.toml` 是 **resolver 产物，不可手改**，必须靠 `weft generate` 重新解析生成。

---

## 2. index.toml 逐条体检表（实测）

发现逻辑（`core/src/package/mod.rs`）：
- `discover_runtime_packages`(L364)：收集 `installed/` + `official/` + 每条 index 的 `current_source`，再 `select_runtime_packages` 去重
- `compare_runtime_plugin_package_precedence`(L288)：优先选 `is_declared_current_runtime_source`（dir 精确等于 index `current_source` 的那个）
- `discover_package`(L556)：**wasm 包 entry(.wasm) 不存在 → `warn` 跳过返回 None**（L576-586），不崩溃但能力无 provider

| 包 | kind | current_source | 有 .wasm | 判定 |
|---|---|---|---|---|
| agent-runtime | wasm | official/agent-core | ✓ | 正常 |
| skills-runtime | wasm | official/skills | ✓ | 正常 |
| session-events | wasm | official/session-events | ✓ | 正常 |
| weft-claw | wasm | official/weft-claw | ✓ | 正常 |
| tool-runtime-core | wasm | **installed**/tool-runtime-core | ✓ | 正常（official 同名是空壳源码） |
| tool-shell | wasm | **installed**/tool-shell | ✓ | 正常 |
| tool-files | wasm | **installed**/tool-files | ✓ | 正常 |
| tool-web | wasm | **installed**/tool-web | ✓ | 正常（official 也有 wasm） |
| tool-git | wasm | **installed**/tool-git | ✓ | 正常 |
| weft-code-runtime | wasm | **installed**/weft-code-runtime | ✓ | 正常 |
| **memory-store** | ~~wasm~~ → **service** | official/memory | ✗(service) | **已修**：是 Python service（entry=`start-memory-runtime.ps1`），原 index 误标 kind=wasm |
| channel-core | wasm | official/channels | ✗ | **STUB**：有源码+Cargo，缺编译产物 |
| prompt-system | wasm | official/prompt-system | ✗ | **STUB**（wiki P0） |
| workflow-orchestrator | wasm | official/workflow-orchestrator | ✗ | **STUB** |
| team-task-board | wasm | official/team-task-board | ✗ | **STUB** |
| workflow-template-devteam | wasm | official/workflow-template-devteam | ✗ | **STUB** |
| team-runtime | wasm | official/team-runtime | ✗ | **STUB** |
| cron | wasm | official/cron | ✗ | **STUB** |
| mcp-client | wasm | official/mcp-client | ✗ | **STUB**（wiki P0） |
| companion-core / memory-runtime / context-engine / generic-agent-runtime / js-extension-runtime | service | official/* | n/a | 正常（service 不需 .wasm） |
| ai-workspace-browser / weft-claw-ui / weft-code | embedded | official 或 installed | n/a | 正常 |

---

## 3. memory 三方矛盾（最深，需单独决策）

铁证：
- `index.toml`：`memory-store` 原 `kind="wasm"`（**错**，已修为 service），`current_source=official/memory`
- `official/memory/package.toml`：`runtime="service"`，entry=`start-memory-runtime.ps1`，有 `server.py`，provides `memory.store`
- `weft-claw/lock.toml` 的 `memory-store` 条目：`source=official/memory` 但 `runtime="wasm"` / `entry_kind="wasm"` / `sha512=e901dff1…`
- **该 `e901dff1` wasm 在整个仓库不存在**（official/memory 无 .wasm）
- `installed/memory/package.wasm` 实际 sha512 = `e9eeecbf…`（**与 lock 记录不符**），name=`memory`（≠ memory-store），无源码 → **孤儿**，不被任何 index 条目引用

**解读**：index 的 `kind=wasm` 错误向下游污染了 lock（lock 把 service 包记成 wasm + 幻影 digest）。memory.store 的真实实现是 **service**（official/memory）。`installed/memory` 是历史遗留的 wasm 孤儿。

**修复**：
1. ✅ index `memory-store` kind → service（已做）
2. ⏳ 重生成 lock 后，`memory-store` 条目应变为 `runtime=service`/`entry_kind=service`，幻影 wasm digest 消除
3. ⏳ 归档孤儿 `packages/installed/memory/`（移到备份目录，不直接删，便于回溯）

---

## 4. 修复清单与执行顺序

### 已完成
- [x] **index.toml：`memory-store` kind `wasm`→`service`**（修正 index↔manifest 矛盾。备份：`packages/index.toml.bak-b2-20260603`）

<!-- B2-EXEC-RESULT-2026-06-04 -->
## 8. 执行结果（2026-06-04）

> lock 重生成实际执行，覆盖原 §4「待执行」项。

### 前置：weft CLI 两处修复
1. **propose 缺 Content-Type**（`weft.rs` `post_json`）：空 body POST 补 `.json(&json!({}))`，propose/verify/activate/rollback 一并修复。
2. **CLI 不支持 D2 token**（鉴权开启后 `weft gen` 全线 401 回归）：新增 `build_client()`，按 `WEFT_TOKEN` env → `data/runtime-token` → `runtime-token` 读 token 注入 Bearer 头。

### lock 重生成（generate→verify→activate 全 exit 0）
- 新 generation `binding-set:sha256:97f75aab…`（原 `b4078325…`），可 rollback。
- **memory 核心矛盾修复**（实测 lock.toml）：`memory-store.runtime` wasm→**service**、`entry_kind` wasm→**service**、幻影 wasm digest `e901dff1…`（仓库不存在）**已消除**。
- **孤儿归档**：`packages/installed/memory` → `packages/.archive-b2-20260604/installed-memory-orphan`（移动非删除，可逆）。

### 意外发现：tool-* 从 lock 剔除（⚠️需运行时验证）
重生成后 lock 仅 13 包，**无任何 tool-\* 条目**（重生成前有 tool-shell/files/git/web/runtime-core 共 5 条，均指 official 幻影 digest）。
- 原因：lock 由 resolver 按 weft-claw declaration 能力依赖链解析；weft-claw 未把独立 tool-* 声明为直接依赖（工具能力由 agent-runtime/tool-runtime-core 内部聚合）。
- 判定：这些 tool-* 条目本是陈旧错误残留，重生成正确剔除——比「修正 source」更彻底。
- ⚠️ **待验证**：若运行时确需独立加载 tool-shell/files，移除可能影响工具能力。需「执行带工具调用的 agent 任务」实测。当前 binding 解析显示非直接依赖，判定为 resolver 正确行为，但未端到端验证。

### 备份
`index.toml.bak-b2-20260603`、`weft-claw/lock.toml.bak-b2-20260604`、`.archive-b2-20260604/installed-memory-orphan/`

### 待执行（需环境 / 决策）
- [ ] **重生成 lock**（正道，可 rollback）：
  ```
  # 需先编译并运行 weft-core，或使用已运行的 127.0.0.1:3004
  weft generate --app weft-claw       # POST /api/apps/weft-claw/generations
  weft verify   --app weft-claw       # 校验 digest 一致
  weft activate --app weft-claw       # 切到新 generation（失败可 weft rollback）
  ```
  预期效果：lock 的 `memory-store` 变 service、tool-* 的 `source` 从 official 幻影对齐到 installed 真产物、幻影 digest 消除。
- [ ] **归档孤儿** `packages/installed/memory/`（哈希 `e9eeecbf`，无源码、无引用、与 lock 不符）→ 移至备份目录。
- [ ] **8 个 STUB 标注**（本波不编译，按用户决策）：建议在 index.toml 每条 STUB 加注释 `# STUB: 源码在 official/<x>，缺 package.wasm，待编译`，或加 `build_status = "source-only"` 字段（需 core 端 PackageSource 解析支持，属 B1 范畴）。

### 不在本波（留作独立任务）
- 编译 8 个 stub（cargo build wasm32）恢复 mcp-client/prompt-system 等 P0 能力 — 用户已定本波不做。
- official/tool-* 空壳源码目录：保留（将来编译源），不清理。

---

## 5. 风险评估

| 改动 | 影响 | 可逆性 |
|---|---|---|
| index memory-store kind→service | discover 时把 memory 当 service 处理（与 manifest 一致），不再期待 .wasm | ✓ 有 .bak |
| 重生成 lock | 改变激活快照；旧 generation 仍在 generation-store，可 rollback | ✓ generation rollback |
| 归档 installed/memory | 无引用、无源码，归档无运行时影响 | ✓ 移动非删除 |
| STUB 标注 | 纯注释/元数据 | ✓ |

**packages/ 不受 git 跟踪**（顶层无 .git，仅 core/ 与 clients/ 有）→ 所有改动需手动备份，已对 index.toml 建 .bak。

---

## 6. 与 server v2 的关系

待核实：`docs/server/03-package-saas-alignment.md`、`05-ai-native-distribution.md` 描述的 registry/分发是否覆盖「编译并分发 official 包产物」。若是，则 8 个 stub 的编译产物供给应归入 server 侧 Package Server 流程，本地 index 仅作消费端 — 避免重复实现。**结论：stub 编译归属 server v2 判断，本波不碰。**

---

## 7. 后续与 A1 的衔接

A1（host_fn 权限门控）依赖本诊断：开启「默认拒绝」前需对**实际运行集**补 `[permissions]` 声明。实际运行的 WASM 包（有 .wasm 且被加载）为：
agent-runtime, skills-runtime, session-events, weft-claw, tool-runtime-core, tool-shell, tool-files, tool-web, tool-git, weft-code-runtime, installed/memory(若保留)。
service 包（companion-core/memory-runtime/context-engine/generic-agent-runtime/js-extension-runtime）走 HTTP 不经 host_fn，不在 A1 gate 范围。
STUB 包未加载，暂不需声明。
→ A1 补声明只需覆盖上述 ~10 个 wasm 运行包，范围大幅收敛。
