# Skills Plugin

Skill and tool management with MCP integration - Register and execute agent skills.

## Features

- **Skill Registration**: Register custom skills with JSON Schema definitions
- **Tool Execution**: Execute skills with validated parameters
- **MCP Integration**: Automatically discover and register MCP tools
- **Agent Permissions**: Control which agents can use which skills

## Configuration

Skills are defined using JSON Schema:

```json
{
  "name": "web_search",
  "description": "Search the web for information",
  "parameters": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "Search query"
      },
      "limit": {
        "type": "integer",
        "description": "Maximum number of results",
        "default": 5
      }
    },
    "required": ["query"]
  }
}
```

## WebSocket API

Connect to `ws://localhost:17830/ws/plugins/skills`

### Actions

#### Register Skill
```json
{
  "action": "register_skill",
  "data": {
    "name": "calculator",
    "description": "Perform mathematical calculations",
    "parameters": {
      "type": "object",
      "properties": {
        "expression": {
          "type": "string",
          "description": "Mathematical expression to evaluate"
        }
      },
      "required": ["expression"]
    },
    "handler": "builtin:calculator"
  }
}
```

#### Enable Skill for Agent
```json
{
  "action": "enable_for_agent",
  "data": {
    "agent": "assistant",
    "skill": "web_search"
  }
}
```

#### Get Tool Specs
```json
{
  "action": "get_tool_specs",
  "data": {
    "agent": "assistant"
  }
}
```

Returns OpenAI-compatible tool definitions for all enabled skills.

#### Execute Tool
```json
{
  "action": "execute_tool",
  "data": {
    "agent": "assistant",
    "tool": "web_search",
    "args": {
      "query": "WASM performance",
      "limit": 3
    }
  }
}
```

#### List Skills
```json
{
  "action": "list_skills",
  "data": {}
}
```

## Built-in Skills

### Calculator
Evaluate mathematical expressions:
```json
{
  "tool": "calculator",
  "args": {
    "expression": "2 + 2 * 3"
  }
}
```

### Web Search
Search the web (requires MCP integration):
```json
{
  "tool": "web_search",
  "args": {
    "query": "Rust WASM tutorial",
    "limit": 5
  }
}
```

## Use Cases

### Custom Tools
Create domain-specific tools for your agents:
```rust
// Register a custom skill
register_skill("database_query", schema, handler);
```

### MCP Integration
Automatically expose MCP tools to agents:
```rust
// MCP tools are automatically discovered and registered
// Agents can use them through the skills plugin
```

### Permission Control
Control which agents can use which skills:
```rust
// Only allow specific agents to use sensitive skills
enable_for_agent("admin-agent", "database_query");
```

## Integration

This package integrates with:
- **agent-core**: Provides tools for LLM function calling
- **mcp-client**: Discovers and registers MCP tools as skills

## Building

```bash
cargo build --release --target wasm32-wasip1
```

## License

MIT
