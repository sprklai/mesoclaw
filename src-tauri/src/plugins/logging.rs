use tauri::App;
use tauri_plugin_log::{Target, TargetKind, TimezoneStrategy};

pub fn build() -> tauri_plugin_log::Builder {
    tauri_plugin_log::Builder::new()
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::Webview),
            Target::new(TargetKind::LogDir {
                file_name: Some("logs".to_string()),
            }),
        ])
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .max_file_size(50_000)
}

pub fn init(app: &App) {
    log::info!("Application started");
    log::debug!("App identifier: {}", app.config().identifier);
}
