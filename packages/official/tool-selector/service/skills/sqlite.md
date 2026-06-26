# SQLite 数据库 领域知识

## 可用工具
| 工具 | 功能 | 备注 |
|------|------|------|
| `read_query` | 执行 `SELECT` 只读查询并返回结果行 | `query` 必填；仅允许以 SELECT 开头的语句，写操作会被拒绝 |
| `write_query` | 执行 `INSERT` / `UPDATE` / `DELETE` 写入语句 | `query` 必填；不能用于 SELECT，也不能用于建表 |
| `create_table` | 执行 `CREATE TABLE` 创建新表 | `query` 必填，且必须是建表语句 |
| `list_tables` | 列出数据库中所有表名 | 无参数；用于探查库结构的第一步 |
| `describe_table` | 返回指定表的列名、类型等 schema 信息 | `table_name` 必填，需为已存在的表 |
| `append_insight` | 向 memo 资源追加一条业务洞察 | `insight` 必填；写入备忘录而非数据表，不影响数据 |

## 工具选择指南
- 查数据 / 读取行 → `read_query`
- 改数据（增删改）→ `write_query`
- 新建表结构 → `create_table`
- 不知道有哪些表 → `list_tables`
- 知道表名但不清楚字段 → `describe_table`
- 记录分析结论 / 业务发现（非数据本身）→ `append_insight`

## 技能协作流程
- 探查再操作：先 `list_tables` 看全貌，再对目标表 `describe_table` 拿到列名和类型，最后才 `read_query` / `write_query`，避免凭空猜字段名导致 SQL 报错。
- 建表后即校验：`create_table` 之后用 `describe_table` 确认 schema 已按预期落地，再开始 `write_query` 灌数据。
- 写前先读：执行 `UPDATE` / `DELETE` 前，先用 `read_query` 加相同 `WHERE` 条件确认命中行数，确认无误再 `write_query`，防止误伤。
- 分析沉淀：在多轮查询得出结论后，用 `append_insight` 把要点写入 memo，供后续会话或报告复用，而不是塞进数据表。

## 常见陷阱
- 工具按语句类型严格分流：SELECT 只能走 `read_query`，增删改只能走 `write_query`，建表只能走 `create_table`，混用会被拒绝。
- 所有 query 类参数都必须是单条 SQL 字符串，避免一次塞多条语句或加多余分号。
- `describe_table` 的 `table_name` 不存在会报错，不确定时先 `list_tables`，并以其返回的准确表名为准（SQLite 表名匹配大小写不敏感，但建议照抄原名）。
- `write_query` 的 `DELETE` / `UPDATE` 若漏写 `WHERE`，会作用于全表，属高风险操作，务必先确认条件。
- `append_insight` 只写备忘录，不会修改任何表数据；需要持久化结构化数据时仍要用 `write_query`。
- 大结果集会占用大量上下文：`read_query` 尽量加 `WHERE`、`LIMIT` 和指定列，不要 `SELECT *` 拉全表。
- 该 server 面向单个 SQLite 文件，无跨库 / 事务回滚保障，批量写入前应自行评估可恢复性。
