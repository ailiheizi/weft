# Agent Core Plugin

AI Agent instance management and LLM dialog engine - Core package for WEFT.

## Features

- **Agent Management**: Create, configure, and manage AI agent instances
- **LLM Integration**: Built-in support for multiple LLM providers (DeepSeek, OpenAI, Anthropic, etc.)
- **Conversation History**: Persistent chat history with automatic memory management
- **Tool Integration**: Execute skills and tools through the skills plugin
- **Memory System**: Automatic recall of relevant memories from the memory plugin
- **Channel Integration**: Receive messages from connected channels (Slack, Discord, etc.)

## Configuration

Each agent has the following configuration:

```rust
struct AgentConfig {
    name: String,           // Unique agent identifier
    label: String,          // Display name
    role: String,           // Agent role/purpose
    model: String,          // LLM model (default: "deepseek-chat")
    temperature: f64,       // Temperature (default: 0.7)
    system_prompt: String,  // System prompt
    skills: Vec<String>,    // Enabled skills
    channels: Vec<Value>,   // Connected channels
}
```

## WebSocket API

Connect to `ws://localhost:3004/ws/plugins/agent-core`

### Actions

#### Get Agents
```json
{
  "action": "get_agents",
  "data": {}
}
```

#### Create Agent
```json
{
  "action": "create_agent",
  "data": {
    "name": "assistant",
    "label": "My Assistant",
    "role": "General purpose assistant",
    "model": "deepseek-chat",
    "temperature": 0.7,
    "system_prompt": "You are a helpful assistant.",
    "skills": ["web_search", "calculator"],
    "channels": []
  }
}
```

#### Delete Agent
```json
{
  "action": "delete_agent",
  "data": {
    "name": "assistant"
  }
}
```

#### Send Message
```json
{
  "action": "send_message",
  "data": {
    "agent": "assistant",
    "content": "Hello, how are you?"
  }
}
```

#### Get History
```json
{
  "action": "get_history",
  "data": {
    "agent": "assistant"
  }
}
```

#### Clear History
```json
{
  "action": "clear_history",
  "data": {
    "agent": "assistant"
  }
}
```

## Use Cases

- **Personal AI Assistant**: Create agents with different personalities and expertise
- **Customer Support**: Deploy specialized agents for different support topics
- **Content Generation**: Configure agents for writing, coding, or creative tasks
- **Research Assistant**: Set up agents with access to search and analysis tools
- **Multi-Agent Systems**: Coordinate multiple agents for complex workflows

## Integration

This package integrates with:
- **memory**: Recalls relevant memories during conversations
- **skills**: Executes tools and skills requested by the LLM
- **channels**: Receives messages from external platforms
- **mcp-client**: Provides MCP tools to agents

## Building

```bash
cargo build --release --target wasm32-wasip1
```

## License

MIT
