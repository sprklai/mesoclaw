use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, AppMode, ChatStatus, OnboardField, OnboardStep};

/// Handle a keyboard event, dispatching by current mode.
pub fn handle_key_event(app: &mut App, key: KeyEvent) {
    // Global bindings first
    match (key.code, key.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            app.should_quit = true;
            return;
        }
        (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
            // Refresh — handled by caller (triggers async reload)
            app.notification_text = Some("Refreshing...".into());
            return;
        }
        _ => {}
    }

    // Cancel delete confirmation on any key except 'y'
    if app.confirm_delete {
        if key.code == KeyCode::Char('y') {
            // Delete confirmed — handled by caller checking confirm_delete
            // The actual delete is performed in the main loop
        }
        app.confirm_delete = false;
        return;
    }

    match app.mode {
        AppMode::Help => match key.code {
            KeyCode::Char('?') | KeyCode::Esc => app.toggle_help(),
            _ => {}
        },
        AppMode::SessionList => handle_session_list(app, key),
        AppMode::Chat => handle_chat(app, key),
        AppMode::Input => handle_input(app, key),
        AppMode::Onboard => handle_onboard(app, key),
    }
}

fn handle_session_list(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => app.select_next_session(),
        KeyCode::Char('k') | KeyCode::Up => app.select_prev_session(),
        KeyCode::Char('g') => app.select_first_session(),
        KeyCode::Char('G') => app.select_last_session(),
        KeyCode::Enter => {
            if app.selected_session.is_some() {
                app.enter_chat_mode();
            }
        }
        KeyCode::Char('n') => {
            // New session — handled by caller
            app.notification_text = Some("__create_session__".into());
        }
        KeyCode::Char('d') => {
            if app.selected_session.is_some() {
                app.confirm_delete = true;
                app.notification_text = Some("Delete session? (y/n)".into());
            }
        }
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('?') => app.toggle_help(),
        KeyCode::Tab => {
            if app.current_session_id.is_some() {
                app.enter_chat_mode();
            }
        }
        KeyCode::Char('i') => {
            if app.current_session_id.is_some() {
                app.enter_chat_mode();
                app.enter_input_mode();
            }
        }
        _ => {}
    }
}

fn handle_chat(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => app.scroll_down(),
        KeyCode::Char('k') | KeyCode::Up => app.scroll_up(),
        KeyCode::Char('g') => app.scroll_to_top(),
        KeyCode::Char('G') => app.scroll_to_bottom(),
        KeyCode::PageUp => app.scroll_page_up(10),
        KeyCode::PageDown => app.scroll_page_down(10),
        KeyCode::Char('i') => app.enter_input_mode(),
        KeyCode::Esc | KeyCode::Tab => app.enter_session_list_mode(),
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('?') => app.toggle_help(),
        _ => {}
    }
}

fn handle_input(app: &mut App, key: KeyEvent) {
    match (key.code, key.modifiers) {
        (KeyCode::Esc, _) => app.exit_input_mode(),
        (KeyCode::Enter, _) => {
            if !app.input.content.trim().is_empty() && app.chat_status != ChatStatus::Streaming {
                // Signal send — handled by caller checking chat_status transition
                app.chat_status = ChatStatus::Streaming;
                app.notification_text = Some("__send_message__".into());
            }
        }
        (KeyCode::Char('u'), KeyModifiers::CONTROL) => app.input.clear(),
        (KeyCode::Char('w'), KeyModifiers::CONTROL) => app.input.delete_word_backward(),
        (KeyCode::Char('a'), KeyModifiers::CONTROL) => app.input.move_home(),
        (KeyCode::Char('e'), KeyModifiers::CONTROL) => app.input.move_end(),
        (KeyCode::Backspace, _) => app.input.delete_back(),
        (KeyCode::Left, _) => app.input.move_left(),
        (KeyCode::Right, _) => app.input.move_right(),
        (KeyCode::Up, _) => app.input.history_up(),
        (KeyCode::Down, _) => app.input.history_down(),
        (KeyCode::Home, _) => app.input.move_home(),
        (KeyCode::End, _) => app.input.move_end(),
        (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => app.input.insert(c),
        _ => {}
    }
}

fn handle_onboard(app: &mut App, key: KeyEvent) {
    // Allow quit from provider select step
    if key.code == KeyCode::Char('q') && app.onboard_step == OnboardStep::ProviderSelect {
        app.should_quit = true;
        return;
    }

    match app.onboard_step {
        OnboardStep::ProviderSelect => match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !app.onboard_providers.is_empty() {
                    app.onboard_selected_provider =
                        (app.onboard_selected_provider + 1).min(app.onboard_providers.len() - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.onboard_selected_provider = app.onboard_selected_provider.saturating_sub(1);
            }
            KeyCode::Enter => {
                if let Some(provider) = app.onboard_providers.get(app.onboard_selected_provider) {
                    app.onboard_provider_id = provider["id"].as_str().unwrap_or("").to_string();
                    app.onboard_requires_key =
                        provider["requires_api_key"].as_bool().unwrap_or(true);
                    app.onboard_models = provider["models"].as_array().cloned().unwrap_or_default();
                    if app.onboard_requires_key {
                        app.onboard_step = OnboardStep::ApiKey;
                    } else {
                        // Skip API key for providers that don't need it (e.g. Ollama)
                        app.onboard_selected_model = 0;
                        app.onboard_step = OnboardStep::ModelSelect;
                    }
                }
            }
            _ => {}
        },
        OnboardStep::ApiKey => match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => {
                app.onboard_step = OnboardStep::ProviderSelect;
                app.onboard_api_key.clear();
                app.onboard_error = None;
            }
            (KeyCode::Enter, _) => {
                if !app.onboard_api_key.content.trim().is_empty() {
                    // Signal to save API key — handled by main loop
                    app.notification_text = Some("__onboard_save_key__".into());
                }
            }
            (KeyCode::Backspace, _) => app.onboard_api_key.delete_back(),
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                app.onboard_api_key.insert(c);
            }
            _ => {}
        },
        OnboardStep::ModelSelect => match key.code {
            KeyCode::Esc => {
                if app.onboard_requires_key {
                    app.onboard_step = OnboardStep::ApiKey;
                } else {
                    app.onboard_step = OnboardStep::ProviderSelect;
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !app.onboard_models.is_empty() {
                    app.onboard_selected_model =
                        (app.onboard_selected_model + 1).min(app.onboard_models.len() - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.onboard_selected_model = app.onboard_selected_model.saturating_sub(1);
            }
            KeyCode::Enter => {
                // Signal to save model — handled by main loop
                app.notification_text = Some("__onboard_save_model__".into());
            }
            _ => {}
        },
        OnboardStep::Profile => match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => {
                app.onboard_step = OnboardStep::ModelSelect;
                app.onboard_error = None;
            }
            (KeyCode::Tab, _) => {
                app.onboard_field = match app.onboard_field {
                    OnboardField::Name => OnboardField::Location,
                    OnboardField::Location => OnboardField::Timezone,
                    OnboardField::Timezone => OnboardField::Name,
                };
            }
            (KeyCode::BackTab, _) => {
                app.onboard_field = match app.onboard_field {
                    OnboardField::Name => OnboardField::Timezone,
                    OnboardField::Location => OnboardField::Name,
                    OnboardField::Timezone => OnboardField::Location,
                };
            }
            (KeyCode::Enter, _) => {
                if !app.onboard_name.content.trim().is_empty()
                    && !app.onboard_location.content.trim().is_empty()
                {
                    app.notification_text = Some("__onboard_save_profile__".into());
                } else {
                    app.onboard_error = Some("Name and location are required".into());
                }
            }
            (KeyCode::Backspace, _) => match app.onboard_field {
                OnboardField::Name => app.onboard_name.delete_back(),
                OnboardField::Location => app.onboard_location.delete_back(),
                OnboardField::Timezone => app.onboard_timezone.delete_back(),
            },
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                match app.onboard_field {
                    OnboardField::Name => app.onboard_name.insert(c),
                    OnboardField::Location => app.onboard_location.insert(c),
                    OnboardField::Timezone => app.onboard_timezone.insert(c),
                }
            }
            _ => {}
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::SessionSummary;
    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn key_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    fn app_with_sessions() -> App {
        let mut app = App::new();
        for i in 0..3 {
            app.sessions.push(SessionSummary {
                id: format!("s{i}"),
                title: format!("Session {i}"),
                updated_at: String::new(),
                message_count: 0,
            });
        }
        app.selected_session = Some(0);
        app
    }

    #[test]
    fn j_moves_down_session_list() {
        let mut app = app_with_sessions();
        handle_key_event(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.selected_session, Some(1));
    }

    #[test]
    fn k_moves_up_session_list() {
        let mut app = app_with_sessions();
        app.selected_session = Some(2);
        handle_key_event(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.selected_session, Some(1));
    }

    #[test]
    fn j_scrolls_chat() {
        let mut app = App::new();
        app.mode = AppMode::Chat;
        app.scroll_offset = 0;
        handle_key_event(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.scroll_offset, 1);
    }

    #[test]
    fn enter_sends_message() {
        let mut app = App::new();
        app.mode = AppMode::Input;
        app.chat_status = ChatStatus::Composing;
        app.input.insert('h');
        app.input.insert('i');
        handle_key_event(&mut app, key(KeyCode::Enter));
        assert_eq!(app.chat_status, ChatStatus::Streaming);
    }

    #[test]
    fn ctrl_c_quits() {
        let mut app = App::new();
        handle_key_event(&mut app, key_mod(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.should_quit);
    }

    #[test]
    fn char_appends_to_input() {
        let mut app = App::new();
        app.mode = AppMode::Input;
        handle_key_event(&mut app, key(KeyCode::Char('a')));
        handle_key_event(&mut app, key(KeyCode::Char('b')));
        assert_eq!(app.input.content, "ab");
        assert_eq!(app.input.cursor_pos, 2);
    }

    #[test]
    fn backspace_deletes() {
        let mut app = App::new();
        app.mode = AppMode::Input;
        app.input.insert('a');
        app.input.insert('b');
        handle_key_event(&mut app, key(KeyCode::Backspace));
        assert_eq!(app.input.content, "a");
        assert_eq!(app.input.cursor_pos, 1);
    }

    #[test]
    fn ctrl_u_clears() {
        let mut app = App::new();
        app.mode = AppMode::Input;
        app.input.insert('h');
        app.input.insert('i');
        handle_key_event(&mut app, key_mod(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert!(app.input.content.is_empty());
        assert_eq!(app.input.cursor_pos, 0);
    }

    #[test]
    fn input_history_navigation() {
        let mut app = App::new();
        app.mode = AppMode::Input;
        app.input.history = vec!["first".into(), "second".into()];
        handle_key_event(&mut app, key(KeyCode::Up));
        assert_eq!(app.input.content, "second");
        handle_key_event(&mut app, key(KeyCode::Up));
        assert_eq!(app.input.content, "first");
        handle_key_event(&mut app, key(KeyCode::Down));
        assert_eq!(app.input.content, "second");
    }

    #[test]
    fn text_input_insert_at_cursor() {
        let mut app = App::new();
        app.mode = AppMode::Input;
        app.input.insert('a');
        app.input.insert('c');
        app.input.move_left();
        app.input.insert('b');
        assert_eq!(app.input.content, "abc");
    }

    #[test]
    fn text_input_cursor_movement() {
        let mut app = App::new();
        app.input.insert('a');
        app.input.insert('b');
        app.input.insert('c');
        assert_eq!(app.input.cursor_pos, 3);

        app.input.move_home();
        assert_eq!(app.input.cursor_pos, 0);

        app.input.move_end();
        assert_eq!(app.input.cursor_pos, 3);

        app.input.move_left();
        assert_eq!(app.input.cursor_pos, 2);

        app.input.move_right();
        assert_eq!(app.input.cursor_pos, 3);
    }

    #[test]
    fn text_input_delete_word() {
        let mut app = App::new();
        app.input.content = "hello world".into();
        app.input.cursor_pos = 11;
        app.input.delete_word_backward();
        assert_eq!(app.input.content, "hello ");
        assert_eq!(app.input.cursor_pos, 6);
    }
}
