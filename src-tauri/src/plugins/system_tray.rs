use tauri::{
    App, Manager,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
};

use crate::config::app_identity::TRAY_TOOLTIP;
use crate::{database::DbPool, services::settings::get_settings};

/// Setup the system tray, apply initial visibility based on settings,
/// and store the tray icon in app state for later management.
pub fn setup(app: &App, pool: &DbPool) -> Result<(), Box<dyn std::error::Error>> {
    // Create menu items
    let show_i = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let hide_i = MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    // Create the tray menu
    let menu = Menu::with_items(app, &[&show_i, &hide_i, &quit_i])?;

    // Build the tray icon
    let tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip(TRAY_TOOLTIP)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "hide" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    // Apply initial visibility based on settings
    apply_settings_visibility(&tray, pool);

    // Store tray icon in app state for later management
    app.manage(tray);

    Ok(())
}

/// Apply tray visibility based on current settings
fn apply_settings_visibility(tray: &TrayIcon, pool: &DbPool) {
    if let Ok(mut conn) = pool.get()
        && let Ok(settings) = get_settings(&mut conn)
        && !settings.show_in_tray
    {
        let _ = tray.set_visible(false);
    }
}
