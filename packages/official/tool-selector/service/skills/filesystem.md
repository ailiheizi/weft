# 文件系统 领域知识

## 可用工具
| 工具 | 功能 | 备注 |
|------|------|------|
| `read_file` | 读取单个文件的文本内容 | 仅限允许目录；支持 head/tail 参数只读首/尾若干行，大文件优先用以省上下文 |
| `read_multiple_files` | 一次性读取多个文件内容 | 传路径数组；比逐个调用更高效，单个失败不影响其余 |
| `write_file` | 创建新文件或整体覆盖已有文件 | 全量覆盖，非追加；会无声覆盖原内容 |
| `edit_file` | 基于文本匹配做局部替换编辑 | 传 edits 数组（oldText/newText）；支持 dryRun 预览 diff |
| `create_directory` | 创建目录，支持多级递归创建 | 目录已存在不报错；可一次建嵌套路径 |
| `list_directory` | 列出目录下的文件与子目录 | 输出带 [FILE]/[DIR] 前缀；非递归 |
| `directory_tree` | 递归返回目录的 JSON 树结构 | 适合整体把握项目结构；大目录输出量大 |
| `move_file` | 移动或重命名文件/目录 | 目标已存在会失败；源与目标都需在允许目录内 |
| `search_files` | 按名称模式递归搜索文件/目录 | 模糊匹配文件名，非内容检索；支持 excludePatterns |
| `get_file_info` | 获取文件元数据（大小、时间、权限、类型） | 不读取内容，开销小 |
| `list_allowed_directories` | 列出当前可访问的根目录范围 | 所有操作都受此白名单约束 |

## 工具选择指南
- 不确定能访问哪些目录 → `list_allowed_directories`
- 只想知道文件大小/是否存在/类型 → `get_file_info`（别用 `read_file`）
- 看目录有哪些文件 → 单层用 `list_directory`，整体结构用 `directory_tree`
- 按文件名找文件 → `search_files`（按内容找请用宿主的 Grep 类工具）
- 读 1 个文件 → `read_file`；读多个相关文件 → `read_multiple_files`
- 改文件局部几行 → `edit_file`；从零创建或整体重写 → `write_file`
- 新建目录 → `create_directory`；改名/挪位 → `move_file`

## 技能协作流程
- 探索定位：先 `list_allowed_directories` 确认边界 → `directory_tree` 或 `list_directory` 摸清结构 → `search_files` 精确定位目标文件 → `read_file`/`read_multiple_files` 读取内容。
- 安全编辑：先用 `read_file` 读到准确原文 → `edit_file` 带 dryRun=true 预览 diff 确认无误 → 再正式应用，避免误改。
- 批量修改前先建好目录：`create_directory` 准备好目标路径 → `write_file` 写入，避免因父目录缺失失败。
- 重组文件：`get_file_info` 确认源存在与目标不冲突 → `move_file` 执行重命名或迁移。

## 常见陷阱
- 所有路径必须落在 `list_allowed_directories` 返回的白名单内，越界操作会被拒绝；不确定时先查白名单。
- `write_file` 是整体覆盖而非追加，对已有文件会无声清空原内容；改局部务必用 `edit_file`。
- `edit_file` 的 oldText 必须与文件现有内容逐字符精确匹配（含缩进/换行），匹配不到会失败；建议先 `read_file` 取原文、再用 dryRun 验证。
- `search_files` 只匹配文件名，不搜文件内容；要按内容查找需借助宿主的内容搜索工具。
- `directory_tree` 和 `read_multiple_files` 在大目录/大文件上会产生大量输出，注意节省上下文，能用 `get_file_info`、`list_directory` 或 `read_file` 的 head/tail 替代就替代。
- `move_file` 目标已存在时会失败，不会自动覆盖；需要替换请先处理目标文件。
