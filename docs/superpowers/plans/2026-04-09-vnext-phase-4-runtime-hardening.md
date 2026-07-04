# WEFT vNext Phase 4: Runtime Hardening Plan

## Goal

在 Core 主体能力已落地的基础上，补强 runtime hardening、provider lifecycle 和 rollback 安全边界。

## Current Baseline

当前已具备：

- profile / policy enforcement
- runtime trust/signature gate
- workspace sandbox
- generation verify / activate / rollback
- native/wasm 路由与 probe

## Phase 4.1 - Sandbox Hardening

### Focus

- 扩大 `core.files` 的边界测试
- 检查 `core.execution` 的 cwd/env/timeout 边界
- 评估 Safe/Developer/Trusted 在运行时的实际差异是否足够清晰

## Phase 4.2 - Provider Lifecycle Hardening

### Focus

- provider load/unload 失败路径
- probe/build/load 失败后的错误语义
- native provider 路径的加载失败、返回非法 JSON、空指针等异常处理

## Phase 4.3 - Crash-safe Generation Semantics

### Focus

- activate/rollback 前后状态一致性
- candidate -> active -> rollback 的边界测试
- lock 回写失败时的行为定义

## Phase 4.4 - Auditability

### Focus

- verify 结果的可追踪性
- provider 被拒绝的错误可诊断性
- 签名失败/authority 不匹配的错误分类

## Acceptance Criteria

- 新增硬化项都具备 focused tests
- 不破坏当前 generation/lock/verify 主链路
- `cargo test -p weft-core` 保持绿色
