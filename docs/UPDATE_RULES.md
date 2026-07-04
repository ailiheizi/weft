# 更新规则

## 1. 目标

本规则用于维护 `WEFT-plug/docs/` 目录，确保这里的内容可以直接作为上传资料使用。

## 2. 正式内容优先级

优先维护以下五类内容：

1. 当前主线架构（`architecture/vNext/`）
2. **SaaS 规划三组**（`saas-features/` / `server/` / `client/`）
3. 产品蓝图与设计稿（`superpowers/specs/`）
4. Wiki 开发者向代码地图（`wiki/Developers/`）
5. 少量必要的实施计划资料（`superpowers/plans/`）

默认不把 brainstorming 草稿、`legacy/`、`chores/` 作为正式上传主内容。

## 2.1 新增目录归属（2026-05-26 起）

| 内容类型 | 放哪 |
|---|---|
| SaaS 商业 / 功能规划 | `saas-features/` |
| 服务端架构 / 阿里云 / 迁移 | `server/` |
| 客户端架构 / 手机端 / 包隔离 | `client/` |
| 使用 / 构建指南 | `guides/` |
| 早期被替代的文档 | `legacy/` |
| git 维护记录 | `chores/` |

## 3. 更新前必须读取的来源

### 3.1 git 提交追踪来源

每次准备更新正式文档前，必须先读取以下目录中**已经 push 的提交**：

- `WEFT-plug/core/`
- `WEFT-plug/packages/weft-claw/`

目的：

- 判断当前主线架构是否发生变化
- 判断 `weft-claw` 产品装配与 package 声明是否发生变化
- 为 `docs/CHANGELOG.md` 提供简短更新说明

### 3.2 快照来源

以下内容当前按文件快照理解：

- `WEFT-plug/packages/weft-code/`
- `WEFT-plug/docs/architecture/`
- `WEFT-plug/docs/superpowers/specs/`
- `WEFT-plug/docs/superpowers/plans/`

其中 `packages/weft-code/` 当前不是独立 git 仓库，因此更新时只能写“快照已刷新”或“设计稿已同步”，不能伪造 commit 来源。

## 4. 每次更新的固定流程

1. 读取 `core/` 已 push 提交
2. 读取 `packages/weft-claw/` 已 push 提交
3. 判断是否影响以下主题：
   - 当前主线架构
   - vNext 核心模型
   - Weft Claw 产品蓝图
   - WEFT-Code 设计稿
   - package / capability / binding 关系
4. 若影响，则更新对应文档或根目录说明
5. 在 `CHANGELOG.md` 中追加一段简短说明
6. 检查是否仍保持“主线架构优先、蓝图直达、草稿降级”的整理原则

## 5. 根目录 README 的维护要求

`docs/README.md` 应始终做到：

- 明确哪些文件最适合直接上传
- 明确哪些目录是背景资料
- 明确哪些目录是过程稿或次优先级资料

不要把 README 写成独立文档仓库说明。

## 6. CHANGELOG 写法

每次只需简短记录：

- 日期
- 这次整理了哪类文档
- 变化依据来自哪里（`core` / `packages/weft-claw` / `weft-code 快照`）

不要：

- 粘贴大量 commit 列表
- 记录未 push 的本地变化
- 把快照来源写成 git 历史来源

## 7. 上传建议

如果后续要把 `docs/` 内容同步到独立文档仓库或外部平台，优先上传：

- `architecture/vNext/00-overview.md`
- `architecture/vNext/products/weft-claw/09-product-blueprint.md`
- `superpowers/specs/2026-04-10-weft-claw-package-blueprint-design.md`
- `superpowers/specs/2026-04-15-weft-code-design.md`

如需补充实施背景，再从 `superpowers/plans/` 里挑选少量关键计划文档。
