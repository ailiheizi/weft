# Memory Runtime Precise Long-Term Memory Design

## Goal

在 `weft-plugins/memory-runtime` 内，把当前 PostgreSQL + tag 图记忆机制补成真正可用的长期记忆闭环：

- 长期持久化
- 更精确的检索
- 同对象同维度的版本/冲突判定
- 会话后自动提炼成 durable facts

范围只包含：

- `weft-plugins/memory-runtime/*`
- `weft/scripts/test-weft-memory-runtime-host.ts`

不碰 desktop UI、不碰 main、不碰其他插件。

## Core Decision

这次不引入多种 memory object type。

统一使用一种 durable fact object，只在事实上增加固定字段：

- `entity_key`
- `facet`
- `validity`
- `supersedes_fact_id`
- `source_kind`
- `evidence`

这样既能保持模型简单统一，也足够表达：

- 这条记忆是关于谁/什么
- 这条记忆在说它的哪一面
- 这条记忆现在是否仍有效
- 它是否覆盖了旧事实
- 它来自哪里

## Retrieval Model

检索继续保留当前优势：

- PostgreSQL 全文检索
- tag 提取
- tag 共现扩展
- trust / residual 排序

但增加两条精确约束：

1. `search` 默认只返回 `active` / `uncertain` 事实，不默认把历史和失效事实混进来。
2. 同时把 `entity_key` / `facet` 纳入检索和排序，让“同对象同维度”的事实更容易被稳定召回。

## Version And Contradiction Model

当两条事实具有相同的 `entity_key + facet` 时，系统需要做状态收口：

- 内容相同：视为同一事实，更新即可
- 内容不同且带明显演化/替换语义：新事实 `active`，旧事实 `superseded`
- 内容不同但没有明确演化语义：判定为冲突，相关事实标记为 `contradicted`

这次只做统一、可解释的规则式实现，不引入复杂 ML 分类器。

## Session Extraction

当前 `sync_turn()` 是空的，`on_session_end()` 只有很浅的正则抽取。

这次改成统一的会话提炼路径：

- 把 turn / session 里的文本切成短句
- 从短句中提炼 preference / decision / constraint / project fact
- 生成 durable fact object
- 标记 `source_kind = session_extract`
- 在 `evidence` 中保留原句摘要

这样机制上就支持“对话越多，长期记忆越完整”。

## Runtime Surface

为了让 host test 能真正验证机制，这次在 runtime webhook 内增加最小调试/注入动作：

- `sync_turn`
- `session_end`

它们不是新的 AI-facing memory type，只是把 provider 生命周期能力暴露给测试和服务运行时。

## Success Criteria

完成后应满足：

1. `fact_store(add/list/search)` 能返回新的事实字段
2. 同 `entity_key + facet` 的新旧事实会自动收口到 `active / superseded / contradicted`
3. `contradict` 不再是空实现
4. `sync_turn` / `session_end` 能自动提炼 durable facts
5. host test 能覆盖上面这些行为

