pub mod keys;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux_evdev;

#[cfg(target_os = "linux")]
mod linux_x11;

use keys::BlockableKey;
use std::collections::HashSet;

/// Start the keyboard guard, blocking the given set of keys at the OS level.
/// Spawns a background thread that runs for the lifetime of the process.
pub fn start_guard(keys: HashSet<BlockableKey>) {
    if keys.is_empty() {
        log::info!("Keyboard guard: no keys to block, skipping");
        return;
    }

    log::info!(
        "Keyboard guard: blocking {} keys: {}",
        keys.len(),
        keys.iter()
            .map(|k| k.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    #[cfg(target_os = "windows")]
    windows::install_hook(keys);

    #[cfg(target_os = "linux")]
    {
        let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
        let wayland = std::env::var("WAYLAND_DISPLAY").ok();
        let is_wayland =
            session.eq_ignore_ascii_case("wayland") || wayland.is_some();

        if is_wayland {
            log::warn!(
                "Keyboard guard: Wayland session detected (XDG_SESSION_TYPE={session:?}, \
                 WAYLAND_DISPLAY={wayland:?}). X11 key grabs will NOT work. \
                 Trying evdev backend (works on both X11 and Wayland)."
            );
        }

        log::info!("Keyboard guard: trying evdev backend (requires root or 'input' group)");
        if linux_evdev::install_hook(keys.clone()) {
            log::info!("Keyboard guard: evdev backend active");
            return;
        }

        if is_wayland {
            log::error!(
                "Keyboard guard: evdev failed and Wayland has no X11 fallback. \
                 Keys will NOT be blocked. Run as root, add user to 'input' group, \
                 or use a kiosk compositor like cage."
            );
            return;
        }

        log::info!("Keyboard guard: evdev unavailable, falling back to X11 grabs");
        linux_x11::install_hook(keys);
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    log::warn!("Keyboard guard: not supported on this platform");
}
