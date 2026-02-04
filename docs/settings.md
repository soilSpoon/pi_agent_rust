# Settings

Pi reads JSON settings and applies them with clear precedence rules.

## Locations

Pi loads settings from (up to) two files:

| Location | Scope |
|----------|-------|
| `~/.pi/agent/settings.json` | Global (all projects) |
| `.pi/settings.json` | Project (current directory) |

You can override the path entirely with `PI_CONFIG_PATH` (see below).

Run `pi config` to print the effective paths and precedence.

## Precedence (highest → lowest)

1. CLI flags
2. Environment variables
3. Project settings (`.pi/settings.json`)
4. Global settings (`~/.pi/agent/settings.json`)
5. Built-in defaults

## `PI_CONFIG_PATH` (single-file mode)

If `PI_CONFIG_PATH` is set, Pi loads *only* that file and skips the global/project merge.

## Merge behavior (global vs project)

Project settings override global settings on a per-field basis.

Important detail: nested objects like `compaction`, `retry`, `images`, `terminal`, `branch_summary`,
and `thinking_budgets` are treated as *single* fields. If `.pi/settings.json` contains a
`compaction` object, it replaces the entire global `compaction` object.

Within a single file, missing nested keys fall back to built-in defaults when accessed (see
`src/config.rs`).

Example:

```json
// ~/.pi/agent/settings.json (global)
{ "compaction": { "enabled": false, "reserve_tokens": 16384 } }
```

```json
// .pi/settings.json (project)
{ "compaction": { "reserve_tokens": 8192 } }
```

Resulting behavior:
- `compaction.reserve_tokens` becomes `8192`
- `compaction.enabled` does **not** inherit `false` from global; it falls back to its built-in default

## Common settings (snake_case JSON keys)

### Model selection

```json
{
  "default_provider": "anthropic",
  "default_model": "claude-sonnet-4-20250514",
  "default_thinking_level": "medium",
  "enabled_models": ["claude-*", "gpt-*"]
}
```

### Message delivery

`steering_mode` and `follow_up_mode` accept the legacy camelCase aliases `steeringMode` and
`followUpMode`.

```json
{
  "steering_mode": "one-at-a-time",
  "follow_up_mode": "one-at-a-time"
}
```

### Compaction (defaults)

`src/config.rs` accessor defaults:
- `compaction.enabled`: `true`
- `compaction.reserve_tokens`: `16384`
- `compaction.keep_recent_tokens`: `20000`

```json
{
  "compaction": {
    "enabled": true,
    "reserve_tokens": 16384,
    "keep_recent_tokens": 20000
  }
}
```

### Retry (defaults)

`src/config.rs` accessor defaults:
- `retry.enabled`: `true`
- `retry.max_retries`: `3`
- `retry.base_delay_ms`: `2000`
- `retry.max_delay_ms`: `60000`

```json
{
  "retry": {
    "enabled": true,
    "max_retries": 3,
    "base_delay_ms": 2000,
    "max_delay_ms": 60000
  }
}
```

### Images

```json
{
  "images": {
    "auto_resize": true,
    "block_images": false
  }
}
```

### Shell

```json
{
  "shell_path": "/bin/bash",
  "shell_command_prefix": "set -e"
}
```

## Full reference

`src/config.rs` is the authoritative list of supported fields and defaulting behavior.
