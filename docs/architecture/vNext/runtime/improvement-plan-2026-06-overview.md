# WEFT 提升计划 — 实施总览（2026-06-04 ~ 06-05）

> 起因：分析 Clawith / paperclip / openakita 三个参考项目，对标 WEFT（weft-core 内核 / WASM 包系统 / Flutter 客户端），产出 Top 提升清单并落地。
> 本文是 21–29 共 9 篇实施文档的索引 + 状态总表。

## 状态总表

| 项 | 内容 | 子系统 | 结果 | 最高验证 | 文档 |
|---|---|---|---|---|---|
| A1 | WASM host_fn 权限门控 | core | ✅ 落地 | 运行时（抓 session-events 越权→修） | — |
| A2 | 故障隔离沙箱（熔断器） | core | ✅ 落地 | 运行时 + 6 单测 | [24](./24-a2-circuit-breaker.md) |
| A3 | 编排去硬编码包名 | core | ✅ 落地 | 运行时（capability 解析） | [25](./25-a3-capability-resolution.md) |
| B1 | manifest validator | package | ✅ 落地 | 运行时（抓真问题 + product 豁免） | [23](./23-b1-manifest-validator.md) |
| B2 | official/installed 分裂 | package | ✅ 落地 | 运行时（lock 重生成 + 孤儿归档） | [21](./21-b2-official-installed-split-diagnosis.md) |
| B3 | 依赖 requires 完整性校验 | package | ✅ 落地 | 3 单测 + 运行时 | [29](./29-b3-dependency-requires-check.md) |
| B4 | 升级扩权需重新批准 | package | ✅ 落地 | 7 单测 + 审查 | [26](./26-b4-permission-escalation.md) |
| B5 | 渐进式披露 | package | ❌ 明确不做 | 调研（prefix-cache 已更优解决） | [27](./27-b5-progressive-disclosure-analysis.md) |
| C1 | config_schema 驱动前端表单 | 跨系统 | ✅ 落地 | 数据通路 + analyze | [28](./28-c1-config-schema-form.md) |
| D1 | 真流式 + 修复停止生成 | client | ✅ 落地 | 数据通路 + analyze | — |
| D2 | Core loopback token 鉴权 | client/core | ✅ 落地 | 运行时（鉴权矩阵） | [22](./22-d2-core-loopback-auth-plan.md) |

> A1 / D1 无独立文档（A1 见各文档交叉引用；D1 改动记录在任务与本表）。

## 最终验证（2026-06-05）
- core：`cargo test --lib` → **272 passed, 0 failed**；`cargo build` exit 0。
- client：`dart analyze lib` → 0 error。

## 附带成果

### 修复的既有 bug（7 个）
1. weft CLI `propose` 空 body POST 缺 `Content-Type`（→ 415）
2. weft CLI 不支持 D2 token（鉴权开启后全线 401 回归）
3. session-events 缺 `[permissions] storage` 声明（A1 运行时抓到越权）
4. config 三端点硬编码只看 `installed/`（official/ 包查不到，C1 时挖出）
5. A1 hot-load / reload 路径未注入 `permissions`（启动路径已对，热加载遗漏）
6. D2 鉴权 middleware：`runtime_token: None` 误返回 401（应放行 = 鉴权未启用）
7. 预存测试 fixture bug：`app::resolve` 测试 `packages` Vec 缺 session-events（git stash 确认非本次引入，顺手修）

### 纠正的诊断误判（3 个）
1. 8 个"stub 包未加载" → 实为有 `target/wasm32-wasip1/` 产物且正常加载（漏看 `resolve_entry_path` fallback）
2. `weft-code` "死条目" → 实为 `kind=product` 产品包（走 declaration 路径，非 wasm 加载）
3. B5 "省 token 优化" → 实为伪优化（prefix-cache 已更优，精简反而破坏缓存 + 降准确率）

### 新增模块
- `core/src/package/circuit_breaker.rs`（A2）
- `core/src/package/validate.rs`（B1）
- `clients/weft_client/lib/features/packages/package_config_dialog.dart`（C1）

## 方法论（事后回顾）
- **先核实再动手**：多次用 grep/read/git-stash 确认现状，避免了 2 处适得其反的改动（B5 精简、B3 空转 semver）。
- **运行时验证 > 编译通过**：D2 回归正是"只验编译没跑全套件"漏掉的；全量 `cargo test` 才是完成判据。
- **没靶子不硬做**：B5 不做、B3 只做有数据的 requires 校验、semver/拓扑留待第三方包生态。

## 备份与可回滚
所有 packages/ 改动（无 git 跟踪）均有 `.bak-*` / `.archive-*`：
- `packages/index.toml.bak-b2-*` / `.bak-cleanup-*`
- `packages/weft-claw/lock.toml.bak-b2-*`
- `packages/*/package.toml.bak-a1-*`（9 个权限声明）/ `.bak-c1-*`（cron config）
- `packages/.archive-b2-*/installed-memory-orphan/`
core / clients 改动受各自 .git 跟踪。

## 未触及（留作未来，各文档"后续"节有详）
- D1 / C1 的 GUI 点击实测（数据通路已验，纯交互层未点验）
- stub 包编译产物供给（归 server v2 / Package Server）
- semver 版本求解 + 依赖拓扑（待第三方包生态有真实 `[dependencies]`）
- config 保存后热应用、扩权 UI 对话框、熔断状态前端展示等
- core release 构建（全程用 debug 验证）
