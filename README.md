# kiosk-browser

A kiosk browser built with Tauri v2 that loads a remote URL and captures low-level
keyboard shortcuts (Win, Alt+Tab, etc.) to prevent users from escaping the kiosk.

See platform-specific setup and build instructions:
- [README.windows.md](README.windows.md)
- [README.linux.md](README.linux.md)

## Features

- Loads any URL in a chromium-based webview (WebView2 on Windows, WebKitGTK on Linux)
- Blocks configurable system keyboard shortcuts at the OS level
- Full browser capabilities: file upload, file download, clipboard
- Fullscreen mode
- Cross-platform: Windows and Linux (X11)

## Usage

```
kiosk-browser [OPTIONS] --url <URL>

Options:
      --url <URL>
          URL to load in the kiosk browser

      --fullscreen
          Start in fullscreen mode

      --block-keys <KEY,KEY,...>
          Comma-separated list of keys to block (see table below)

      --block-keys-preset <PRESET>
          Use a preset: "kiosk" (block all) or "none" (block nothing)

  -h, --help
          Print help
```

### Examples

Block everything (kiosk mode):

```bash
kiosk-browser --url https://tailscale-services:38999/ --fullscreen --block-keys-preset kiosk
```

Block only specific keys:

```bash
kiosk-browser --url https://my-app.local/ --block-keys win,alt+tab,alt+f4
```

No key blocking (just a borderless fullscreen browser):

```bash
kiosk-browser --url https://dashboard.local/ --fullscreen
```

## Blockable keys

| Key               | CLI value   | Windows | Linux X11 |
|-------------------|-------------|---------|-----------|
| Windows / Super   | `win`       | Yes     | Yes       |
| Alt + Tab         | `alt+tab`   | Yes     | Yes       |
| Alt + F4          | `alt+f4`    | Yes     | Yes       |
| Alt + Escape      | `alt+esc`   | Yes     | Yes       |
| Ctrl + Escape     | `ctrl+esc`  | Yes     | Yes       |
| Win + Tab         | `win+tab`   | Yes     | Yes       |
| Win + D           | `win+d`     | Yes     | Yes       |
| Win + E           | `win+e`     | Yes     | Yes       |
| Win + R           | `win+r`     | Yes     | Yes       |
| Win + L           | `win+l`     | Partial | Yes       |

**Ctrl+Alt+Del** cannot be blocked on any platform (kernel-level).

## File downloads

Downloads are saved to the system Downloads directory by default.
Download activity is logged to stdout (visible with `RUST_LOG=info`).

## Architecture

```
kiosk-browser (Tauri v2)
├── Webview ─── loads external URL
│                 ├── file upload (native)
│                 ├── file download (on_download handler)
│                 └── clipboard (native)
│
├── Window Protection
│   └── on_window_event → prevent_close() (blocks Alt+F4 close)
│
├── Keyboard Guard (background thread)
│   ├── Windows: SetWindowsHookEx(WH_KEYBOARD_LL)
│   └── Linux:   WM shortcut disable + X11 grabs + evdev/uinput filter
│
└── CLI (clap)
    ├── --url
    ├── --fullscreen
    ├── --block-keys
    └── --block-keys-preset
```

## License

MIT
