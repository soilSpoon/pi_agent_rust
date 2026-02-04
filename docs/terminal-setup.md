# Terminal Setup

Pi works in any modern terminal, but some features (like image display) require specific support.

## Recommended Terminals

- **Ghostty**: Excellent image support (Kitty protocol) and performance.
- **WezTerm**: Good image support (iTerm protocol).
- **iTerm2**: Good image support (iTerm protocol).
- **Kitty**: Good image support (Kitty protocol).
- **Windows Terminal**: Good Unicode support, but no inline images yet.

## Image Support

Pi detects terminal capabilities to display images inline (e.g. when using the `read` tool on an image file).

If your terminal does not support images, Pi will display a placeholder like `[Image: 1024x768 placeholder]`.

You can force disable images in `settings.json`:

```json
{
  "terminal": {
    "showImages": false
  }
}
```

## Keybindings

Some terminals intercept key combinations needed by Pi (e.g., `Ctrl+Arrow`, `Shift+Enter`).

- **Windows Terminal**: Use `Ctrl+Enter` for newlines instead of `Shift+Enter`.
- **VS Code Terminal**: Some shortcuts may be captured by VS Code. Check your `terminal.integrated.commandsToSkipShell` setting.
