# 工具与技能执行管线

## 完整工具清单

### 内置工具 (skills-runtime WASM)

| 工具名 | 实现函数 | 用途 |
|--------|---------|------|
| `fs_read` | do_fs_read | 读取文件 |
| `fs_write` | do_fs_write | 写入文件 |
| `fs_list` | do_fs_list | 列出目录 |
| `shell_exec` | do_shell_exec | 执行命令 |
| `web_fetch` | do_web_fetch | HTTP 请求 |
| `web_search` | do_web_search | 网络搜索 |

### MCP 工具 (动态发现)

- 来源: MCP 服务器通过 JSON-RPC 2.0 `tools/list`
- 命名: `mcp::<server_name>::<tool_name>`
- 缓存: KV store (`mcp:tools:{agent}:{server}`)
- 传输: stdio (子进程) 或 HTTP (JSON-RPC HTTP)

### 虚拟能力工具 (agent-core 注入)

| 工具名 | 用途 |
|--------|------|
| `render_prompt` | Prompt 渲染 |
| `delegate_to_workflow` | 工作流委托 |
| `list_mcp_servers` | 列出 MCP 服务器 |
| `add_mcp_server` | 添加 MCP 服务器 |
| `remove_mcp_server` | 移除 MCP 服务器 |
| `send_channel_message` | 发送通道消息 |
| `read_channel_inbox` | 读取通道收件箱 |

### JS Extension Runtime 工具

| 工具名 | 实现位置 | 特点 |
|--------|---------|------|
| `web_search` | `js-extension-runtime/server.js` | 9 提供者 fallback 链 |
| `fetch_url` | `js-extension-runtime/server.js` | HTTP 请求 |

### Evolved Skills (自动生成的技能)

- 来源: 成功的 agent trajectory 自动 crystallize
- 生命周期: `active → pending_review → promoted → archived`
- 包含: 过程步骤、验证步骤、工具需求、置信度评分

## 工具执行调度树

```
execute_tool(tool_name, args)
  │
  ├─ tool_name 是内置工具?
  │   ├─ "fs_read"     → do_fs_read(args)
  │   ├─ "fs_write"    → do_fs_write(args)
  │   ├─ "fs_list"     → do_fs_list(args)
  │   ├─ "shell_exec"  → do_shell_exec(args)
  │   ├─ "web_fetch"   → do_web_fetch(args)
  │   └─ "web_search"  → do_web_search(args)
  │                         ↓ 9 引擎 fallback (DuckDuckGo → Google → Bing → ...)
  │
  ├─ tool_name 匹配 "mcp::"?
  │   └─ call_plugin("mcp-client", "call_tool", {server, tool, args})
  │       ↓ JSON-RPC 2.0 tools/call
  │       ↓ stdio 子进程 或 HTTP POST
  │
  └─ 都不是?
      ├─ 查 external_tool_specs (MCP 工具)
      │   └─ 找到匹配 → mcp-client.call_tool
      │
      └─ 没找到 → 错误
```

## JS Runtime 优先路径

前端对 `web_search/search_web/fetch_url/web_fetch` 有特殊处理：

```ts
executeTool(toolName, args) {
    if (shouldExecuteToolWithJsFirst(normalizedToolName)) {
        try {
            const jsResponse = executeJsExtensionRuntime({
                action: "execute_tool",
                payload: { tool: normalizedToolName, args }
            })
            if (jsResponse.ok) return jsResponse.result
        } catch {
            // Fall back to Rust/WASM skills-runtime
        }
    }
    return callPlugin("skills", "execute_tool", { tool, args })
}
```

JS Runtime `web_search` 的 9 引擎 fallback 链：`DuckDuckGo → Google → Bing → Yahoo → Brave → Mojeek → MetaGer → SearXNG → Qwant`

## Companion-Core 规划逻辑

### 两阶段 LLM 流水线

**阶段 1: Companion LLM (轻量规划)**

```
用户消息 + 历史 + 会话上下文
  ↓
Companion LLM (简单 prompt)
  ↓ 判断 kind
  ├─ "direct" → 直接返回伴侣文本 (不调用工具)
  └─ "delegate" → 委托给 agent (reason: 任务描述)
```

**阶段 2: Work Agent LLM (实际执行)**

```
系统 prompt: "You are a session work agent. Use tools, search, files, and delegation..."
  ↓
"Delegate reason: {reason}"
"Latest user query: {query}"
  ↓ LLM 结构化输出 (JSON)
  {
    "action": "call_tool",
    "tool": "web_search",
    "args": {"query": "..."}
  }
  ↓ 执行工具 (最多 3 轮)
  ↓
  {
    "action": "respond",
    "content": "搜索结果显示..."
  }
```

### turn prompt 关键规则

```
kind rules:
- direct: 不需要工具或后台工作
- delegate: 必须交给 session work agent 处理

Field rules:
- For director, voice/subtitle = 自然回答
- For delegate, voice/subtitle = 简短前言
- For delegate, reason = 已选择的实际任务路由
```

## Agent-Core 调度

### 完整循环

```
1. 接收 delegate 请求
2. 检索 applicable evolved skills (基于触发词匹配)
3. 组装 tool specs (builtin + MCP + virtual)
4. 构建 LLM prompt (系统 + 工具定义 + 技能 + 上下文)
5. LLM 返回 action 计划 (tool_calls 或 respond)
6. 执行工具 → skills.execute_tool() / mcp-client.call_tool()
7. 工具结果追加到对话历史
8. 最多 3 轮工具调用循环
9. 记录 trajectory (成功路径)
10. 如果成功且有新轨迹 → 触发 skill evolution (crystallize)
```

## Evolved Skills 系统

### 生命周期

```
active → pending_review → promoted → archived
  ↑                                  ↓
  └──── 如果质量下降 ────────────────┘
```

### 技能结构

```rust
struct EvolvedSkill {
    id: String,
    title: String,
    description: String,
    triggers: Vec<String>,       // 触发词 (如 "search", "look up")
    anti_triggers: Vec<String>,  // 反触发词
    required_tools: Vec<String>, // 需要的工具
    procedure: String,           // 过程描述
    steps: Vec<SkillStep>,       // 详细步骤
    verification: String,        // 验证方法
    confidence: f64,             // 置信度
    quality_score: f64,          // 质量分数
    usage_count: u32,            // 使用次数
    source_trajectory_id: String // 来源轨迹
}
```

### 检索策略
- 基于触发词匹配 (用户查询 vs skills.triggers)
- 反触发词排除
- 按 quality_score 排序
- 限制返回数量

## MCP 集成架构

### 服务器生命周期

```
add_server → start_server → (connected) → stop_server → remove_server
                ↓
           get_tools (tools/list JSON-RPC)
                ↓
           call_tool (tools/call JSON-RPC)
```

### 传输机制

**stdio 传输**:
1. Core 启动临时子进程 (`command` + `args`)
2. 发送 `initialize` JSON-RPC 请求
3. 发送 `notifications/initialized`
4. 发送 `tools/list` → 获取工具列表
5. 工具调用时: 发送 `tools/call` → 等待响应
6. 子进程持续运行直到 stop_server

**HTTP/SSE 传输**:
1. POST JSON-RPC 请求到 `server.url`
2. 携带 `server.headers`
3. 解析 JSON-RPC 响应

### 工具缓存
- 工具列表缓存在 KV store: `mcp:tools:{agent}:{server}`
- 服务器信息缓存在: `mcp:servers:{agent}`

## companion-core 完整的 delegate 流程

```python
def execute_delegate_turn(input_payload, content, session_context, plan):
    # 1. 确保 agent session 存在
    ensure_delegate_session(
        agent_runtime_plugin,
        session_id,
        delegate_request={"reason": plan["reason"], "query": content}
    )
    # 2. 构建 agent 系统 prompt
    system_prompt = build_agent_system_prompt({
        "reason": plan["reason"],
        "latest_user_query": content
    })
    # 3. 调用 agent completion (结构化 JSON 输出)
    assistant_reply, tool_results = generate_agent_completion_structured(
        provider, model, system_prompt, content, session_context
    )
    # 4. 最多 3 轮工具调用
    for round in range(3):
        if action == "respond": break
        result = execute_tool(tool_name, tool_args)
        tool_results.append(result)
        # 继续下一轮 LLM 补全

    # 5. 返回结果 (包含 tool_results + final_reply)
    return {...}
```
