# Android (Termux) Notes

Pi can run on Android via [Termux](https://termux.dev/).

## Clipboard

To use clipboard features (`/copy`, pasting images), you must install the Termux API package.

1. Install the API app from the Play Store or F-Droid.
2. Install the package in Termux:
   ```bash
   pkg install termux-api
   ```

Pi detects the `termux-clipboard-get` and `termux-clipboard-set` commands if standard clipboard access fails.

## Terminal

- If arrow keys or shortcuts behave unexpectedly, check your Termux keyboard extra keys row configuration.
- `Ctrl` key modifiers usually work as expected.

## Storage

- Pi stores sessions in `~/.pi/agent/sessions`.
- Ensure you have granted storage permissions if you intend to access files outside the Termux private storage.
