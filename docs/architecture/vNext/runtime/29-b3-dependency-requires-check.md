# B3：依赖求解 — capability requires 完整性校验（已落地）

> 状态：requires 完整性校验已落地 + 编译 + 单测(3) + 运行时验证通过。semver/拓扑排序经评估**不做**（见 §1）。
> 关联：`core/src/app/packages.rs`（PackageIndex::unmet_requirements）、`main.rs`（启动校验）。

## 1. 范围决策：做 requires 校验，不做 semver/拓扑

B3 原描述 = 版本/依赖求解器（semver + 拓扑排序 + 冲突消解）。调研后发现这大部分**无真实数据驱动、不符合 WEFT 架构**：

- **无包声明 `[dependencies]` 版本约束**（实测全部包为空）。`manifest.dependencies` 字段被解析、被 `find_missing_dependencies` 做名字存在性检查，但 `required_version` 只存不比对 → semver 求解器**没有数据可解**（B5 式空转风险）。
- **WEFT 依赖是 capability 级，不是包版本级**：包声明 `requires = ["session.events"]`（能力），不是 `dependencies = {foo="^1.0"}`（版本）。能力依赖由 capability registry + binding resolution 处理。
- **加载顺序不是硬约束**：WASM 包加载后经 `package_map` 共享，跨包调用走运行时 `host_call_package` 查表，不要求被依赖者先 init（不像 native 链接）→ 拓扑排序加载收益low。

**真正有数据、有价值的子集**：capability requires 的完整性校验。index.toml 里 `agent-runtime requires session.events`、`weft-claw requires [agent.runtime, ext.skills, ext.mcp, memory.store, tool.runtime, session.events]` 是真实声明——若某 requires 能力无 provider（如 mcp-client 未装但 weft-claw 要 ext.mcp），运行时会静默失败。这是该补的检查。

## 2. 实现

`PackageIndex::unmet_requirements() -> Vec<UnmetRequirement>`（packages.rs）：
- 汇总所有 source 的 `provides` 成集合。
- 遍历每个 source 的 `requires`，若某能力不在 provides 集合 → 记一条 `UnmetRequirement { package, capability }`。
- 空 = 每个声明的能力依赖都有 provider（不保证该 provider 可加载——那是 validate/runtime 的事）。

`main.rs`：包发现后调用 `unmet_requirements()`，对每条缺口 `tracing::warn!`（**警告不阻断**——保持宽容启动，只暴露）。

## 3. 验证

- `cargo build` exit 0。
- `app::packages::tests`：3 passed —
  - `unmet_requirements_empty_when_all_satisfied`
  - `unmet_requirements_flags_missing_provider`（requires ext.mcp 无 provider → 精确标记）
  - `unmet_requirements_repo_index_is_satisfied`（**校验真实 packages/index.toml 无 unmet** → 证明当前所有包的 requires 都有 provider，无静默依赖缺口）
- **运行时**：起 core，无 "Unmet requirement" 警告（不误报，与单测一致）。

## 4. 验证边界 / 后续
- **semver 版本求解 + 依赖拓扑排序 + 冲突消解未做**（§1，当前无 `[dependencies]` 数据、capability 模型不需要）。等第三方包生态出现真实包版本依赖时再做，届时 `find_missing_dependencies` 的 `required_version` 可接入 semver crate 做约束匹配。
- 当前校验在「索引层」（声明的 provides/requires），不校验「该 provider 是否真能加载」（stub/缺产物由 B1 validator 覆盖）。两者互补。
