# B4：升级扩权需重新批准（已落地）

> 状态：核心逻辑 + 接入 install_package + 编译 + 单测(7) 全通过。
> 关联：`core/src/package/permissions.rs`、`api/packages_runtime.rs`（install_package）、A1（同权限模型）。

## 1. 问题

包升级（`POST /api/packages/install` 从 store 下载新版本覆盖旧版本）原先对权限变化无任何检查：一个原本只要 `storage` 的包，新版本可以静默声明 `process`+`network`，安装即生效。这是供应链扩权风险——用户批准过的是旧版本的权限面，升级不该偷偷扩大它。

## 2. 实现

### 权限差分（permissions.rs，纯函数可测）
- `Permission::ALL`：11 个权限变体的常量数组，供穷举。
- `escalated_permissions(old, new) -> Vec<Permission>`：返回 `new` 有而 `old` 无的权限（即升级引入的扩权）。空 = 无扩权，安全。
- 复用 `PermissionChecker::check` 判断单个权限是否授予，避免重复字段访问逻辑。

### install_package 拦截（packages_runtime.rs）
在**写盘覆盖旧 package.toml 之前**插入检查：
1. 旧权限：若 `installed/<name>/package.toml` 已存在，`load_manifest` 读旧 `permissions`；否则全 false（全新安装无旧版，不算扩权）。
2. 新权限：`parse_manifest_permissions(manifest_bytes)` 从下载的 manifest 提取 `[permissions]` 段（轻量 struct，方言无关——`[package_info]` / `[identity]+[package]` 都适用）。
3. `escalated_permissions(old, new)`：非空且 `body.approve_escalation != true` → 返回 **409 Conflict**，body 含 `escalation`（新增权限名列表）+ `requires_approval: true`。
4. `InstallPackageRequest` 新增 `approve_escalation: bool`（`#[serde(default)]`）——显式批准后放行。

## 3. 验证

- `cargo build` exit 0；`cargo test --no-run` exit 0。
- `permissions` 模块：5 passed（含 `no_escalation_when_permissions_unchanged` / `detects_added_permissions` / `dropping_permissions_is_not_escalation`）。
- `b4_tests`（packages_runtime.rs）：4 passed —— `parse_manifest_permissions` 对两种方言正确提取、缺 `[permissions]` 段零授予、升级加 `process` 被 `escalated_permissions` 标记。

## 4. 验证边界

- **端到端未实测**：触发真实 409 需一个 store server + 一个声明扩权的新版本包。逻辑由 7 个单测覆盖（diff 正/反/降权 + 方言解析 + 扩权检测），install_package 的接入点经代码审查（写盘前、旧权限读取、approve 豁免）。
- 降权（新版本要更少权限）正确地**不**触发（`dropping_permissions_is_not_escalation` 验证）。
- `reload_package`（本地重载，非 store 升级）未加此检查——它重载同一磁盘 manifest，无"新旧版本"语义；扩权风险仅存在于 store 下载路径。

## 5. 后续可扩展（未做）
- 前端 UI：收到 409 时弹出"此升级请求新权限 [...],是否批准?"对话框，批准后带 `approve_escalation=true` 重试。
- 持久化"已批准的权限面"到 package 元数据，供审计。
- 扩展到 capability provides 的变化检测（新版本声明提供更多 capability 也可能需审查）。
- 与 06-profiles-and-policy 的 profile 联动:Safe profile 下扩权一律拒绝、Trusted 下放行。
