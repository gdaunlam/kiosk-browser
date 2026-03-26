use super::keys::BlockableKey;
use std::collections::HashSet;
use std::sync::OnceLock;

use windows::Win32::Foundation::{HMODULE, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_D, VK_E, VK_ESCAPE, VK_F4, VK_L, VK_LWIN, VK_R, VK_RWIN,
    VK_TAB,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    HHOOK, KBDLLHOOKSTRUCT, LLKHF_ALTDOWN, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
};

static BLOCKED_KEYS: OnceLock<HashSet<BlockableKey>> = OnceLock::new();

/// Install a low-level keyboard hook that blocks the configured keys.
/// Runs the message pump on a dedicated background thread.
pub fn install_hook(keys: HashSet<BlockableKey>) {
    BLOCKED_KEYS.set(keys).ok();

    std::thread::Builder::new()
        .name("keyboard-guard".into())
        .spawn(|| unsafe {
            let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), HMODULE::default(), 0) {
                Ok(h) => h,
                Err(e) => {
                    log::error!("Failed to install keyboard hook: {e}");
                    return;
                }
            };

            log::info!("Low-level keyboard hook installed");

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = DispatchMessageW(&msg);
            }

            let _ = UnhookWindowsHookEx(hook);
            log::info!("Keyboard hook removed");
        })
        .expect("Failed to spawn keyboard guard thread");
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let action = wparam.0 as u32;
        if action == WM_KEYDOWN || action == WM_SYSKEYDOWN {
            let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            if should_block(kb) {
                return LRESULT(1);
            }
        }
    }
    CallNextHookEx(HHOOK::default(), code, wparam, lparam)
}

unsafe fn should_block(kb: &KBDLLHOOKSTRUCT) -> bool {
    let keys = match BLOCKED_KEYS.get() {
        Some(k) => k,
        None => return false,
    };

    let vk = kb.vkCode;
    let alt_down = (kb.flags & LLKHF_ALTDOWN).0 != 0;

    let win_down = || -> bool {
        GetAsyncKeyState(VK_LWIN.0 as i32) < 0 || GetAsyncKeyState(VK_RWIN.0 as i32) < 0
    };
    let ctrl_down = || -> bool { GetAsyncKeyState(VK_CONTROL.0 as i32) < 0 };

    // Win key alone
    if (vk == VK_LWIN.0 as u32 || vk == VK_RWIN.0 as u32) && keys.contains(&BlockableKey::Win) {
        return true;
    }

    // Alt + Tab
    if vk == VK_TAB.0 as u32 && alt_down && keys.contains(&BlockableKey::AltTab) {
        return true;
    }

    // Alt + F4
    if vk == VK_F4.0 as u32 && alt_down && keys.contains(&BlockableKey::AltF4) {
        return true;
    }

    // Alt + Escape
    if vk == VK_ESCAPE.0 as u32 && alt_down && keys.contains(&BlockableKey::AltEsc) {
        return true;
    }

    // Ctrl + Escape (Start menu)
    if vk == VK_ESCAPE.0 as u32 && ctrl_down() && keys.contains(&BlockableKey::CtrlEsc) {
        return true;
    }

    // Win + Tab (Task View)
    if vk == VK_TAB.0 as u32 && win_down() && keys.contains(&BlockableKey::WinTab) {
        return true;
    }

    // Win + D (show desktop)
    if vk == VK_D.0 as u32 && win_down() && keys.contains(&BlockableKey::WinD) {
        return true;
    }

    // Win + E (explorer)
    if vk == VK_E.0 as u32 && win_down() && keys.contains(&BlockableKey::WinE) {
        return true;
    }

    // Win + R (run dialog)
    if vk == VK_R.0 as u32 && win_down() && keys.contains(&BlockableKey::WinR) {
        return true;
    }

    // Win + L (lock screen — may be handled at kernel level on some versions)
    if vk == VK_L.0 as u32 && win_down() && keys.contains(&BlockableKey::WinL) {
        return true;
    }

    false
}
