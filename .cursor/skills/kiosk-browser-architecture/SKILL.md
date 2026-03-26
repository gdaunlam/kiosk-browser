---
name: kiosk-browser-architecture
description: Architecture reference for the kiosk-browser Tauri v2 project. Covers module structure, platform-specific patterns, design decisions, and known issues. Use when making structural changes, adding platform support, debugging platform-specific behavior, or reviewing architectural decisions.
---

# Kiosk Browser — Architecture

## Stack

| Layer | Technology |
|-------|------------|
| App framework | Tauri v2 (Rust backend, OS webview) |
| Backend | Rust 2021 edition |
| Frontend | Static HTML/CSS (no bundler, no JS framework) |
| CLI | clap 4 (derive) |
| Logging | log + env_logger |
| Windows hooks | windows crate 0.58 (Win32 `WH_KEYBOARD_LL`) |
| Linux grabs (primary) | evdev 0.13 (`EVIOCGRAB` + `uinput` virtual device) |
| Linux grabs (fallback) | x11 crate 2.21 (`XGrabKey` on root window) |
| Linux TLS | webkit2gtk 2.0 (direct access to WebKit context) |
| URLs | url 2 |
| Paths | dirs 6 |

## Module map

```
src-tauri/src/
├── main.rs            # Binary entry, calls lib::run()
├── lib.rs             # Tauri setup, window creation, navigation, downloads, close handler
├── cli.rs             # clap-derived CLI args (--url, --fullscreen, --block-keys, --block-keys-preset)
└── keyboard/
    ├── mod.rs          # start_guard() dispatcher: evdev → X11 fallback, Wayland detection
    ├── keys.rs         # BlockableKey enum, FromStr, resolve_blocked_keys()
    ├── windows.rs      # WH_KEYBOARD_LL hook + GetMessageW pump
    ├── linux_evdev.rs  # evdev EVIOCGRAB + uinput virtual keyboard filter (primary on Linux)
    └── linux_x11.rs    # XGrabKey on root window + XNextEvent drain loop (fallback)
```

## Key design decisions

### 1. Window creation in Rust, not JSON

`tauri.conf.json` has `"windows": []`. The window is built entirely via `WebviewWindowBuilder` in `lib.rs`. This allows runtime configuration from CLI args (URL, fullscreen) and programmatic access to `on_navigation`, `on_download`, `initialization_script`.

### 2. Local page first, then navigate

The webview loads `index.html` (splash) before navigating to the target URL. On Linux this is **required** — `with_webview` only works after the WebKit view is fully initialized, and we need to set `TLSErrorsPolicy::Ignore` on the actual `WebContext` before loading remote content.

### 3. Close button via custom protocol, not IPC

The close button is injected via `initialization_script` (CLOSE_BUTTON_JS constant). When clicked, it navigates to `kiosk://close`. The `on_navigation` handler intercepts this URL and calls `std::process::exit(0)`. This avoids depending on `window.__TAURI__` or `withGlobalTauri`, which don't reliably work on remote URLs.

### 4. TLS certificate bypass (Linux)

`webkit2gtk::TLSErrorsPolicy::Ignore` is set on the webview's own `WebContext` (obtained via `wv.inner().context()`), **not** on `WebContext::default()` which is a different instance. This is a deliberate trade-off for internal networks with self-signed certs.

On Windows, WebView2 handles TLS errors through its own mechanisms (expected to show a warning page).

### 5. Keyboard guard — platform dispatch

`keyboard/mod.rs` checks the OS at compile time:
- **Windows**: Spawns a thread with `SetWindowsHookExW(WH_KEYBOARD_LL)` + `GetMessageW` message pump. Blocks keys by returning `LRESULT(1)` without calling `CallNextHookEx`.
- **Linux**: Multi-layer approach, all layers run simultaneously:
  - **Layer 0 (Tauri)**: `on_window_event` in `lib.rs` intercepts `CloseRequested` and calls `prevent_close()` when AltF4 is blocked. Last line of defense on both OS.
  - **Layer 1 (WM config)**: `try_disable_wm_shortcuts()` in `mod.rs` disables WM shortcuts before grabbing keys. KDE 6 uses D-Bus `disableGlobalShortcuts`. KDE 5 modifies `kglobalshortcutsrc` + `kwinrc` via `kwriteconfig5` and triggers `KWin.reconfigure`. GNOME uses `gsettings`. XFCE uses `xfconf-query`. Handles `sudo` by running `kwriteconfig` as the original user.
  - **Layer 2 (X11 grabs)**: `XGrabKey` on root window — catches VNC/xrdp injected events that bypass `/dev/input`.
  - **Layer 3 (evdev)**: `EVIOCGRAB` on `/dev/input/eventN` + `uinput` filter — effective on bare-metal, bypasses WM entirely. Requires root or `input` group.
- **Other**: Logs a warning, no blocking.

### 6. Key blocking granularity

`BlockableKey` enum in `keys.rs` defines individual shortcuts (Win, Alt+Tab, Alt+F4, etc.). The `kiosk` preset blocks all of them. Users can also specify a comma-separated list via `--block-keys`.

For Super/Win key on Linux X11, `AnyModifier` is used so all Super+X combos are captured, not just bare Super.

## Known limitations and issues

### Keyboard capture on Linux

- **Multi-layer approach**: Layer 0 (Tauri prevent_close) + Layer 1 (WM shortcut disabling) + Layer 2 (X11 XGrabKey) + Layer 3 (evdev EVIOCGRAB). All layers run simultaneously.
- **VNC/container environments**: evdev grabs `/dev/input` devices but VNC keyboard events arrive via the display protocol, bypassing `/dev/input`. Layers 0–2 handle these cases.
- **KDE Plasma 5**: Does NOT support `disableGlobalShortcuts` D-Bus method (Plasma 6 only). Config files are modified via `kwriteconfig5` + `reconfigure` instead.
- **Permissions**: evdev requires root or `input` group membership. WM config changes work as the current user.
- **Wayland without evdev**: Keys will NOT be blocked. Recommendation: use a kiosk compositor like `cage` or grant `input` group access.
- **Config persistence**: WM shortcut changes (KDE, GNOME) persist until manually restored. This is intentional for kiosk deployments.
- **Graceful ungrab**: The evdev filter loop runs until process exit or device error. On `std::process::exit(0)`, the kernel automatically releases the `EVIOCGRAB` when the fd is closed.

### Keyboard capture on Windows

- **Win+L**: Partially blockable (kernel/security policy may override).
- **Ctrl+Alt+Del**: Not blockable (Windows Secure Attention Sequence, kernel-level).

### Unused dependencies

`serde` and `serde_json` are declared in `Cargo.toml` but not used directly in application code. Can be removed (Tauri pulls them transitively).

### README inconsistency

`README.linux.md` has a "Build to Windows" section with `cargo build --target x86_64-pc-windows-msvc` but later states cross-compilation is not practical. The cross-compile snippet should be removed or clarified.

## What works (verified)

- Borderless/fullscreen webview loading remote URL
- Close button overlay with hover reveal and `kiosk://close` navigation
- TLS certificate bypass on Linux via webkit2gtk
- File downloads to Downloads directory with filename extraction
- CLI argument parsing with presets
- GitHub Actions CI for Windows + Linux
- Keyboard capture on Linux via evdev (bypasses WM grabs, works on X11 and Wayland)
- Multi-layer keyboard guard: Tauri prevent_close + WM shortcut disabling (KDE 5/6, GNOME, XFCE) + X11 grabs + evdev

## What needs work

- See `TODO.md` for current task list.

## Adding a new platform

1. Create `src-tauri/src/keyboard/<platform>.rs` with `pub fn install_hook(keys: HashSet<BlockableKey>)`.
2. Add `#[cfg(target_os = "<platform>")]` branch in `keyboard/mod.rs`.
3. Add platform-specific dependencies under `[target.'cfg(target_os = "<platform>")'.dependencies]` in `Cargo.toml`.
4. Create `README.<platform>.md` with build instructions.
5. Add a job to `.github/workflows/build.yml`.
6. Update this skill document.
