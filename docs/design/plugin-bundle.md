# MCP 工具集成设计 — 决策记录

> 状态: **已实现(v1)** + 未来方向
> 最后更新: 2026-07-04
> 关联: [mcp-integration.md](./mcp-integration.md)

---

## 背景

早期设想过"MCP + skill 捆绑插件"(一个插件 = MCP server + 教 agent 用法的 skill,
一键开关)。经过对竞品(Claude Desktop / Cursor / ChatGPT / Cherry Studio / Cline)
和 MCP 协议本身的调研,**放弃了 skill 捆绑方案**。本文记录决策依据。

---

## 关键决策

### 决策 1: 不做 skill+MCP 捆绑,MCP tool schema 已足够

**依据:**
- MCP 协议里每个 tool 自带 `name + description + inputSchema`,直接进模型上下文
- 现代 LLM function-calling 靠这些信息就能正确选择和调用工具
- 额外的 skill 提示词只在**复杂多步骤工作流、领域约定、安全规则**时才有价值
- 工具用不好的正确修法是**改进 tool description**,不是外挂 skill

**结论:** 默认信任 tool schema。skill 层作为未来的可选增强,不作为 MCP 的前置依赖。

### 决策 2: MCP 添加 = 粘贴 JSON 导入(兼容全生态)

**依据:**
- `claude_desktop_config.json` 的 `mcpServers` 格式是事实标准
- Claude Desktop / Cursor / Cline / VS Code / Smithery 都用这个格式或其变体
- 支持粘贴导入 = 瞬间兼容全网上千个 MCP server,零维护

**实现(clients/weft_client, management_tabs.dart):**
- 粘贴 JSON,兼容 `mcpServers` / `servers` 两种键 + 可选 `type` 字段
- 实时解析预览,展示每个 server 的 name / command / args
- 命令安全提示:
  - 常规命令(npx/node/python/uvx/docker/deno/bun...) → 绿色 ✓
  - 可疑命令(cmd/powershell/bash/curl/rm...) → 红色警告"请确认来源可信"
  - 未知命令 → 黄色提示
- 用户确认后才安装,不自动执行

### 决策 3: 远程 URL 作为零依赖选项

**依据:**
- 远程 MCP(Streamable HTTP / SSE)不需要本地安装任何运行时
- 是行业趋势,对无环境用户最友好

**实现:** "远程 MCP (URL)" 入口,填 URL 即连,transport=sse。

### 决策 4: 环境依赖问题交给 AI,不做检测/内置

**依据:**
- 大部分 MCP 用 `npx`(需 Node)或 `uvx`(需 Python),普通用户可能没装
- 曾考虑:内置 uv+bun 运行时 / 按需下载 / 环境检测按钮
- **最终放弃所有方案**:weft 本身是 AI agent,MCP 启动失败时,AI 看到报错
  自然会告诉用户"需要先装 Node.js"并给出指引。不需要专门的 UI 机制。
- 内置运行时会增加 ~70-120MB 体积,只服务小众场景,不划算

**结论:** 非必要不做。环境问题由 AI 对话解决。

---

## 当前 MCP 添加流程(v1 已实现)

```
点击 + →
  ┌─ 粘贴配置 JSON   (兼容 Claude/Cursor/Smithery,带解析预览+安全提示)
  ├─ 远程 MCP (URL)  (零依赖)
  └─ 手动添加        (自填 command/args)
```

---

## 未来方向(未实现,按需再做)

按性价比排序,**均非当前必需**:

1. **应用内浏览 Smithery/registry** — Smithery 有公开 API,可做搜索+一键装。
   但粘贴导入已覆盖(Smithery 每个 server 都有复制按钮),优先级低。

2. **skill 增强层** — 当出现"agent 用工具用得不好"的真实反馈时,
   再考虑给特定 MCP 配领域提示词。用 MCP server 原生的 `instructions` 字段
   优于外挂 skill。

3. **运行时按需下载** — 若未来目标用户明确是零环境小白,再考虑
   按需下载 uv/bun。当前交给 AI 提示即可。

---

## 反面教训

- 不要为小众场景给所有用户增加体积/复杂度
- 不要重复造协议已经解决的东西(tool schema 已含用法说明)
- weft 是 AI agent,很多"配置引导"类问题可以由 AI 对话解决,不必做成 UI
