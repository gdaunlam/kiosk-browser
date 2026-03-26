use super::keys::BlockableKey;
use std::collections::HashSet;
use std::os::raw::{c_int, c_uint};
use std::ptr;

use x11::keysym::*;
use x11::xlib::*;

const IGNORED_MODS: [c_uint; 4] = [
    0,
    Mod2Mask,             // NumLock
    LockMask,             // CapsLock
    Mod2Mask | LockMask,  // both
];

/// Install X11 key grabs that intercept system shortcuts before the window manager.
/// Runs an event drain loop on a dedicated background thread.
pub fn install_hook(keys: HashSet<BlockableKey>) {
    std::thread::Builder::new()
        .name("keyboard-guard".into())
        .spawn(move || unsafe {
            let display = XOpenDisplay(ptr::null());
            if display.is_null() {
                log::error!("Failed to open X11 display — keyboard guard disabled");
                return;
            }

            let root = XDefaultRootWindow(display);
            let mut grab_count = 0u32;

            for key in &keys {
                grab_count += grab_key_combo(display, root, key);
            }

            XSync(display, False);
            log::info!("X11 keyboard guard installed ({grab_count} grabs on root window)");

            drain_events(display);

            for key in &keys {
                ungrab_key_combo(display, root, key);
            }
            XCloseDisplay(display);
            log::info!("X11 keyboard guard removed");
        })
        .expect("Failed to spawn keyboard guard thread");
}

/// Grab a single key combo on the root window, accounting for NumLock/CapsLock.
/// Returns the number of successful grabs.
unsafe fn grab_key_combo(display: *mut Display, root: Window, key: &BlockableKey) -> u32 {
    let combos = key_to_x11(display, key);
    let mut count = 0;
    for (keycode, base_mod) in combos {
        if keycode == 0 {
            continue;
        }
        // AnyModifier already covers all modifier combinations
        if base_mod == AnyModifier {
            let result = XGrabKey(
                display,
                keycode as c_int,
                AnyModifier,
                root,
                True,
                GrabModeAsync,
                GrabModeAsync,
            );
            if result != 0 {
                count += 1;
            }
        } else {
            for &extra in &IGNORED_MODS {
                let modmask = base_mod | extra;
                let result = XGrabKey(
                    display,
                    keycode as c_int,
                    modmask,
                    root,
                    True,
                    GrabModeAsync,
                    GrabModeAsync,
                );
                if result != 0 {
                    count += 1;
                }
            }
        }
    }
    count
}

unsafe fn ungrab_key_combo(display: *mut Display, root: Window, key: &BlockableKey) {
    let combos = key_to_x11(display, key);
    for (keycode, base_mod) in combos {
        if keycode == 0 {
            continue;
        }
        if base_mod == AnyModifier {
            XUngrabKey(display, keycode as c_int, AnyModifier, root);
        } else {
            for &extra in &IGNORED_MODS {
                XUngrabKey(display, keycode as c_int, base_mod | extra, root);
            }
        }
    }
}

/// Map a BlockableKey to (keycode, modifier_mask) pairs for X11.
unsafe fn key_to_x11(display: *mut Display, key: &BlockableKey) -> Vec<(u32, c_uint)> {
    let kc = |sym: u32| -> u32 { XKeysymToKeycode(display, sym as KeySym) as u32 };

    match key {
        BlockableKey::Win => vec![
            (kc(XK_Super_L), AnyModifier),
            (kc(XK_Super_R), AnyModifier),
        ],
        BlockableKey::AltTab => vec![(kc(XK_Tab), Mod1Mask)],
        BlockableKey::AltF4 => vec![(kc(XK_F4), Mod1Mask)],
        BlockableKey::AltEsc => vec![(kc(XK_Escape), Mod1Mask)],
        BlockableKey::CtrlEsc => vec![(kc(XK_Escape), ControlMask)],
        BlockableKey::WinTab => vec![(kc(XK_Tab), Mod4Mask)],
        BlockableKey::WinD => vec![(kc(XK_d), Mod4Mask)],
        BlockableKey::WinE => vec![(kc(XK_e), Mod4Mask)],
        BlockableKey::WinR => vec![(kc(XK_r), Mod4Mask)],
        BlockableKey::WinL => vec![(kc(XK_l), Mod4Mask)],
    }
}

/// Drain X events forever so grabbed key events don't pile up.
/// The events are intentionally discarded — we only want to suppress them.
unsafe fn drain_events(display: *mut Display) {
    let mut event: XEvent = std::mem::zeroed();
    loop {
        XNextEvent(display, &mut event);
    }
}
