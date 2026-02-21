// @generated automatically by Diesel CLI.

diesel::table! {
    agent_runs (id) {
        id -> Text,
        session_id -> Text,
        agent_id -> Text,
        parent_run_id -> Nullable<Text>,
        status -> Text,
        input_message -> Text,
        output_message -> Nullable<Text>,
        error_message -> Nullable<Text>,
        tokens_used -> Nullable<Integer>,
        duration_ms -> Nullable<Integer>,
        started_at -> Nullable<Text>,
        completed_at -> Nullable<Text>,
        created_at -> Text,
    }
}

diesel::table! {
    agent_sessions (id) {
        id -> Text,
        agent_id -> Text,
        name -> Text,
        status -> Text,
        created_at -> Text,
        updated_at -> Text,
        completed_at -> Nullable<Text>,
    }
}

diesel::table! {
    agents (id) {
        id -> Text,
        name -> Text,
        description -> Nullable<Text>,
        system_prompt -> Text,
        model_id -> Text,
        provider_id -> Text,
        temperature -> Float,
        max_tokens -> Nullable<Integer>,
        tools_enabled -> Integer,
        memory_enabled -> Integer,
        workspace_path -> Nullable<Text>,
        is_active -> Integer,
        created_at -> Text,
        updated_at -> Text,
    }
}

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
    generated_prompts (id) {
        id -> Text,
        name -> Text,
        artifact_type -> Text,
        content -> Text,
        disk_path -> Nullable<Text>,
        created_at -> Timestamp,
        provider_id -> Nullable<Text>,
        model_id -> Nullable<Text>,
    }
}

diesel::table! {
    memories (id) {
        id -> Text,
        key -> Text,
        content -> Text,
        category -> Text,
        embedding -> Nullable<Binary>,
        score -> Float,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    memories_fts (rowid) {
        rowid -> Integer,
        content -> Nullable<Binary>,
        memories_fts -> Nullable<Binary>,
        rank -> Nullable<Binary>,
    }
}

diesel::table! {
    memories_fts_config (k) {
        k -> Binary,
        v -> Nullable<Binary>,
    }
}

diesel::table! {
    memories_fts_data (id) {
        id -> Nullable<Integer>,
        block -> Nullable<Binary>,
    }
}

diesel::table! {
    memories_fts_docsize (id) {
        id -> Nullable<Integer>,
        sz -> Nullable<Binary>,
    }
}

diesel::table! {
    memories_fts_idx (segid, term) {
        segid -> Binary,
        term -> Binary,
        pgno -> Nullable<Binary>,
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
        dnd_schedule_enabled -> Integer,
        dnd_start_hour -> Integer,
        dnd_end_hour -> Integer,
        notify_heartbeat -> Integer,
        notify_cron_reminder -> Integer,
        notify_agent_complete -> Integer,
        notify_approval_request -> Integer,
        skill_auto_select -> Integer,
        skill_enabled_ids -> Text,
        user_name -> Nullable<Text>,
        app_display_name -> Nullable<Text>,
    }
}

diesel::table! {
    tool_audit_log (id) {
        id -> Integer,
        timestamp -> Text,
        session_id -> Nullable<Text>,
        tool_name -> Text,
        args -> Text,
        risk_level -> Text,
        decision -> Text,
        result -> Nullable<Text>,
        success -> Integer,
    }
}

diesel::joinable!(agent_runs -> agent_sessions (session_id));
diesel::joinable!(agent_runs -> agents (agent_id));
diesel::joinable!(agent_sessions -> agents (agent_id));
diesel::joinable!(agents -> ai_providers (provider_id));
diesel::joinable!(ai_models -> ai_providers (provider_id));
diesel::joinable!(chat_messages -> chat_sessions (session_id));

diesel::allow_tables_to_appear_in_same_query!(
    agent_runs,
    agent_sessions,
    agents,
    ai_models,
    ai_providers,
    chat_messages,
    chat_sessions,
    generated_prompts,
    memories,
    memories_fts,
    memories_fts_config,
    memories_fts_data,
    memories_fts_docsize,
    memories_fts_idx,
    scheduled_jobs,
    settings,
    tool_audit_log,
);
