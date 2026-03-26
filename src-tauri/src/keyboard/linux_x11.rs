use super::keys::BlockableKey;
use std::collections::HashSet;
use std::os::raw::{c_int, c_uint};
use std::ptr;
use std::sync::atomic::{AtomicU32, Ordering};

use x11::keysym::*;
use x11::xlib::*;

const IGNORED_MODS: [c_uint; 4] = [
    0,
    Mod2Mask,             // NumLock
    LockMask,             // CapsLock
    Mod2Mask | LockMask,  // both
];

static GRAB_ERRORS: AtomicU32 = AtomicU32::new(0);

unsafe extern "C" fn x_error_handler(_display: *mut Display, event: *mut XErrorEvent) -> c_int {
    let err = &*event;
    // BadAccess (10) = another client already grabbed this key
    if err.error_code == 10 {
        GRAB_ERRORS.fetch_add(1, Ordering::Relaxed);
        log::debug!("X11 BadAccess on grab (request={})", err.request_code);
    } else {
        log::warn!(
            "X11 error: code={}, request={}",
            err.error_code,
            err.request_code
        );
    }
    0
}

/// Install X11 key grabs that intercept system shortcuts before the window manager.
/// Runs an event drain loop on a dedicated background thread.
pub fn install_hook(keys: HashSet<BlockableKey>) {
    std::thread::Builder::new()
        .name("keyboard-guard".into())
        .spawn(move || unsafe {
            let display = XOpenDisplay(ptr::null());
            if display.is_null() {
                log::error!(
                    "Failed to open X11 display (DISPLAY={:?}) — keyboard guard disabled",
                    std::env::var("DISPLAY").ok()
                );
                return;
            }

            XSetErrorHandler(Some(x_error_handler));

            let root = XDefaultRootWindow(display);
            GRAB_ERRORS.store(0, Ordering::Relaxed);
            let mut grab_count = 0u32;

            for key in &keys {
                grab_count += grab_key_combo(display, root, key);
            }

            XSync(display, False);

            let errors = GRAB_ERRORS.load(Ordering::Relaxed);
            if errors > 0 {
                log::warn!(
                    "X11 keyboard guard: {errors} grabs failed (BadAccess). \
                     The window manager likely already holds these key grabs. \
                     On GNOME, try: gsettings set org.gnome.mutter overlay-key '' \
                     to release the Super key."
                );
            }

            log::info!(
                "X11 keyboard guard installed ({grab_count} grabs attempted, {errors} failed)"
            );

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
/// Returns the number of grabs attempted.
unsafe fn grab_key_combo(display: *mut Display, root: Window, key: &BlockableKey) -> u32 {
    let combos = key_to_x11(display, key);
    let mut count = 0;
    for (keycode, base_mod) in combos {
        if keycode == 0 {
            log::warn!("Keycode 0 for {:?} — key not found on this keyboard", key);
            continue;
        }
        if base_mod == AnyModifier {
            XGrabKey(
                display,
                keycode as c_int,
                AnyModifier,
                root,
                True,
                GrabModeAsync,
                GrabModeAsync,
            );
            count += 1;
        } else {
            for &extra in &IGNORED_MODS {
                XGrabKey(
                    display,
                    keycode as c_int,
                    base_mod | extra,
                    root,
                    True,
                    GrabModeAsync,
                    GrabModeAsync,
                );
                count += 1;
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
