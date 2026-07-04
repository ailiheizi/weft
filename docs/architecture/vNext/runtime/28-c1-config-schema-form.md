# C1：config_schema 驱动前端配置表单（已落地）

> 状态：client 动态表单 + core 端点 bug 修复 + 范例 schema，编译/analyze/schema 数据通路验证通过。
> 关联：`clients/weft_client/lib/features/packages/package_config_dialog.dart`、`core/src/api/packages_runtime.rs`（config 三端点）、`packages/official/cron/package.toml`（范例）、B4（PackageConfigField）。

## 1. 现状与缺口

- core 早已有三端点：`GET .../config/schema`、`GET .../config`（secret 已遮罩）、`PUT .../config`。
- client 有 `getPackageConfig`/`savePackageConfig`，但**缺 schema 调用 + 无任何配置 UI**。
- **无任何包声明 `[config]` 字段** → 该机制此前无消费者、无法验证。

## 2. 实现（三部分）

### a) client 动态表单（lib/features/packages/）
- `core_repository.dart`：新增 `getPackageConfigSchema(name)`。
- 新建 `package_config_dialog.dart`（554 行）：`PackageConfigDialog`，initState 用 `Future.wait` 并行拉 schema+config，按字段名排序渲染（稳定顺序），按 type 渲染控件：
  - string → TextField（secret → obscureText + 显隐切换）
  - number → 数字 TextField + min/max 校验
  - boolean → SwitchListTile
  - select → DropdownButtonFormField(options)
  - 未知 → 退化 string
- 初值优先级：当前 config > schema.default > 空。required 标 `*` 并校验非空。loading/error/saving 三态，dispose 释放 controller。
- **secret "留空保持不变"**：当前值为 `****` 时输入框留空 + 提示，保存时若仍空则跳过该字段（不把 `****` 写回）。
- `packages_screen.dart`：`_PackageTile` 加 `Icons.tune` 齿轮入口 → showDialog 打开 PackageConfigDialog。

### b) core config 端点 bug 修复（顺带挖出）
原 `managed_packages_dir` 硬编码只看 `packages/installed/`，但很多运行包（按 index.toml `current_source`）源在 `packages/official/` → schema/get/save 三端点对这些包**查不到**（cron 的 schema 返回空就是此 bug）。
- 新增 `resolve_managed_package_dir(state, name)`：用 `resolve_runtime_package`（按 index `current_source` 解析真实 dir），回退 `installed/<name>`。
- schema/get/save 三端点统一改用它（读写路径一致）。这是与 B2 同源的"只看 installed"遗留 bug。

### c) 范例 schema（packages/official/cron/package.toml）
加 `[config.*]` 5 字段覆盖全类型：`timezone`(string,required) / `max_concurrent_jobs`(number,min1 max64) / `catch_up_missed`(boolean) / `log_level`(select 4 options) / `webhook_secret`(string,secret)。`[config]` 是可选元数据段，不影响加载（B1 validator 也不检查）。备份 `package.toml.bak-c1-20260605`。

## 3. 验证

- client：`dart analyze lib/features/packages/ lib/core/providers/core_repository.dart` → **No issues found!**
- core：`cargo build` exit 0。
- **数据通路（运行时）**：起 core（health 轮询确认就绪），`GET /api/packages/cron/config/schema` 返回全部 5 字段、类型正确、`webhook_secret` 标 `secret=true`。修复前返回空 `[]`，修复后正确。

## 4. 验证边界
- **client 表单 GUI 渲染未点验**：5 字段渲染成对应控件、secret 留空逻辑、保存流程未起 Flutter app 实测。schema 数据通路已实测正确、analyze 通过、designer 按契约实现。
- save 写 config.json 到解析出的包目录（official/cron），与读路径一致；写后 secret 遮罩依赖 manifest schema 的 `secret` 标记（已验证标记正确）。

## 5. 后续
- 给更多包补 `[config]` schema（当前仅 cron 范例）。
- 保存后让 package 重载/通知运行时应用新配置（当前仅写 config.json）。
