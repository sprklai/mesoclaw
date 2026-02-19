// @generated automatically by Diesel CLI.

diesel::table! {
    ai_models (id) {
        id -> Text,
        provider_id -> Text,
        model_id -> Text,
        display_name -> Text,
        context_limit -> Nullable<Integer>,
        is_custom -> Integer,
        is_active -> Integer,
        created_at -> Text,
    }
}

diesel::table! {
    ai_providers (id) {
        id -> Text,
        name -> Text,
        base_url -> Text,
        requires_api_key -> Integer,
        is_active -> Integer,
        created_at -> Text,
        is_user_defined -> Integer,
    }
}

diesel::table! {
    chat_sessions (id) {
        id -> Text,
        session_key -> Text,
        agent -> Text,
        scope -> Text,
        channel -> Text,
        peer -> Text,
        created_at -> Text,
        updated_at -> Text,
        compaction_summary -> Nullable<Text>,
    }
}

diesel::table! {
    scheduled_jobs (id) {
        id -> Text,
        name -> Text,
        schedule_json -> Text,
        session_target -> Text,
        payload_json -> Text,
        enabled -> Integer,
        error_count -> Integer,
        next_run -> Nullable<Text>,
        created_at -> Text,
        active_hours_json -> Nullable<Text>,
        delete_after_run -> Integer,
    }
}

diesel::table! {
    settings (id) {
        id -> Integer,
        theme -> Text,
        sidebar_expanded -> Integer,
        show_in_tray -> Integer,
        launch_at_login -> Integer,
        enable_logging -> Integer,
        log_level -> Text,
        enable_notifications -> Integer,
        notify_general -> Integer,
        notify_reminders -> Integer,
        notify_updates -> Integer,
        notify_alerts -> Integer,
        notify_activity -> Integer,
        llm_model -> Text,
        use_cloud_llm -> Integer,
        explanation_verbosity -> Text,
        temperature -> Float,
        max_tokens -> Integer,
        timeout -> Integer,
        stream_responses -> Integer,
        enable_caching -> Integer,
        debug_mode -> Integer,
        custom_base_url -> Nullable<Text>,
        default_provider_id -> Nullable<Text>,
        default_model_id -> Nullable<Text>,
    }
}

diesel::joinable!(ai_models -> ai_providers (provider_id));

diesel::allow_tables_to_appear_in_same_query!(
    ai_models,
    ai_providers,
    chat_sessions,
    scheduled_jobs,
    settings,
);
