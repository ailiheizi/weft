# Memory Runtime MemPalace Bridge Design

## Goal

把官方 `MemPalace` 作为本机可运行的原文记忆引擎接入 `memory-runtime`，让 `raw verbatim drawer retrieval` 成为新的主检索骨架，同时保留现有 `Postgres` 运行时、工具接口和结构化对象能力。

## Problem

当前 `memory-runtime` 已经有：

- `Postgres` 结构化记忆
- legacy wiki sidecar (removed)
- `Chroma` sidecar / benchmark adapter

但长期记忆效果不稳定，尤其是偏好与细节回忆。主要问题不是“有没有存”，而是：

1. 原文证据没有成为系统的第一检索对象。
2. 检索到的证据和运行时输出之间隔着太多中间层。
3. Benchmark 适配逻辑开始膨胀，但核心记忆骨架仍不够强。

## Scope

本设计只覆盖：

- 官方 `MemPalace` 在本机的完整部署方式
- `memory-runtime` 到 `MemPalace` 的桥接层
- 面向运行时的最小读写接缝
- focused tests

本设计不覆盖：

- 大规模 benchmark 再跑分
- UI、desktop、companion、main 进程
- PostgreSQL schema 大改
- 把 `memory-runtime` 整个替换成 `MemPalace`

## Core Decision

采用 `MemPalace sidecar bridge`，不是平替，也不是纯 benchmark 抄法。

系统职责拆分如下：

- `MemPalace`
  - 原文 drawer 存储
  - Chroma 向量检索
  - wing / room 元数据过滤
  - 返回 verbatim evidence

- `memory-runtime`
  - 服务入口
  - 工具 schema 与宿主接入
  - Postgres 结构化对象记忆
  - 统一调度、回包与后续扩展

因此，`MemPalace` 进入的是“主检索路径”，不是“另一个外挂实验件”。

## Target Architecture

### Write Path

写入 durable memory 时，运行双落：

1. 继续写 `PostgresMemoryStore`
2. 同步写入 `MemPalace` drawer collection

第一阶段不追求把所有历史对象都映射成完整 palace 世界，只先支持一条稳定桥：

- `content`
- `entity_key`
- `facet`
- `category`
- `source_kind`
- `metadata`

映射为 `MemPalace` 可用的：

- `document`
- `wing`
- `room`
- `source_file`
- 其他 metadata

### Read Path

查询时新增一条显式桥接检索：

1. 先查 `MemPalace` verbatim drawers
2. 返回原文命中块与 metadata
3. 再由 `memory-runtime` 决定是否补充 Postgres object rows

第一阶段不做复杂融合，只要求：

- `memory-runtime` 能调用 `MemPalace`
- 能拿到稳定的 verbatim 命中
- 能把结果以统一 payload 暴露给上层

### Deployment

官方 `MemPalace` 必须能在本机被完整部署：

- 本地目录
- 可安装依赖
- 可初始化 palace
- 可执行基本 search

为了避免把第三方源码硬塞进插件目录，优先采用：

- 外部安装目录
- 本地 bridge helper 读取该目录

## Bridge Surface

新增一个非常小的桥接接口：

- `MemPalaceBridge.ensure_ready()`
- `MemPalaceBridge.ingest_memory_entry(...)`
- `MemPalaceBridge.search(...)`

桥接层只负责：

- 定位 palace 目录
- 创建 / 获取 Chroma collection
- 将 runtime entry 映射为 drawer document
- 返回 verbatim 结果

桥接层不负责：

- benchmark prompt 编译
- 复杂 rerank
- 偏好特判

## Why This Direction

因为这条路满足三个目标：

1. 先把强原文检索骨架接进来，而不是继续在旧骨架上补丁。
2. 不放弃现有 `Postgres runtime` 的服务化能力。
3. 不制造第三套完全独立的记忆系统。

## Testing Strategy

Focused tests 先证明机制接通，不证明 benchmark 分数：

1. bridge 能初始化本地 palace / collection
2. 写入一条 runtime memory 后，能在 bridge 侧搜回 verbatim text
3. `memory-runtime` 能暴露 bridge search 结果
4. 不启用 bridge 时，现有行为不被破坏

## Success Criteria

本阶段成功的标准是：

1. 官方 `MemPalace` 在本机有明确可执行的部署路径。
2. `memory-runtime` 内存在真实可调用的 `MemPalace bridge`。
3. bridge 写入和检索都有 focused test 证明。
4. 不引入 benchmark-only 特判、关键词硬编码或 fallback 抽取链。
