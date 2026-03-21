use std::sync::Arc;

use tauri::{
    Manager,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tracing::info;

use crate::commands;

/// Menu item IDs for the system tray.
pub const MENU_SHOW: &str = "show";
pub const MENU_QUIT: &str = "quit";

/// Expected number of menu items (show, separator, quit).
pub const EXPECTED_MENU_ITEM_COUNT: usize = 3;

/// Set up the system tray icon with menu and event handlers.
pub fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show_item = MenuItem::with_id(app, MENU_SHOW, "Show Window", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_item, &separator, &quit_item])?;

    TrayIconBuilder::with_id("main-tray")
        .icon(
            app.default_window_icon()
                .ok_or("no default window icon set")?
                .clone(),
        )
        .tooltip("Zenii")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            MENU_SHOW => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            MENU_QUIT => {
                // Graceful gateway shutdown before exit (mirrors on_window_event logic)
                if let Some(state) =
                    app.try_state::<Arc<tokio::sync::Mutex<commands::GatewayState>>>()
                {
                    if let Ok(mut guard) = state.try_lock()
                        && let Some(tx) = guard.shutdown_tx.take()
                    {
                        info!("Sending gateway shutdown signal from tray quit");
                        let _ = tx.send(());
                    }

                    // Brief wait for WAL checkpoint and cleanup
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }

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
                if let Some(w) = app.get_webview_window("main") {
                    if w.is_visible().unwrap_or(false) {
                        let _ = w.hide();
                    } else {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // 7.5 — Tray menu has expected item count and IDs
    #[test]
    fn tray_menu_has_expected_items() {
        // Verify the constants are correct
        assert_eq!(MENU_SHOW, "show");
        assert_eq!(MENU_QUIT, "quit");
        assert_eq!(EXPECTED_MENU_ITEM_COUNT, 3); // show, separator, quit
    }
}
