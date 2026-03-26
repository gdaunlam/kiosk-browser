use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "kiosk-browser",
    about = "Kiosk browser with low-level keyboard capture",
    long_about = "Opens a URL in a locked-down browser window that captures system keyboard \
                  shortcuts (Win, Alt+Tab, etc.) to prevent users from escaping the kiosk."
)]
pub struct Cli {
    /// URL to load in the kiosk browser
    #[arg(long)]
    pub url: String,

    /// Start in fullscreen mode
    #[arg(long, default_value_t = false)]
    pub fullscreen: bool,

    /// Comma-separated list of keys to block.
    /// Available keys: win, alt+tab, alt+f4, alt+esc, ctrl+esc,
    /// win+tab, win+d, win+e, win+r, win+l
    #[arg(long, value_delimiter = ',')]
    pub block_keys: Option<Vec<String>>,

    /// Use a preset key blocking profile.
    /// "kiosk" blocks all available system shortcuts.
    /// "none" blocks nothing (default behavior).
    #[arg(long, value_parser = ["kiosk", "none"])]
    pub block_keys_preset: Option<String>,
}
