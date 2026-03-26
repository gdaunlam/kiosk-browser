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

The keyboard guard uses a two-tier strategy:

### Primary: evdev backend (recommended)

Grabs keyboard devices exclusively at the kernel input level (`/dev/input/eventN`)
using `EVIOCGRAB`. Blocked key events are discarded; all other events are
forwarded through a virtual keyboard created via `uinput`.

This **bypasses the window manager entirely**, so it works even when KDE, GNOME,
or other WMs hold their own key grabs (the `BadAccess` problem with X11 grabs).

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

### Fallback: X11 grabs

If evdev is unavailable (insufficient permissions), the guard falls back to
`XGrabKey` on the X11 root window. This approach has known limitations:

- **Window manager conflicts:** KDE (KWin), GNOME Shell, etc. hold their own
  grabs on keys like Super. `XGrabKey` fails with `BadAccess` for those keys.
  The app logs warnings and suggests workarounds (e.g., `gsettings` to release
  the Super key on GNOME).
- **X11 only.** Does not work under Wayland.

### Limitations (both backends)

- **Ctrl+Alt+Del** cannot be blocked (kernel-level on Linux).
- On Wayland without evdev permissions, keys **will not be blocked**. Use a
  kiosk compositor like [cage](https://github.com/cage-kiosk/cage) or grant
  `input` group access.

## Cross-compilation

Cross-compiling **from Linux to Windows** is not practical with Tauri.
Use GitHub Actions or build natively on each target OS.
