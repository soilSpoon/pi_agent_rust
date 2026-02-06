# agentsbox Configuration

## Paths

| Item | Default |
|---|---|
| Config dir | `$XDG_CONFIG_HOME/agentsbox` (usually `~/.config/agentsbox`) |
| Config file | `$XDG_CONFIG_HOME/agentsbox/config.jsonc` |
| Schema file | `$XDG_CONFIG_HOME/agentsbox/agentsbox.schema.json` |

Override the config file path via:

```bash
export AGENTSBOX_CONFIG=/path/to/config.jsonc
```

## Scaffolding

Create a minimal/empty config + schema + bundled skill:

```bash
agentsbox init
```

## Config schema

The generated config references a **local** schema file:

```jsonc
{
  "$schema": "./agentsbox.schema.json",
  "mcp": { /* ... */ }
}
```

This keeps the config self-contained and avoids relying on hosted schemas (npm/unpkg).

## MCP servers

### Local servers

Runs an MCP server as a child process via stdio:

```jsonc
{
  "mcp": {
    "time": {
      "type": "local",
      "command": ["uvx", "mcp-server-time"],
      "environment": {
        "SOME_TOKEN": "{env:SOME_TOKEN}"
      }
    }
  }
}
```

### Remote servers

Connects to a remote MCP endpoint:

```jsonc
{
  "mcp": {
    "tavily": {
      "type": "remote",
      "url": "https://mcp.tavily.com/mcp/",
      "headers": {
        "Authorization": "Bearer {env:TAVILY_API_KEY}"
      }
    }
  }
}
```

## Settings

```jsonc
{
  "settings": {
    "defaultLimit": 5,
    "initMode": "eager", // eager | lazy
    "connection": {
      "connectTimeout": 5000,
      "requestTimeout": 30000,
      "retryAttempts": 2,
      "retryDelay": 1000
    }
  }
}
```
