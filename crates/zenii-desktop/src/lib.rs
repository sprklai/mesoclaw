pub mod commands;
pub mod tray;

use std::sync::Arc;

use tauri::Manager;
use tracing::info;

#[allow(clippy::expect_used)]
pub fn run() {
    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default();

    #[cfg(feature = "devtools")]
    {
        builder = builder.plugin(tauri_plugin_devtools::init());
    }

    builder
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::SIZE
                        | tauri_plugin_window_state::StateFlags::POSITION
                        | tauri_plugin_window_state::StateFlags::MAXIMIZED,
                )
                .build(),
        )
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Focus existing window when second instance launched
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_websocket::init())
        .setup(|app| {
            tray::setup_tray(app)?;
            commands::boot_gateway(app)?;

            // Background update check after app fully initializes
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                commands::check_update_background(handle).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::show_window,
            commands::get_app_version,
            commands::get_boot_status,
            commands::open_data_dir,
            commands::open_config_file,
            commands::show_notification,
            commands::check_for_update,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let app_handle = window.app_handle();

                // Attempt graceful gateway shutdown before exiting
                if let Some(state) =
                    app_handle.try_state::<Arc<tokio::sync::Mutex<commands::GatewayState>>>()
                {
                    // Use try_lock to avoid blocking the UI thread indefinitely.
                    // The inner lock is a tokio::sync::Mutex, but we can try_lock
                    // synchronously here since we're in a sync callback.
                    if let Ok(mut guard) = state.try_lock()
                        && let Some(tx) = guard.shutdown_tx.take()
                    {
                        info!("Sending gateway shutdown signal");
                        let _ = tx.send(());
                    }

                    // Brief wait for WAL checkpoint and cleanup
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }

                app_handle.exit(0);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
