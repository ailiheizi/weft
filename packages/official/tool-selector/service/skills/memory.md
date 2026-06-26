# 知识图谱记忆 领域知识

基于实体-关系-观察三元模型的持久化记忆库。数据以 JSONL 形式存储在本地文件中，跨会话保留。核心概念：实体（entity，带 name/entityType）、关系（relation，主动语态连接两个实体）、观察（observation，挂在实体下的离散事实字符串）。

## 可用工具
| 工具 | 功能 | 备注 |
|------|------|------|
| `create_entities` | 批量创建实体节点 | 每个实体需 `name`、`entityType`，可附 `observations`；同名实体自动跳过 |
| `create_relations` | 批量创建有向关系 | 每条需 `from`、`to`、`relationType`；关系用主动语态命名（如 `works_at`）；重复跳过 |
| `add_observations` | 向已有实体追加观察 | 需 `entityName` 与 `contents` 数组；实体不存在会报错 |
| `delete_entities` | 删除实体及其级联关系 | 传 `entityNames` 数组；连带删除涉及该实体的关系 |
| `delete_observations` | 删除实体下指定观察 | 需 `entityName` 与待删观察的 `observations` 数组 |
| `delete_relations` | 删除指定关系 | 需完整匹配 `from`/`to`/`relationType` 三元组 |
| `read_graph` | 读取整个知识图谱 | 无参数；返回全部实体与关系，数据量大时慎用 |
| `search_nodes` | 按关键词搜索节点 | 传 `query`；匹配实体名、类型、观察内容；返回命中实体及其相互关系 |
| `open_nodes` | 按名称精确打开节点 | 传 `names` 数组；返回指定实体及其相互关系 |

## 工具选择指南
- 不知道存了什么 → `read_graph`（小图）或 `search_nodes`（大图，先缩小范围）
- 知道关键词、模糊查找 → `search_nodes`
- 已知确切实体名、要拉取详情 → `open_nodes`
- 记录新对象/人物/概念 → `create_entities`
- 给已存在对象补充事实 → `add_observations`（勿重复 create_entities）
- 建立对象之间的联系 → `create_relations`
- 清理过时信息 → 删观察用 `delete_observations`，删整个对象用 `delete_entities`，删联系用 `delete_relations`

## 技能协作流程
1. 会话开始先 `search_nodes` 或 `read_graph` 回忆已有记忆，避免重复建图。
2. 建图顺序：先 `create_entities` 落地节点，再 `create_relations` 连边（关系两端实体必须已存在）。
3. 增量更新：用 `search_nodes`/`open_nodes` 定位实体，再用 `add_observations` 追加，而非新建同名实体。
4. 信息更正：先 `delete_observations` 删旧事实，再 `add_observations` 写新事实，保持图谱干净。

## 常见陷阱
- `create_relations` 的两端实体须先存在，否则关系无意义；关系名用主动语态、小写下划线（`manages` 而非 `is_managed_by`）。
- `add_observations` 针对不存在的实体会报错，先确认实体已创建。
- `delete_relations` 必须三元组完全匹配才能删除，`relationType` 拼错则无效。
- `read_graph` 返回全量数据，图谱大时严重消耗上下文，优先用 `search_nodes` 精准检索。
- 实体 `name` 是唯一主键，重复 `create_entities` 会被静默跳过而非更新；改内容用 add/delete 系列工具。
- 观察是离散字符串，应拆成独立短句而非塞进一大段，便于后续精确删除与检索。
- 该 server 仅在用户明确要求跨会话记忆时使用，避免无意义写入污染图谱。
