# WEFT vNext Phase 3: Server Source Authority Plan

## Goal

在不打断当前 Core 主线的前提下，把 `registry-server` 演进为未来的 package source authority。

## Scope

本阶段已进入“最小落地 + 后续递进”的状态：当前先把只读 source authority、Core HTTP consumption、revision/version 与本地快照 fallback 收进主线；更完整的 artifact authority、PKI、发布后台仍按后续阶段推进。

## Current Baseline

当前已具备：

- `server` 已暴露 package source 路由骨架
- `core` 已具备 `PackageIndex` 与 `AppSourceIndex`
- package 元数据已包含 `trusted` / `signature` / `source_authority` / `source_public_keys`
- lock / verify / runtime gate 已能消费这些元数据

## Phase 3.1 - Read-only Source Index Authority

### Objective

让 server 成为官方 source index 的只读发布点。

### APIs

- `GET /api/sources/packages`
- `GET /api/sources/products`
- `GET /api/sources/products/{name}`

### Requirements

- 返回稳定 JSON/TOML 结构
- 返回 source-level trust/signature metadata
- 返回 revision/version 信息用于缓存

## Phase 3.2 - Core HTTP Consumption

### Objective

让 Core 可以优先从 server 拉取 source index，而本地 index 作为 fallback。

### Requirements

- config 中声明 source authority URL
- 本地缓存 package source index
- 当 server 不可用时回退到本地快照
- resolve/verify/activate 继续使用统一内存模型

## Phase 3.3 - Artifact / Release Authority

### Objective

让 server 不仅提供 source index，还提供 release/artifact 元数据。

### Requirements

- release digest
- signature
- provenance
- source authority public keys
- compatible runtime/platform 信息

## Phase 3.4 - Trust & Signing Rollout

### Objective

让 server 成为 source authority 和公钥 authority 的发布点。

### Requirements

- authority -> public keys mapping
- key rotation metadata
- revocation metadata
- CI/generated artifact signing integration

## Deferred Until Core Priority Ends

以下内容后置：

- server 写路径/发布后台
- UI 管理界面
- 完整 PKI/组织级信任模型

## Acceptance Criteria

- 计划与当前代码状态一致
- server 后续工作不阻塞当前 Core 主线
- 未来 Core 接入 server 时不需要重做 package/lock/verify 模型
