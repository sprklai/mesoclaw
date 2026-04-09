// Gate 2 TDD: wiki_dir config field tests.
//
// These tests will NOT compile until `wiki_dir: Option<String>` is added to
// `AppConfig` in `schema.rs` (Gate 3 implementation step).  That is intentional —
// Gate 2 specifies the contract; the implementation makes it green.
//
// To activate these tests, add the following line to `config/mod.rs`:
//   mod wiki_config_tests;

#[cfg(test)]
mod wiki_config_tests {
    use crate::config::schema::AppConfig;

    #[test]
    fn wiki_dir_field_is_none_by_default() {
        let config = AppConfig::default();
        // wiki_dir should be None; the resolved path falls back to {data_dir}/wiki/
        assert!(config.wiki_dir.is_none());
    }

    #[test]
    fn wiki_dir_override_can_be_set() {
        let config = AppConfig {
            wiki_dir: Some("/tmp/my-wiki".into()),
            ..Default::default()
        };
        assert_eq!(config.wiki_dir.as_deref(), Some("/tmp/my-wiki"));
    }
}
