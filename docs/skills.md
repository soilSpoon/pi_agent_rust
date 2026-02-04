# Skills

Skills provide specialized instructions and capabilities to the agent. They are defined using the [Agent Skills](https://github.com/Start-Agent-Skills) format.

## Locations

Skills are loaded from:

1. **Global**: `~/.pi/agent/skills/*/SKILL.md` or `~/.pi/agent/skills/*.md`
2. **Project**: `.pi/skills/*/SKILL.md` or `.pi/skills/*.md`
3. **Packages**: Installed packages.

## File Format

A skill is defined in a `SKILL.md` file with YAML frontmatter.

```markdown
---
name: "sql-expert"
description: "Expert at writing and optimizing SQL queries"
disable-model-invocation: false
---
You are an expert SQL developer. When writing queries:
1. Always prefer CTEs over subqueries.
2. Use uppercase for keywords.
3. Check for index usage.
```

### Frontmatter Fields

| Field | Description |
|-------|-------------|
| `name` | Skill ID (must match directory name if in a subdir) |
| `description` | Short description used for selection |
| `disable-model-invocation` | If `true`, the skill is not shown to the model in the system prompt. |
| `allowed-tools` | List of tools this skill is allowed to use (optional enforcement) |

## Usage

### Auto-Discovery

By default, Pi includes all enabled skills in the system prompt. The model can decide to "activate" a skill by reading its definition file using the `read` tool.

### Explicit Invocation

You can explicitly invoke a skill using the slash command:

```bash
/skill:sql-expert "Optimize this query..."
```

This effectively wraps your prompt with the skill's instructions.

## Configuration

To disable the `/skill:` slash commands, set `enableSkillCommands` to `false` in `settings.json`.
