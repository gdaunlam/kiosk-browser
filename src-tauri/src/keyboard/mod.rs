pub mod keys;

#[cfg(target_os = "windows")]
mod windows;

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
    linux_x11::install_hook(keys);

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    log::warn!("Keyboard guard: not supported on this platform");
}
