# A2：故障隔离沙箱（Circuit Breaker，已落地）

> 状态：核心逻辑 + 接入 + 编译 + 单测(6) + 运行时(健康路径) 全通过。
> 关联：`core/src/package/circuit_breaker.rs`、`bridge.rs`（两条 call 路径）、A1（同 per-package 基础设施）。

## 1. 问题

反复 trap/panic 的 WASM 包会被无限重复调用：每次失败都耗一次锁 + 实例化 + 崩溃日志刷屏，且可能拖垮依赖它的能力链。原先 `WasmPackageHost::call` / `WasmHandle::call` 对失败无任何记忆。

## 2. 设计

独立模块 `circuit_breaker.rs`，per-package 三态熔断器：

```
Closed ──失败数≥阈值/窗口──> Open ──冷却到期──> HalfOpen ──探针成功──> Closed
                                                          └─探针失败──> Open
```

- `CircuitBreaker`：`Arc<Mutex<HashMap<pkg, Entry>>>`，Clone 共享（两条 call 路径同一状态）。
- 滑窗：最近 `window` 次结果环，失败数 ≥ `failure_threshold` → Open。
- `check_at(pkg, now)` → `Err` 表示当前 Open 且冷却未到，调用方直接拒绝、不碰插件。
- `record_at(pkg, success, now)` → 更新状态机。
- 时间注入（`_at` 版本收 `now: Instant`）→ 状态机可确定性测试；生产用 `check()`/`record()` 包装真实时钟。
- 默认配置（保守，避免误伤）：`window=10, failure_threshold=5, cooldown=30s`。

**"失败"的定义**：host 侧 `Plugin::call` 返回 `Err`（trap/panic/实例化错误）才算。guest 在 `Ok` 里返回的 `{"error":...}` 业务错误**不算**——那是正常返回。

## 3. 接入（bridge.rs）

两条调用路径都接入（关键：API handler 走 `WasmHandle::call`，内部走 `WasmPackageHost::call`，两者都要拦）：
- 调用前 `breaker.check(pkg)`：Open 且冷却内 → `bail!` 拒绝。
- 调用后 `breaker.record(pkg, outcome.is_ok())`：在 `?` 早返回前显式记录，失败也计入。
- `WasmHandle::call` 在同一短 host 锁内 clone breaker handle，锁外 check/record（不延长持锁）。
- `unload_package`（reload 也经它）调 `breaker.reset(pkg)`：上传新版本给坏包重新机会。

`WasmPackageHost` 新增 `breaker: CircuitBreaker` 字段，构造时 `CircuitBreaker::default()`。

## 4. 验证

- `cargo build` exit 0；`cargo test --lib package::circuit_breaker` → **6 passed**：
  healthy-stays-closed / trips-open-after-threshold / occasional-below-threshold / halfopen-probe-success-closes / halfopen-probe-failure-reopens / reset-clears。
- `cargo test --no-run` exit 0（A2 改动不破坏既有测试）。
- **运行时（健康路径）**：起 core，weft-claw 连续 3 次 `list_sessions` 全部成功（经 `WasmHandle::call` 的 check+record），**零误熔断**（无 "circuit is open" 日志）、无 WASM trap。

## 5. 验证边界
- **故障路径未端到端实测**：触发真实熔断需一个会 trap 的 WASM 包（需专门编译）。状态机由 6 个单测覆盖（含 Open/HalfOpen/reopen/reset 全转移），但"真包 trap 5 次后被拒"未在运行时造。
- 熔断仅记录 host 侧 `Plugin::call` 的 Err；若某类故障表现为 guest 返回 Ok+错误 JSON，则不被熔断（设计如此）。

## 6. 后续可扩展（未做）
- 熔断状态暴露到 `/api/packages/{name}` 或新增 `/api/packages/{name}/health`（前端显示"已熔断"）。
- 熔断/恢复事件写入 session events 或指标。
- 配置可调（当前 default 硬编码；可由 core policy / package.toml 提供 per-package 阈值）。
- A1 遗留（顺带发现）：`load_package`（hot-load 路径，bridge.rs:1491）的 `per_plugin_state` 未注入 `info.permissions`，hot-loaded 包会拿默认空权限。启动路径（`WasmPackageHost::new`）已正确注入，仅 hot-load 这条遗漏。
