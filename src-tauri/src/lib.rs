mod cli;
mod keyboard;

use cli::Cli;
use keyboard::keys::resolve_blocked_keys;
use url::Url;

const CLOSE_BUTTON_JS: &str = r#"
(function() {
    function inject() {
        if (document.getElementById('kiosk-close-tab')) return;
        var el = document.createElement('div');
        el.id = 'kiosk-close-tab';
        el.innerHTML = '<style>'
            + '#kiosk-close-tab { position:fixed; top:0; right:20px; z-index:2147483647; }'
            + '#kiosk-close-tab .ktab { width:48px; height:6px; background:rgba(180,180,180,0.35);'
            + '  border-radius:0 0 8px 8px; transition:all .25s ease; display:flex;'
            + '  align-items:center; justify-content:center; cursor:pointer; overflow:hidden; }'
            + '#kiosk-close-tab:hover .ktab { height:36px; background:rgba(200,40,40,0.92);'
            + '  border-radius:0 0 10px 10px; }'
            + '#kiosk-close-tab .kx { opacity:0; color:#fff; font:bold 18px/1 system-ui,sans-serif;'
            + '  user-select:none; transition:opacity .15s; }'
            + '#kiosk-close-tab:hover .kx { opacity:1; }'
            + '</style>'
            + '<div class="ktab"><span class="kx">\u2715</span></div>';
        el.addEventListener('click', function() {
            window.location.href = 'kiosk://close';
        });
        document.body.appendChild(el);
    }
    if (document.body) inject();
    else document.addEventListener('DOMContentLoaded', inject);
})();
"#;

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

            let downloads_dir = dirs::download_dir().unwrap_or_else(std::env::temp_dir);

            // Build with local page first so TLS policy can be set before loading external URL
            let win = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Kiosk Browser")
                .decorations(false)
                .fullscreen(fullscreen)
                .initialization_script(CLOSE_BUTTON_JS)
                .on_navigation(|url| {
                    if url.as_str().starts_with("kiosk://close") {
                        log::info!("Close requested via UI");
                        std::process::exit(0);
                    }
                    true
                })
                .on_download(move |_webview, event| {
                    use tauri::webview::DownloadEvent;
                    match event {
                        DownloadEvent::Requested { url, destination } => {
                            let url_str = url.to_string();
                            if let Some(filename) = url
                                .path_segments()
                                .and_then(|s| s.last())
                                .filter(|s| !s.is_empty())
                            {
                                *destination = downloads_dir.join(filename);
                            }
                            log::info!("Download started: {url_str} -> {destination:?}");
                            true
                        }
                        DownloadEvent::Finished { url, path, success } => {
                            if success {
                                log::info!("Download complete: {} -> {path:?}", url);
                            } else {
                                log::warn!("Download failed: {}", url);
                            }
                            true
                        }
                        _ => true,
                    }
                })
                .build()?;

            // On Linux: set TLS policy on the webview's actual context, then navigate
            #[cfg(target_os = "linux")]
            {
                let nav_url = target_url.to_string();
                win.with_webview(move |wv| {
                    use webkit2gtk::{WebContextExt, WebViewExt};
                    let webview = wv.inner();
                    if let Some(ctx) = webview.context() {
                        ctx.set_tls_errors_policy(webkit2gtk::TLSErrorsPolicy::Ignore);
                        log::info!("TLS errors policy set to Ignore on webview context");
                    }
                    webview.load_uri(&nav_url);
                })?;
            }

            // On non-Linux: navigate directly
            #[cfg(not(target_os = "linux"))]
            {
                win.navigate(target_url)?;
            }

            if !blocked_keys.is_empty() {
                keyboard::start_guard(blocked_keys);
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error running kiosk-browser");
}
