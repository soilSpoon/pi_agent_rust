# Tree Navigation

Pi sessions are trees, not just flat lists. You can branch the conversation at any point.

## Concepts

- **Branching**: Occurs when you reply to a message that is not the current leaf.
- **Tree Navigator**: The `/tree` command helps you visualize and traverse these branches.

## Usage

Run `/tree` to open the navigator.

```bash
/tree
```

### Controls

- **Up/Down**: Move selection.
- **Enter**: Switch to the selected node.
- **Escape**: Cancel.
- **Ctrl+U**: Toggle "User Only" mode (hides assistant messages and tool calls for a cleaner view).
- **Ctrl+O**: Toggle "Show All" (includes hidden/system entries).

### Selection Behavior

- **Selecting a Leaf**: Switches context to that point in the conversation.
- **Selecting a Non-Leaf**: Switches context to that point. If you then type a message, a new branch is created.
- **Selecting a User Message**: Pre-fills the editor with that message text, allowing you to edit and resubmit (creating a branch).

## Forking vs Tree Switching

- **Tree Switching** (`/tree`): Stays in the **same session file**. Useful for quick experiments or retries.
- **Forking** (`/fork`): Creates a **new session file** starting from the current branch. Useful when a tangent becomes a substantial conversation of its own.

## Branch Summarization

When you switch branches, Pi may offer to summarize the path you are leaving. This summary is stored as a `branch_summary` entry, helping the model understand context if you ever switch back or reference that branch.
