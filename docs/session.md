# Sessions

Pi stores conversation history in session files.

## File Format

Sessions are stored as JSONL (JSON Lines) files.

### Location

Sessions are grouped by project directory:
`~/.pi/agent/sessions/--encoded-project-path--/`

Filename format: `YYYY-MM-DDTHH-MM-SS.sssZ_id.jsonl`

### Structure

1. **Header**: The first line is always a `SessionHeader` object containing metadata (ID, timestamp, CWD, initial settings).
2. **Entries**: Subsequent lines are `SessionEntry` objects representing events in the conversation.

### Entry Types

- `message`: User or Assistant message.
- `model_change`: User switched models.
- `thinking_level_change`: User changed thinking settings.
- `compaction`: Context was summarized to save tokens.
- `branch_summary`: A summary of a branch point (when forking).
- `session_info`: Updates like session renaming.

## Tree Structure

Pi supports conversation branching. Each entry has an `id` and an optional `parent_id`.

- **Linear Conversation**: A -> B -> C
- **Branching**:
  ```
  A -> B -> C
       \ -> D
  ```

When you navigate to a previous message and reply, Pi creates a new branch.

## Management

### Resume (`/resume`, `pi -r`)

Opens the session picker to switch between sessions.
- **Select**: Enter
- **Delete**: Ctrl+D (requires confirmation)

### Tree Navigator (`/tree`)

Visualizes the branching structure of the current session.
- **Navigate**: Up/Down
- **Switch**: Enter (switches active context to the selected node)

### Forking (`/fork`)

Creates a **new session file** starting from the current point (or a selected point). This is useful when you want to explore a significantly different direction without cluttering the current session file.

### Compaction (`/compact`)

Manually triggers context compaction. Pi also compacts automatically based on the `compaction` settings in `settings.json`.