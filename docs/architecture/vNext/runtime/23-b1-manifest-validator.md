# B1：Manifest Validator（已落地）

> 状态：validator 核心已落地、编译+单测+运行时验证全通过。
> 关联：`core/src/package/validate.rs`、`core/src/package/config.rs`（`load_manifest`）、`21-b2-*.md`。

## 1. 范围决策：加 validator，不重构 schema

调研发现：**统一解析已存在**——`load_manifest`（config.rs:251）已把多方言隐式收敛成单一 `PackageManifest`：
- 有 `[package_info]` → 直接反序列化
- 无 `[package_info]`（`[identity]+[package]` 方言）→ 用临时 struct 解析后映射；name 空则从 `[identity]` 回退

**缺的不是 schema，是校验**：`load_manifest` 只"尽力解析"，不校验。`installed/memory`（name 解析为空）、`memory-store`（kind=wasm 但 service entry）这类"能解析但不合规"的 manifest 全部静默通过。

→ 因此 B1 落地为**新增 validator**（补缺口、低风险），而非高风险的全量 schema 重构（该项与包生态演进绑定，留待明确需求）。

## 2. 实现

`core/src/package/validate.rs`（纯函数，无 I/O）：
- `validate_manifest(&PackageManifest, entry_exists: Option<bool>) -> Vec<ManifestIssue>`
- `ManifestIssue { severity: Error|Warning, code, message }`，code 机器可读
- 复用 `PackageManifest` 现有方法 `runtime_kind()` / `resolved_entry()` / `resolved_provides()`，不重复逻辑

校验项：
| code | severity | 检测 |
|---|---|---|
| `missing-name` | Error | name 空（如归档前的 installed/memory 孤儿） |
| `missing-version` | Warning | version 空 |
| `missing-entry` | Error | 无法解析 entry |
| `runtime-entry-mismatch` | Warning | runtime=wasm 但 entry 非 .wasm（如 B2 修前的 memory-store），或反之 |
| `entry-not-found` | Error | entry 文件不存在于磁盘 |
| `no-provides` | Warning | provides 空，无法满足任何 binding |

接入：`mod.rs` 的 `discover_package` 在成功解析后调用 validator，Error→`warn!` 日志、Warning→`debug!`（避免刷屏）。**不阻断加载**（保持现有宽容行为），只显式暴露。

## 3. 验证

- `cargo build` exit 0；`cargo test --lib package::validate` → **5 passed**（clean / empty-name / runtime-mismatch / missing-entry-file / product-skip）
- **运行时验证（初版）**：validator 在包发现流程触发，抓到 `packages/weft-code` 的 `entry-not-found`。
  - **后续修正**：`weft-code` 其实是 `[package] kind = "product"` 的**产品包**（能力组合，无可执行 entry），初版 validator 误报。见 §3b。
- **零误报（修正后）**：product 豁免 + weft-code index 条目清理后，全部包 validator 静默通过。

## 3b. Product 包豁免 + weft-code 清理（修正）

初版 validator 对 `packages/weft-code` 报 `entry-not-found`，排查后确认是**误报**：
- `weft-code` 是 `[package] kind = "product"` 产品包，靠 `[requires]`/`[bindings]` 组合能力，**无自己的可执行 entry**；产品发现走 `discover_product_roots`（main.rs:128，独立于 index）。
- 但 `PackageMeta` 当时没解析 `kind` 字段 → validator 走默认 `runtime=wasm` → 误报。

修复：
1. `config.rs`：`PackageMeta` 新增 `kind` 字段 + `PackageManifest::is_product()`。
2. `validate.rs`：product 包跳过 entry/runtime/provides 检查；加 `product_package_skips_entry_and_provides_checks` 测试。
3. `index.toml`：删除冗余的 `weft-code` package_source 条目（`kind=embedded, package_kind=product`，无 provides、无任何引用；产品发现不依赖它）。**回归测试确认**：删除前 `/api/apps` 也只有 `weft-claw`，weft-code 本就不在列，删除无行为变化。备份 `index.toml.bak-cleanup-20260604`。

验证：build/test-compile exit 0、5 单测通过、运行时 0 误报、index 26 条（memory-store=service、weft-code 已删）。

## 4. 后续可扩展（未做）
- `weft package lint <dir>` CLI 子命令（复用 `validate_manifest`，给包作者本地校验）
- 权限-用途一致性校验（需静态分析 wasm 的 host_fn imports，较重）
- index.toml ↔ package.toml 的 kind/name 交叉校验（B2 暴露的那类矛盾）
- 严格模式：Error 级问题拒绝加载（当前仅警告，保持向后兼容）
