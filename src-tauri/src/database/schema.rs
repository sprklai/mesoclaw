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
    chat_messages (id) {
        id -> Text,
        session_id -> Text,
        role -> Text,
        content -> Text,
        created_at -> Text,
    }
}

diesel::table! {
    chat_session_tags (session_id, tag_id) {
        session_id -> Text,
        tag_id -> Text,
    }
}

diesel::table! {
    chat_sessions (id) {
        id -> Text,
        workspace_id -> Text,
        title -> Text,
        model -> Text,
        is_bookmarked -> Integer,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    chat_tags (id) {
        id -> Text,
        name -> Text,
        color -> Text,
        created_at -> Text,
    }
}

diesel::table! {
    explanations (id) {
        id -> Text,
        workspace_id -> Text,
        entity_type -> Text,
        entity_id -> Text,
        explanation -> Text,
        evidence -> Nullable<Text>,
        confidence -> Nullable<Float>,
        fingerprint -> Text,
        created_at -> Text,
    }
}

diesel::table! {
    schema_snapshots (id) {
        id -> Text,
        workspace_id -> Text,
        fingerprint -> Text,
        snapshot_data -> Text,
        created_at -> Text,
    }
}

diesel::table! {
    skill_settings (workspace_id) {
        workspace_id -> Text,
        settings_json -> Text,
        updated_at -> Text,
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

diesel::table! {
    user_annotations (id) {
        id -> Text,
        workspace_id -> Text,
        entity_id -> Text,
        annotation -> Text,
        created_at -> Text,
    }
}

diesel::table! {
    workspaces (id) {
        id -> Text,
        name -> Text,
        database_type -> Text,
        connection_config -> Text,
        created_at -> Text,
        last_accessed -> Text,
        database_summary -> Nullable<Text>,
        llm_provider_id -> Nullable<Text>,
        llm_model_id -> Nullable<Text>,
    }
}

diesel::joinable!(ai_models -> ai_providers (provider_id));
diesel::joinable!(chat_messages -> chat_sessions (session_id));
diesel::joinable!(chat_session_tags -> chat_sessions (session_id));
diesel::joinable!(chat_session_tags -> chat_tags (tag_id));
diesel::joinable!(chat_sessions -> workspaces (workspace_id));
diesel::joinable!(explanations -> workspaces (workspace_id));
diesel::joinable!(schema_snapshots -> workspaces (workspace_id));
diesel::joinable!(skill_settings -> workspaces (workspace_id));
diesel::joinable!(user_annotations -> workspaces (workspace_id));

diesel::allow_tables_to_appear_in_same_query!(
    ai_models,
    ai_providers,
    chat_messages,
    chat_session_tags,
    chat_sessions,
    chat_tags,
    explanations,
    schema_snapshots,
    settings,
    skill_settings,
    user_annotations,
    workspaces,
);
