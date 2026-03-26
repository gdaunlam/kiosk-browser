use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockableKey {
    /// Left/Right Windows key (Super on Linux)
    Win,
    /// Alt + Tab (window switcher)
    AltTab,
    /// Alt + F4 (close window)
    AltF4,
    /// Alt + Escape
    AltEsc,
    /// Ctrl + Escape (Start menu on Windows)
    CtrlEsc,
    /// Win + Tab (Task View on Windows)
    WinTab,
    /// Win + D (show desktop)
    WinD,
    /// Win + E (open explorer)
    WinE,
    /// Win + R (run dialog)
    WinR,
    /// Win + L (lock screen — may not work on all Windows versions)
    WinL,
}

impl BlockableKey {
    pub fn all() -> HashSet<BlockableKey> {
        HashSet::from([
            BlockableKey::Win,
            BlockableKey::AltTab,
            BlockableKey::AltF4,
            BlockableKey::AltEsc,
            BlockableKey::CtrlEsc,
            BlockableKey::WinTab,
            BlockableKey::WinD,
            BlockableKey::WinE,
            BlockableKey::WinR,
            BlockableKey::WinL,
        ])
    }
}

impl fmt::Display for BlockableKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockableKey::Win => write!(f, "win"),
            BlockableKey::AltTab => write!(f, "alt+tab"),
            BlockableKey::AltF4 => write!(f, "alt+f4"),
            BlockableKey::AltEsc => write!(f, "alt+esc"),
            BlockableKey::CtrlEsc => write!(f, "ctrl+esc"),
            BlockableKey::WinTab => write!(f, "win+tab"),
            BlockableKey::WinD => write!(f, "win+d"),
            BlockableKey::WinE => write!(f, "win+e"),
            BlockableKey::WinR => write!(f, "win+r"),
            BlockableKey::WinL => write!(f, "win+l"),
        }
    }
}

impl FromStr for BlockableKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "win" | "super" | "meta" => Ok(BlockableKey::Win),
            "alt+tab" | "alttab" => Ok(BlockableKey::AltTab),
            "alt+f4" | "altf4" => Ok(BlockableKey::AltF4),
            "alt+esc" | "altesc" => Ok(BlockableKey::AltEsc),
            "ctrl+esc" | "ctrlesc" => Ok(BlockableKey::CtrlEsc),
            "win+tab" | "wintab" | "super+tab" => Ok(BlockableKey::WinTab),
            "win+d" | "wind" | "super+d" => Ok(BlockableKey::WinD),
            "win+e" | "wine" | "super+e" => Ok(BlockableKey::WinE),
            "win+r" | "winr" | "super+r" => Ok(BlockableKey::WinR),
            "win+l" | "winl" | "super+l" => Ok(BlockableKey::WinL),
            other => Err(format!("Unknown key: '{other}'")),
        }
    }
}

/// Resolve the set of keys to block from CLI arguments.
pub fn resolve_blocked_keys(
    block_keys: &Option<Vec<String>>,
    preset: &Option<String>,
) -> HashSet<BlockableKey> {
    if let Some(preset_name) = preset {
        return match preset_name.as_str() {
            "kiosk" => BlockableKey::all(),
            "none" | _ => HashSet::new(),
        };
    }

    if let Some(keys) = block_keys {
        let mut set = HashSet::new();
        for k in keys {
            match BlockableKey::from_str(k) {
                Ok(key) => {
                    set.insert(key);
                }
                Err(e) => {
                    log::warn!("{e}");
                }
            }
        }
        return set;
    }

    HashSet::new()
}
