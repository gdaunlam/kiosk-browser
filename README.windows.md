# kiosk-browser — Windows

Setup, build, and platform-specific notes for Windows.

## Prerequisites

### 1. Install Rust

Download and run the installer from [https://rustup.rs](https://rustup.rs/).
This installs `rustup`, `cargo`, and `rustc`. The default options are fine.

After installation, open a **new** terminal and verify:

```powershell
rustc --version
cargo --version
```

### 2. Install Tauri CLI

```powershell
cargo install tauri-cli --version "^2"
```

### 3. WebView2

Windows 10 (1803+) and Windows 11 already include WebView2 via Edge.
No extra installation needed. The `windows` crate handles Win32 API bindings at compile time.

## Build

```powershell
cd src-tauri
cargo tauri build
```

The compiled binary will be in `src-tauri\target\release\kiosk-browser.exe`.

Tauri also generates installers under `src-tauri\target\release\bundle\` (NSIS `.exe` and MSI).

### Development mode

```powershell
cd src-tauri
cargo tauri dev -- -- --url https://example.com --fullscreen --block-keys-preset kiosk
```

## Run

```powershell
kiosk-browser.exe --url https://tailscale-services:38999/ --fullscreen --block-keys-preset kiosk
```

## Keyboard guard details

Uses `SetWindowsHookEx` with `WH_KEYBOARD_LL` (low-level keyboard hook).
Runs a Win32 message pump in a dedicated background thread.

### Limitations

- **Ctrl+Alt+Del** cannot be blocked (Secure Attention Sequence, kernel-level).
- **Win+L** (lock screen) may not be blockable on some Windows versions
  where it is handled at the kernel level before the hook sees it.
- No admin rights are required to install the hook, but running as admin
  may improve reliability on locked-down enterprise systems.

## Cross-compilation

Cross-compiling **from Linux to Windows** is not practical with Tauri because
it depends on native Windows SDK headers and WebView2. Use GitHub Actions
or build natively on a Windows machine.
