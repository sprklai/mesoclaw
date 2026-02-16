use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn close_splashscreen(app: AppHandle) -> Result<(), String> {
    if let Some(splashscreen) = app.get_webview_window("splashscreen") {
        splashscreen.close().map_err(|e| e.to_string())?;
    }

    if let Some(main) = app.get_webview_window("main") {
        // Center the main window to match the splash screen position
        main.center().map_err(|e| e.to_string())?;
        main.show().map_err(|e| e.to_string())?;
        main.set_focus().map_err(|e| e.to_string())?;
    }

    Ok(())
}
