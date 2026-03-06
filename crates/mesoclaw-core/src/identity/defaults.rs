pub const DEFAULT_SOUL: &str = include_str!("defaults/SOUL.md");
pub const DEFAULT_IDENTITY: &str = include_str!("defaults/IDENTITY.md");
pub const DEFAULT_USER: &str = include_str!("defaults/USER.md");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_soul_not_empty() {
        assert!(!DEFAULT_SOUL.is_empty());
        assert!(DEFAULT_SOUL.contains("MesoClaw"));
    }

    #[test]
    fn bundled_identity_not_empty() {
        assert!(!DEFAULT_IDENTITY.is_empty());
        assert!(DEFAULT_IDENTITY.contains("---"));
    }

    #[test]
    fn bundled_user_not_empty() {
        assert!(!DEFAULT_USER.is_empty());
        assert!(DEFAULT_USER.contains("User Context"));
    }
}
