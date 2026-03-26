mod cli;
mod keyboard;

use cli::Cli;
use keyboard::keys::resolve_blocked_keys;
use url::Url;

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = <Cli as clap::Parser>::parse();
    let target_url = Url::parse(&cli.url).unwrap_or_else(|e| {
        log::error!("Invalid URL '{}': {e}", cli.url);
        std::process::exit(1);
    });

    let blocked_keys = resolve_blocked_keys(&cli.block_keys, &cli.block_keys_preset);
    let fullscreen = cli.fullscreen;

    tauri::Builder::default()
        .setup(move |app| {
            use tauri::{WebviewUrl, WebviewWindowBuilder};

            let mut builder =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::External(target_url.clone()))
                    .title("Kiosk Browser")
                    .decorations(false)
                    .fullscreen(fullscreen);

            let downloads_dir = dirs::download_dir().unwrap_or_else(std::env::temp_dir);
            builder = builder.on_download(move |_webview, event| {
                use tauri::webview::DownloadEvent;
                match event {
                    DownloadEvent::Requested { url, destination } => {
                        if let Some(filename) = url.split('/').last().filter(|s| !s.is_empty()) {
                            *destination = downloads_dir.join(filename);
                        }
                        log::info!("Download started: {url} -> {destination:?}");
                        true
                    }
                    DownloadEvent::Finished { url, path, success } => {
                        if success {
                            log::info!("Download complete: {url} -> {path:?}");
                        } else {
                            log::warn!("Download failed: {url}");
                        }
                        true
                    }
                }
            });

            builder.build()?;

            if !blocked_keys.is_empty() {
                keyboard::start_guard(blocked_keys);
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error running kiosk-browser");
}
