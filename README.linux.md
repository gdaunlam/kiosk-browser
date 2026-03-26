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
./kiosk-browser --url https://tailscale-services:38999/ --fullscreen --block-keys-preset kiosk
```

## Keyboard guard details

Uses `XGrabKey` on the X11 root window to intercept key combinations before
they reach the window manager. Grabbed keys are silently consumed in a
background thread (events are drained and discarded).

Accounts for NumLock and CapsLock modifiers automatically (grabs all
combinations of these lock modifiers for each blocked key).

### Limitations

- **X11 only.** Wayland does not allow applications to grab global keys by design.
  For Wayland kiosks, use a kiosk compositor like [cage](https://github.com/cage-kiosk/cage)
  which simply does not expose Alt+Tab, workspaces, etc.
- Super key (Win) is grabbed with `AnyModifier`, so blocking `win` also
  prevents all Super+X combinations from reaching the WM.
- The keyboard guard thread opens its own X11 display connection, separate
  from the Tauri webview. This is safe but means the `DISPLAY` environment
  variable must be set correctly.

## Cross-compilation

Cross-compiling **from Linux to Windows** is not practical with Tauri.
Use GitHub Actions or build natively on each target OS.
