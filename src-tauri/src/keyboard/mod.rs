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
/// On Linux, captured events are replayed to the focused window (the webview)
/// via `XAllowEvents(ReplayKeyboard)` so the web page receives them as trusted
/// DOM events while the WM/OS does not.
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

        // evdev: grabs physical keyboard devices at the kernel level.
        // Always try — effective on bare-metal machines.
        log::info!("Keyboard guard: trying evdev backend (requires root or 'input' group)");
        let evdev_active = linux_evdev::install_hook(keys.clone());
        if evdev_active {
            log::info!("Keyboard guard: evdev backend active");
        }

        if !is_wayland {
            // Release WM grabs so our XGrabKey calls don't get BadAccess
            let _ = try_disable_wm_shortcuts();

            // X11 grabs with GrabModeSync + ReplayKeyboard: events are blocked
            // from the WM but replayed to the focused webview window.
            linux_x11::install_hook(keys);
        } else if !evdev_active {
            log::error!(
                "Keyboard guard: evdev failed and Wayland has no X11 fallback. \
                 Keys will NOT be blocked. Run as root, add user to 'input' group, \
                 or use a kiosk compositor like cage."
            );
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    log::warn!("Keyboard guard: not supported on this platform");
}

/// Best-effort: ask the running Window Manager to release its global shortcut
/// grabs. Returns `true` if at least one WM-specific disabling succeeded,
/// meaning keys will flow naturally to the webview without XGrabKey.
#[cfg(target_os = "linux")]
fn try_disable_wm_shortcuts() -> bool {
    ensure_dbus_session_env();

    if try_disable_kwin() {
        return true;
    }

    if try_disable_gnome() {
        return true;
    }

    if try_disable_xfce() {
        return true;
    }

    false
}

/// When running via `sudo`, DBUS_SESSION_BUS_ADDRESS is usually stripped.
/// Try to reconstruct it from the original user's runtime dir.
#[cfg(target_os = "linux")]
fn ensure_dbus_session_env() {
    if std::env::var("DBUS_SESSION_BUS_ADDRESS").is_ok() {
        return;
    }
    if let Ok(uid) = std::env::var("SUDO_UID") {
        let bus_path = format!("/run/user/{}/bus", uid);
        if std::path::Path::new(&bus_path).exists() {
            let addr = format!("unix:path={}", bus_path);
            // SAFETY: single-threaded at this point (called before spawning guard threads)
            unsafe { std::env::set_var("DBUS_SESSION_BUS_ADDRESS", &addr) };
            log::debug!("Set DBUS_SESSION_BUS_ADDRESS={addr} (from SUDO_UID={uid})");
        }
    }
}

#[cfg(target_os = "linux")]
fn try_disable_kwin() -> bool {
    // Both strategies are complementary, not exclusive.
    // D-Bus toggle (Plasma 6) may succeed but miss modifier-only shortcuts;
    // config-file overrides (Plasma 5/6) handle those.
    let dbus_ok = try_kwin_dbus_disable();
    let config_ok = try_kwin_config_disable();
    dbus_ok || config_ok
}

/// Plasma 6 exposes `disableGlobalShortcuts` on D-Bus. Plasma 5 does not.
#[cfg(target_os = "linux")]
fn try_kwin_dbus_disable() -> bool {
    for cmd in ["qdbus6", "qdbus"] {
        if let Ok(output) = std::process::Command::new(cmd)
            .args(["org.kde.KWin", "/KWin", "disableGlobalShortcuts"])
            .output()
        {
            if output.status.success() {
                log::info!("Disabled KWin global shortcuts via {cmd} D-Bus call");
                return true;
            }
        }
    }

    if let Ok(output) = std::process::Command::new("dbus-send")
        .args([
            "--session",
            "--print-reply",
            "--dest=org.kde.KWin",
            "--type=method_call",
            "/KWin",
            "org.kde.KWin.disableGlobalShortcuts",
        ])
        .output()
    {
        if output.status.success() {
            log::info!("Disabled KWin global shortcuts via dbus-send");
            return true;
        }
    }

    log::debug!("KWin disableGlobalShortcuts not available (expected on Plasma 5)");
    false
}

/// Overwrite shortcut entries in `kglobalshortcutsrc` and `kwinrc`, then
/// ask KWin to reload. Works on both Plasma 5 and 6.
#[cfg(target_os = "linux")]
fn try_kwin_config_disable() -> bool {
    let kwriteconfig = match find_command(&["kwriteconfig6", "kwriteconfig5"]) {
        Some(cmd) => cmd,
        None => {
            log::debug!("kwriteconfig not found, cannot disable KDE shortcuts via config");
            return false;
        }
    };

    let mut changed = false;

    // kglobalshortcutsrc — value format: "current,default,friendly_name"
    let overrides: &[(&str, &str, &str)] = &[
        ("kwin", "Window Close", "none,Alt+F4,Close Window"),
        ("kwin", "Walk Through Windows", "none,Alt+Tab,Walk Through Windows"),
        (
            "kwin",
            "Walk Through Windows (Reverse)",
            "none,Alt+Shift+Backtab,Walk Through Windows (Reverse)",
        ),
        ("kwin", "Show Desktop", "none,Meta+D,Peek at Desktop"),
        (
            "kwin",
            "ExposeAll",
            "none,Ctrl+F10,Toggle Present Windows (All desktops)",
        ),
        (
            "kwin",
            "Expose",
            "none,Ctrl+F9,Toggle Present Windows (Current desktop)",
        ),
    ];

    for (group, key, value) in overrides {
        changed |= kwrite_shortcut(&kwriteconfig, "kglobalshortcutsrc", group, key, value);
    }

    // kwinrc — disable the Meta-only modifier shortcut (opens app launcher)
    changed |= kwrite_shortcut(&kwriteconfig, "kwinrc", "ModifierOnlyShortcuts", "Meta", "");

    if changed {
        reconfigure_kwin();
        log::info!("Disabled KDE shortcuts via {kwriteconfig} + KWin reconfigure");
    }

    changed
}

/// Run `kwriteconfig{5,6}` as the desktop user (handles sudo correctly).
#[cfg(target_os = "linux")]
fn kwrite_shortcut(kwriteconfig: &str, file: &str, group: &str, key: &str, value: &str) -> bool {
    let output = if let Ok(user) = std::env::var("SUDO_USER") {
        std::process::Command::new("sudo")
            .args([
                "-u",
                &user,
                "--",
                kwriteconfig,
                "--file",
                file,
                "--group",
                group,
                "--key",
                key,
                value,
            ])
            .output()
    } else {
        std::process::Command::new(kwriteconfig)
            .args(["--file", file, "--group", group, "--key", key, value])
            .output()
    };

    matches!(output, Ok(ref o) if o.status.success())
}

#[cfg(target_os = "linux")]
fn find_command(candidates: &[&str]) -> Option<String> {
    for cmd in candidates {
        if let Ok(o) = std::process::Command::new("which").arg(cmd).output() {
            if o.status.success() {
                return Some((*cmd).to_string());
            }
        }
    }
    None
}

/// Tell KWin to reload its config files (applies changes made by kwriteconfig).
#[cfg(target_os = "linux")]
fn reconfigure_kwin() {
    for cmd in ["qdbus6", "qdbus"] {
        if let Ok(o) = std::process::Command::new(cmd)
            .args(["org.kde.KWin", "/KWin", "reconfigure"])
            .output()
        {
            if o.status.success() {
                log::debug!("KWin reconfigured via {cmd}");
                return;
            }
        }
    }

    let _ = std::process::Command::new("dbus-send")
        .args([
            "--session",
            "--dest=org.kde.KWin",
            "--type=method_call",
            "/KWin",
            "org.kde.KWin.reconfigure",
        ])
        .output();
}

#[cfg(target_os = "linux")]
fn try_disable_gnome() -> bool {
    let overrides: &[(&str, &str, &str)] = &[
        ("org.gnome.mutter", "overlay-key", ""),
        ("org.gnome.desktop.wm.keybindings", "switch-applications", "[]"),
        ("org.gnome.desktop.wm.keybindings", "switch-windows", "[]"),
        ("org.gnome.desktop.wm.keybindings", "close", "[]"),
        ("org.gnome.desktop.wm.keybindings", "cycle-windows", "[]"),
        ("org.gnome.desktop.wm.keybindings", "panel-main-menu", "[]"),
        ("org.gnome.desktop.wm.keybindings", "show-desktop", "[]"),
    ];

    let mut any = false;
    for (schema, key, value) in overrides {
        if let Ok(o) = std::process::Command::new("gsettings")
            .args(["set", schema, key, value])
            .output()
        {
            if o.status.success() {
                any = true;
            }
        }
    }
    if any {
        log::info!(
            "Disabled GNOME shortcuts via gsettings \
             (changes persist until manually restored)"
        );
    }
    any
}

#[cfg(target_os = "linux")]
fn try_disable_xfce() -> bool {
    let keys: &[(&str, &str)] = &[
        ("/xfwm4/custom/<Super>", "override"),
        ("/xfwm4/custom/<Alt>Tab", "override"),
        ("/xfwm4/custom/<Alt>F4", "override"),
    ];

    let mut any = false;
    for (prop, _) in keys {
        if let Ok(o) = std::process::Command::new("xfconf-query")
            .args(["-c", "xfce4-keyboard-shortcuts", "-p", prop, "-r"])
            .output()
        {
            if o.status.success() {
                any = true;
            }
        }
    }
    if any {
        log::info!("Removed XFCE shortcut overrides via xfconf-query");
    }
    any
}
