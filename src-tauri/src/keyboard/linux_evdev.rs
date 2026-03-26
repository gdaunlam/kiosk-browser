use super::keys::BlockableKey;
use evdev::{Device, EventType, InputEvent, KeyCode};
use std::collections::HashSet;
use std::path::PathBuf;

/// Try to install the evdev-based keyboard guard.
/// Returns `true` if at least one device was successfully grabbed.
pub fn install_hook(keys: HashSet<BlockableKey>) -> bool {
    let devices = find_keyboard_devices();
    if devices.is_empty() {
        log::warn!(
            "evdev: no keyboard devices found in /dev/input/. \
             Ensure the process runs as root or the user is in the 'input' group."
        );
        return false;
    }

    log::info!("evdev: found {} keyboard device(s)", devices.len());
    let mut any_success = false;

    for dev_path in devices {
        let keys = keys.clone();
        let path_display = dev_path.display().to_string();

        let mut device = match Device::open(&dev_path) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("evdev: cannot open {}: {e}", path_display);
                continue;
            }
        };

        let dev_name = device.name().unwrap_or("unknown").to_string();

        if let Err(e) = device.grab() {
            log::warn!("evdev: cannot grab '{}' ({}): {e}", dev_name, path_display);
            continue;
        }

        let virtual_dev = match build_virtual_device(&device, &dev_name) {
            Ok(vd) => vd,
            Err(e) => {
                log::error!(
                    "evdev: cannot create virtual device for '{}': {e}. Releasing grab.",
                    dev_name
                );
                let _ = device.ungrab();
                continue;
            }
        };

        log::info!("evdev: grabbed '{}' ({})", dev_name, path_display);
        any_success = true;

        std::thread::Builder::new()
            .name("evdev-guard".into())
            .spawn(move || {
                run_filter(device, virtual_dev, &dev_name, &keys);
            })
            .expect("Failed to spawn evdev guard thread");
    }

    any_success
}

fn find_keyboard_devices() -> Vec<PathBuf> {
    let mut keyboards = Vec::new();

    let entries = match std::fs::read_dir("/dev/input") {
        Ok(e) => e,
        Err(e) => {
            log::error!("Cannot read /dev/input: {e}");
            return keyboards;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        match path.file_name().and_then(|n| n.to_str()) {
            Some(n) if n.starts_with("event") => {}
            _ => continue,
        }

        match Device::open(&path) {
            Ok(device) => {
                if is_keyboard(&device) {
                    log::debug!(
                        "evdev: {} is a keyboard ('{}')",
                        path.display(),
                        device.name().unwrap_or("unknown")
                    );
                    keyboards.push(path);
                }
            }
            Err(e) => {
                log::debug!("evdev: cannot open {}: {e}", path.display());
            }
        }
    }

    keyboards
}

fn is_keyboard(device: &Device) -> bool {
    let keys = match device.supported_keys() {
        Some(k) => k,
        None => return false,
    };

    keys.contains(KeyCode::KEY_A)
        && keys.contains(KeyCode::KEY_Z)
        && keys.contains(KeyCode::KEY_LEFTCTRL)
        && keys.contains(KeyCode::KEY_LEFTALT)
}

fn build_virtual_device(
    device: &Device,
    name: &str,
) -> Result<evdev::uinput::VirtualDevice, Box<dyn std::error::Error>> {
    let vdev_name = format!("kiosk-{}", name);
    let mut builder = evdev::uinput::VirtualDevice::builder()?.name(&vdev_name);

    if let Some(keys) = device.supported_keys() {
        builder = builder.with_keys(keys)?;
    }

    if let Some(rel) = device.supported_relative_axes() {
        builder = builder.with_relative_axes(rel)?;
    }

    Ok(builder.build()?)
}

fn run_filter(
    mut device: Device,
    mut virtual_dev: evdev::uinput::VirtualDevice,
    dev_name: &str,
    blocked: &HashSet<BlockableKey>,
) {
    let mut filter = KeyFilterState::new(blocked.clone());

    loop {
        let events: Vec<InputEvent> = match device.fetch_events() {
            Ok(iter) => iter.collect(),
            Err(e) => {
                log::error!("evdev: read error on '{}': {e}", dev_name);
                break;
            }
        };

        let mut forward: Vec<InputEvent> = Vec::new();

        for event in events {
            if event.event_type() == EventType::SYNCHRONIZATION {
                continue;
            }

            if event.event_type() == EventType::KEY {
                let key = KeyCode(event.code());
                let action = event.value(); // 0=release, 1=press, 2=repeat

                match action {
                    1 => filter.update_modifier(key, true),
                    0 => filter.update_modifier(key, false),
                    _ => {}
                }

                match action {
                    1 | 2 => {
                        if filter.should_block(key) {
                            filter.suppressed.insert(key);
                            log::debug!("evdev: blocked {:?} (value={})", key, action);
                            continue;
                        }
                        filter.suppressed.remove(&key);
                    }
                    0 => {
                        if filter.suppressed.remove(&key) {
                            log::debug!("evdev: blocked {:?} (release)", key);
                            continue;
                        }
                    }
                    _ => {}
                }
            }

            forward.push(event);
        }

        if !forward.is_empty() {
            if let Err(e) = virtual_dev.emit(&forward) {
                log::error!("evdev: emit error: {e}");
            }
        }
    }

    log::warn!("evdev: filter loop ended for '{}'", dev_name);
    let _ = device.ungrab();
}

struct KeyFilterState {
    blocked: HashSet<BlockableKey>,
    suppressed: HashSet<KeyCode>,
    left_alt: bool,
    right_alt: bool,
    left_ctrl: bool,
    right_ctrl: bool,
    left_super: bool,
    right_super: bool,
}

impl KeyFilterState {
    fn new(blocked: HashSet<BlockableKey>) -> Self {
        Self {
            blocked,
            suppressed: HashSet::new(),
            left_alt: false,
            right_alt: false,
            left_ctrl: false,
            right_ctrl: false,
            left_super: false,
            right_super: false,
        }
    }

    fn update_modifier(&mut self, key: KeyCode, pressed: bool) {
        match key {
            KeyCode::KEY_LEFTALT => self.left_alt = pressed,
            KeyCode::KEY_RIGHTALT => self.right_alt = pressed,
            KeyCode::KEY_LEFTCTRL => self.left_ctrl = pressed,
            KeyCode::KEY_RIGHTCTRL => self.right_ctrl = pressed,
            KeyCode::KEY_LEFTMETA => self.left_super = pressed,
            KeyCode::KEY_RIGHTMETA => self.right_super = pressed,
            _ => {}
        }
    }

    fn alt_held(&self) -> bool {
        self.left_alt || self.right_alt
    }

    fn ctrl_held(&self) -> bool {
        self.left_ctrl || self.right_ctrl
    }

    fn super_held(&self) -> bool {
        self.left_super || self.right_super
    }

    fn should_block(&self, key: KeyCode) -> bool {
        if self.blocked.contains(&BlockableKey::Win)
            && matches!(key, KeyCode::KEY_LEFTMETA | KeyCode::KEY_RIGHTMETA)
        {
            return true;
        }

        if self.alt_held() {
            if self.blocked.contains(&BlockableKey::AltTab) && key == KeyCode::KEY_TAB {
                return true;
            }
            if self.blocked.contains(&BlockableKey::AltF4) && key == KeyCode::KEY_F4 {
                return true;
            }
            if self.blocked.contains(&BlockableKey::AltEsc) && key == KeyCode::KEY_ESC {
                return true;
            }
        }

        if self.ctrl_held()
            && self.blocked.contains(&BlockableKey::CtrlEsc)
            && key == KeyCode::KEY_ESC
        {
            return true;
        }

        if self.super_held() {
            if self.blocked.contains(&BlockableKey::WinTab) && key == KeyCode::KEY_TAB {
                return true;
            }
            if self.blocked.contains(&BlockableKey::WinD) && key == KeyCode::KEY_D {
                return true;
            }
            if self.blocked.contains(&BlockableKey::WinE) && key == KeyCode::KEY_E {
                return true;
            }
            if self.blocked.contains(&BlockableKey::WinR) && key == KeyCode::KEY_R {
                return true;
            }
            if self.blocked.contains(&BlockableKey::WinL) && key == KeyCode::KEY_L {
                return true;
            }
        }

        false
    }
}
