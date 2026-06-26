# Git 版本控制 领域知识

## 可用工具
| 工具 | 功能 | 备注 |
|------|------|------|
| `git_status` | 查看工作区状态（已暂存/未暂存/未跟踪文件） | 仅需 `repo_path` |
| `git_diff_unstaged` | 查看工作区中未暂存的改动 | 对比工作区与暂存区 |
| `git_diff_staged` | 查看已暂存（待提交）的改动 | 对比暂存区与 HEAD |
| `git_diff` | 与指定目标（分支/commit）对比差异 | 必填 `target` |
| `git_add` | 将文件加入暂存区 | `files` 数组，可用 `["."]` 全量 |
| `git_reset` | 取消所有已暂存改动（unstage） | 等价 `git reset`，不影响工作区文件内容 |
| `git_commit` | 提交暂存区内容 | 必填 `message`，仅提交已 add 的内容 |
| `git_log` | 查看提交历史 | 可选 `max_count`（默认 10 条） |
| `git_show` | 查看某个 commit 的详情与改动 | 必填 `revision` |
| `git_create_branch` | 创建新分支 | 必填 `branch_name`，可选 `base_branch` |
| `git_checkout` | 切换到指定分支 | 必填 `branch_name`，仅切换不创建 |
| `git_init` | 初始化一个新的 Git 仓库 | 必填 `repo_path` |

## 工具选择指南
- 想知道"现在改了什么/有哪些文件变动" → `git_status`
- 看具体改了哪几行（还没 add） → `git_diff_unstaged`
- 看即将提交的内容（已 add） → `git_diff_staged`
- 和某个分支/历史 commit 比较 → `git_diff`
- 暂存改动 → `git_add`；撤销暂存 → `git_reset`
- 提交 → `git_commit`（务必先 `git_add`）
- 查历史 → `git_log`；看单个提交细节 → `git_show`
- 开新分支 → `git_create_branch`；切分支 → `git_checkout`
- 全新目录建仓 → `git_init`

## 技能协作流程
- 提交标准流程：`git_status` 先看全局 → `git_diff_unstaged` 确认改动正确 → `git_add` 暂存目标文件 → `git_diff_staged` 复核暂存内容 → `git_commit` 写清晰信息提交。
- 排查历史 bug：`git_log` 定位可疑提交 → `git_show` 看该提交的完整 diff，避免逐文件盲读。
- 特性开发隔离：`git_create_branch` 建分支 → `git_checkout` 切入 → 改动后按提交流程落地，避免直接动主分支。
- 误暂存补救：发现 `git_add` 多加了文件，用 `git_reset` 清空暂存区后重新精确 add。

## 常见陷阱
- `git_commit` 只提交已暂存内容，不会自动包含未 add 的改动；提交前务必先 `git_add`。
- `git_create_branch` 只创建不切换，创建后需再调 `git_checkout` 才会真正切过去。
- `git_reset`（无参数）仅取消暂存，不会删除工作区改动；它不是 `reset --hard`，无法用来丢弃文件内容。
- 几乎所有工具都需 `repo_path` 指向仓库根目录，且应为绝对路径；未 `git_init` 的目录会直接报错。
- `git_log` 默认只返回最近 10 条提交，需要看更早历史时显式调大 `max_count`；排查长链路时别误以为历史只有这些。
- `git_diff` 的 `target` 必填，省略会报错；只想看本地未提交改动应改用 `git_diff_unstaged` / `git_diff_staged`。
- `files` 路径需相对仓库根目录解析；`git_add ["."]` 会暂存全部改动，注意可能误带入不应提交的文件。
