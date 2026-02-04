# Interactive Interface (TUI)

Pi's interactive mode provides a rich terminal user interface.

## Layout

### Header
Displays the current model and version. Can be hidden with `quietStartup` setting.

### Conversation View
The main area shows the conversation history.
- **User messages**: Highlighted in accent color.
- **Assistant messages**: Rendered as Markdown.
- **Thinking blocks**: Muted and italicized.
- **Tool calls/results**: Structured blocks showing tool execution.

### Editor
The input area at the bottom.
- Supports multi-line editing.
- Syntax highlighting for code blocks.
- Autocomplete dropdown.

### Footer
Displays session statistics and status.
- Token usage (input/output) and estimated cost.
- Editor mode hints (Single-line vs Multi-line).
- Current status messages.

## Display Controls

| Action | Shortcut | Description |
|--------|----------|-------------|
| **Toggle Thinking** | `Ctrl+T` | Hide/show thinking blocks to reduce noise. |
| **Expand Tools** | `Ctrl+O` | Collapse/expand detailed tool output. |
| **Scroll History** | `PageUp` / `PageDown` | Scroll conversation view. |

## Navigation & Overlays

### Model Selector (`Ctrl+L`)
Opens a fuzzy-searchable list of available models. Select a model to switch instantly.

### Session Picker (`/resume`)
Browse and resume previous sessions without restarting Pi.
- `Enter`: Select session
- `Delete`: Delete session (with confirmation)

### Tree Navigator (`/tree`)
Visualize the conversation branching structure.
- `Up` / `Down`: Navigate nodes
- `Enter`: Switch to selected node (forks if not a leaf)
- `Ctrl+U`: Toggle user-only view (hides assistant/tool noise)

### Settings (`/settings`)
Change configuration on the fly (Thinking levels, themes, message delivery mode).

## Message Queue

When Pi is busy generating a response or running tools, you can still type.

- **Queue Steering (`Enter`)**: Interrupts the agent after the current step and injects your message.
- **Queue Follow-up (`Alt+Enter`)**: Adds your message to the end of the queue, to be processed after the agent finishes its current task.

The queue is visible above the editor when not empty.
