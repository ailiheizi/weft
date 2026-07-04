# WEFT vNext Phase 5: Weft Claw Productization Plan

## Goal

把 weft-claw 从“架构样板”进一步推进到“真正最小可用产品”。

## Current Baseline

当前已具备：

- `core.execution` 已接入 weft-claw
- product-level UI route 已存在
- generation / verify / activate / rollback 已接入
- lock / digest / trust / signature 已接入

## Phase 5.1 - UI Product Closure

### Focus

- 明确 weft-claw UI 的 product-level 入口说明
- 检查 `weft-claw-ui` 是否仅为静态资源，还是需要更明确的最小交互路径
- 明确 UI metadata / route / asset_root 对外契约

## Phase 5.2 - More Signed Sample Coverage

### Focus

- 扩大真实 signed package 样例覆盖面
- 与 weft-claw 默认依赖保持一致
- 确保 lock / verify / runtime 路径一致消费这些样例

## Phase 5.3 - Legacy Special-case Cleanup Plan

### Focus

- 盘点仍依赖 legacy plugin/suite 假设的路径
- 形成迁移顺序，而不是一次性重写

## Phase 5.4 - Docs & Migration Completion

### Focus

- 对齐 overview / package / lock / policy / workflow / weft-claw blueprint
- 对齐 `.vnext-migration/STATUS.md`
- 明确哪些已经实现，哪些是后续计划

## Phase 5.5 - Final Acceptance

### Manual QA

- `cargo build -p weft-core`
- `cargo test -p weft-core`
- `cargo build -p registry-server`
- `cargo build --manifest-path core/plugins/installed/native-echo/Cargo.toml`
- `cargo run -p weft-core --bin weft-sign -- ...`

## Acceptance Criteria

- weft-claw 作为最小可用 Product Package 的关键链路清晰
- 签名样例、lock、verify、runtime gate 一致
- 文档与代码状态一致
