# B5：渐进式披露 — 调研结论：不做（已被更优方案解决）

> 状态：调研完成，**决定不实现**。理由如下，全部基于实读 agent-core 源码。
> 关联：`packages/official/agent-core/src/lib.rs`（build_completion_request / build_tool_catalog_prompt）。

## 1. 原设想

B5 = 渐进式披露：system prompt 只放精简工具清单（manifest-only），完整 schema 按需加载，以降低 LLM token 成本。

## 2. 靶子确实存在（但已被处理）

实测 `data/agent-core-last-outbound-request.json`：
- system prompt = 10486 字符，其中 `[Available tools]` 全量 JSON schema 占 **8209 字符（78%）**，14 个工具 description+parameters 全展开。
- agent-core 不用 OpenAI function-calling，工具描述全塞进 system prompt 的 `[Available tools]` 段。

## 3. 为何不做：已有 prefix-cache 是更优方案，B5 会适得其反

读 `build_completion_request`（lib.rs:2343-2392）发现 agent-core **已经专门为 token 成本做了优化，方向与 B5 相反但更优**：

- **L2346-2356**：工具按名排序，保证**字节级稳定前缀** —— 注释原文："DeepSeek prefix cache requires byte-identical prefixes across requests; random tool ordering breaks cache hits"。
- **L2381-2383**：稳定块（tools + response contract）放最前，per-request 动态内容在后 —— 注释原文："maximises DeepSeek automatic prefix cache hits: the stable block is written to disk cache on the first request and reused on subsequent ones"。
- **L1607-1614** `fingerprint_str`：追踪 system prompt+tool catalog 稳定前缀的哈希,检测跨轮 mutation 以保护上游 prompt-cache。

**含义**：全量 tool schema 被**故意**放进稳定前缀来命中上游自动前缀缓存。缓存命中后，8K schema 的边际 token 成本趋近于零，同时**完整保留了工具调用准确率**（LLM 有精确参数 schema 可填参）。

**B5 精简的三重代价**：
1. 改 tool catalog 内容 → 改变稳定前缀 → 可能降低缓存命中率（破坏已有优化）。
2. 主 agent 工具多、场景杂，靠"参数摘要"而非完整 schema 填参 → 复杂参数工具调用准确率下降。
3. 省的只是首请求/缓存失效时的 token，远不抵 1+2 的代价。

精简渲染逻辑（`build_tool_catalog_prompt` 的 delegate 分支，L2165-2198）**已存在**，但仅用于 delegate（子 agent 聚焦、工具少）场景 —— 这恰恰印证了"主 agent 不该精简"的权衡是已被开发者考虑过的。

## 4. 结论

B5 在当前 agent-core 架构下是**伪优化甚至有害**：token 问题已由 prefix-cache 更好地解决（省成本且不损准确率），B5 的精简会破坏缓存稳定性并降低工具调用可靠性。**不实现。**

## 5. 若未来仍要降 token，正确方向（非 B5）
- 提升 prefix-cache 命中率（确保更多部署/provider 启用自动前缀缓存）。
- 对**确实很少用**的工具做懒注册（按 agent 角色裁剪 tool 列表，而非精简每个工具的 schema）—— 这减少工具**数量**而非**单工具描述**，不损已选工具的准确率。
- session-history 的 fold（已实现，见 lib.rs:1625+ SESSION_HISTORY_TOKEN_BUDGET）才是动态增长部分的真正优化点。
