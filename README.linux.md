# kiosk-browser — Linux

Setup, build, and platform-specific notes for Linux (X11).

## Prerequisites

### 1. Install Rust and build tools

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
sudo apt update
sudo apt install build-essential
```

Verify:

```bash
rustc --version
cargo --version
```

### 2. Install system libraries

Tauri needs WebKitGTK, GTK, and related libs. The keyboard guard needs libX11.

**Debian / Ubuntu:**

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev libx11-dev
```

**Fedora:**

```bash
sudo dnf install webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel \
  librsvg2-devel libX11-devel
```

**Arch:**

```bash
sudo pacman -S webkit2gtk-4.1 gtk3 libappindicator-gtk3 librsvg libx11
```

### 3. Install Tauri CLI

```bash
cargo install tauri-cli --version "^2"
```

## Build

```bash
cd src-tauri
cargo tauri build
```

## Build to windows

```bash
cd src-tauri
rustup target add x86_64-pc-windows-msvc
cargo build --target x86_64-pc-windows-msvc
```

The compiled binary will be in `src-tauri/target/release/kiosk-browser`.

Tauri also generates packages under `src-tauri/target/release/bundle/` (AppImage, .deb).

### Development mode

```bash
cd src-tauri
cargo tauri dev -- -- --url https://example.com --fullscreen --block-keys-preset kiosk
```

## Run

```bash
./kiosk-browser --url https://tailscale-mudafy:38999/ --fullscreen --block-keys-preset kiosk
```

## Keyboard guard details

The keyboard guard uses a multi-layer strategy. All layers run simultaneously
for maximum coverage across different environments (bare-metal, VNC, containers).

### Layer 0: Tauri window protection

When Alt+F4 is in the blocked keys, the app registers a `on_window_event`
handler that intercepts `CloseRequested` and calls `prevent_close()`. This is
the last line of defense — even if the WM or OS delivers the close signal, the
window stays open. Works on both Windows and Linux.

### Layer 1: Window Manager shortcut disabling

Before setting up key grabs, the app attempts to disable conflicting WM shortcuts:

- **KDE Plasma 6**: Calls `disableGlobalShortcuts` via D-Bus (disables all global shortcuts at runtime).
- **KDE Plasma 5**: Modifies `kglobalshortcutsrc` (Alt+F4, Alt+Tab, Meta+D, etc.) and `kwinrc` (Meta modifier-only shortcut) via `kwriteconfig5`, then triggers `KWin.reconfigure` via D-Bus. When running via `sudo`, executes `kwriteconfig5` as the original user so it writes to the correct config directory.
- **GNOME**: Disables shortcuts via `gsettings` (overlay-key, switch-applications, close, etc.).
- **XFCE**: Removes shortcut overrides via `xfconf-query`.

### Layer 2: X11 key grabs + event forwarding

`XGrabKey` on the X11 root window intercepts key events before the WM or
compositor sees them. Captured events are **forwarded to the webview** as
synthetic DOM `KeyboardEvent`s via `window.eval()`, so the web page can still
react to the blocked key combinations (e.g. Alt+F4 triggers an in-app close).

Layer 1 (WM shortcut disabling) runs first to release the WM's own passive
grabs — this prevents `BadAccess` errors when our grabs are installed.

### Layer 3: evdev backend

Grabs keyboard devices exclusively at the kernel input level (`/dev/input/eventN`)
using `EVIOCGRAB`. Blocked key events are discarded; all other events are
forwarded through a virtual keyboard created via `uinput`.

Bypasses the WM entirely. Effective on bare-metal machines. In VNC/container
environments, keyboard events arrive via the display protocol and bypass
`/dev/input`, so evdev alone is not sufficient (Layers 0–2 handle those cases).

**Requirements:**
- The process must run as **root**, or the user must belong to the **`input` group**:
  ```bash
  sudo usermod -aG input $USER
  # Log out and back in for the group change to take effect
  ```
- The kernel module `uinput` must be loaded (usually loaded by default):
  ```bash
  sudo modprobe uinput
  ```

**Works on both X11 and Wayland.**

### Limitations

- **Ctrl+Alt+Del** cannot be blocked (kernel-level on Linux).
- On Wayland without evdev permissions, keys **will not be blocked**. Use a
  kiosk compositor like [cage](https://github.com/cage-kiosk/cage) or grant
  `input` group access.
- WM shortcut disabling modifies user configuration files. The changes persist
  until manually restored (KDE, GNOME) or until the session is reset.

## Cross-compilation

Cross-compiling **from Linux to Windows** is not practical with Tauri.
Use GitHub Actions or build natively on each target OS.
