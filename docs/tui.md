# Interactive Interface (TUI)

Pi's interactive mode provides a full-screen terminal UI for chatting, streaming
responses, and managing sessions.

## Layout

### Header
Shows high-level session context (current model, status, hints). Exact contents
may vary as the UI evolves.

### Conversation View
The main area shows the conversation history.
- **User messages**: Highlighted in accent color.
- **Assistant messages**: Rendered as Markdown.
- **Thinking blocks**: Muted and italicized.
- **Tool calls/results**: Structured blocks showing tool execution and output.

### Editor
The input area at the bottom.
- **Single-line + multi-line editing** (see shortcuts below).
- **Autocomplete** for `@file` references, `/commands`, and resource names.
- Paste and editing behaviors follow the configured keybindings.

### Footer
Displays session statistics and status.
- Token usage (input/output) and estimated cost.
- Editor mode hints (Single-line vs Multi-line).
- Current status messages.

## Display Controls

| Action | Shortcut | Description |
|--------|----------|-------------|
| **Toggle Thinking** | `Ctrl+T` | Hide/show thinking blocks to reduce noise. |
| **Scroll History** | `PageUp` / `PageDown` | Scroll conversation view. |

## Navigation & Overlays

### Keyboard shortcuts (`/hotkeys`)
Use `/hotkeys` to see the current shortcut list (including any user overrides
from `~/.pi/agent/keybindings.json`).

### Model selection
- Use `/model` to switch models (by `provider/id` or fuzzy match).
- Some builds also define shortcuts like `Ctrl+L` (model selector) and `Ctrl+P`
  (cycle models). If a shortcut appears in `/hotkeys` but does nothing, it
  hasn’t been wired in that build yet.

### Session Picker (`/resume`)
Browse and resume previous sessions without restarting Pi.
- `Enter`: Select session
- `Ctrl+D`: Delete session (with confirmation)

### Tree Navigator (`/tree`)
Visualize the conversation branching structure.
- `Up` / `Down`: Navigate nodes
- `Enter`: Switch to selected node (forks if not a leaf)
- `Ctrl+U`: Toggle user-only view (hides assistant/tool noise)

### Settings (`/settings`)
Change configuration on the fly (Thinking levels, themes, message delivery mode).

## Message Queue

When Pi is busy generating a response or running tools, you can still type.

- **Queue Steering (`Enter`)**: Sends your message as a steering interrupt after
  the current step completes.
- **Queue Follow-up (`Alt+Enter`)**: Adds your message to the follow-up queue to
  be processed when the agent becomes idle.
- **Restore queued messages (`Alt+Up`)**: Pull queued messages back into the
  editor (useful if you queued something by mistake).

The queue is visible above the editor when not empty.
