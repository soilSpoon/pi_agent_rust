# agentsbox reference

## Tool contract

### agentsbox_search_bm25
- Purpose: natural language search over the tool catalog
- Input:
  - `text: string`
  - `limit?: number`
- Output: JSON string with `tools[]` including `name`, `description`, `score`, `schema`

### agentsbox_search_regex
- Purpose: regex-based search over tool names/signatures
- Input:
  - `pattern: string`
  - `limit?: number`

### agentsbox_execute
- Purpose: execute a discovered tool
- Input:
  - `toolId: string` (`{serverName}_{toolName}`)
  - `arguments?: string` (JSON)

### agentsbox_status
- Purpose: connection + catalog health

### agentsbox_perf
- Purpose: performance metrics

### agentsbox_test
- Purpose: run a minimal test across discovered tools

## Suggested troubleshooting sequence

```
agentsbox_search_regex(".*")
  -> agentsbox_search_bm25("describe what you need")
    -> agentsbox_status({})
      -> ask user to confirm servers/credentials
```
