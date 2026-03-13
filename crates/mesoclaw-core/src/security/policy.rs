use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Instant;

use parking_lot::Mutex;

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// The level of autonomy granted to the agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AutonomyLevel {
    ReadOnly,
    Supervised,
    Full,
}

/// Risk classification for a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// Result of a security validation check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    Allowed,
    NeedsApproval,
    Denied(String),
}

/// A single entry in the security audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub action: String,
    pub result: String,
    pub timestamp: String,
}

/// Sliding-window rate limiter.
struct RateLimiter {
    timestamps: VecDeque<Instant>,
    max_requests: u32,
    window_secs: u64,
}

impl RateLimiter {
    fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            timestamps: VecDeque::new(),
            max_requests,
            window_secs,
        }
    }

    /// Returns true if the request is allowed, false if rate-limited.
    fn check_and_record(&mut self) -> bool {
        let now = Instant::now();
        let window = std::time::Duration::from_secs(self.window_secs);

        // Remove entries older than the window
        while let Some(&front) = self.timestamps.front() {
            if now.duration_since(front) > window {
                self.timestamps.pop_front();
            } else {
                break;
            }
        }

        if self.timestamps.len() >= self.max_requests as usize {
            return false;
        }

        self.timestamps.push_back(now);
        true
    }
}

/// Commands that are always denied regardless of autonomy level.
const BLOCKED_COMMANDS: &[&str] = &[
    "rm", "sudo", "chmod", "chown", "kill", "pkill", "shutdown", "reboot", "dd", "mkfs", "fdisk",
];

/// Commands classified as low risk (read-only / informational).
const LOW_RISK_COMMANDS: &[&str] = &["ls", "cat", "head", "tail", "echo", "pwd", "whoami", "date"];

/// Git subcommands classified as low risk.
const LOW_RISK_GIT: &[&str] = &["status", "log", "diff"];

/// Cargo subcommands classified as low risk.
const LOW_RISK_CARGO: &[&str] = &["check"];

/// Commands classified as medium risk (write operations).
const MEDIUM_RISK_COMMANDS: &[&str] = &["mkdir", "cp", "mv", "touch", "npm", "bun"];

/// Git subcommands classified as medium risk.
const MEDIUM_RISK_GIT: &[&str] = &["add", "commit"];

/// Cargo subcommands classified as medium risk.
const MEDIUM_RISK_CARGO: &[&str] = &["build", "test", "run"];

/// Shell injection patterns that are always denied.
const INJECTION_PATTERNS: &[&str] = &[
    "`", "$(", "|", ";", "&&", "||", ">", ">>", "\n", "<(", ">(", "<<",
];

/// Security policy engine that validates commands and paths based on autonomy level.
pub struct SecurityPolicy {
    pub autonomy_level: AutonomyLevel,
    pub workspace_root: Option<PathBuf>,
    pub blocked_dirs: Vec<PathBuf>,
    rate_limiter: Mutex<RateLimiter>,
    audit_log: Mutex<VecDeque<AuditEntry>>,
    audit_capacity: usize,
}

impl SecurityPolicy {
    pub fn new(
        autonomy_level: AutonomyLevel,
        workspace_root: Option<PathBuf>,
        blocked_dirs: Vec<PathBuf>,
        rate_limit_max: u32,
        rate_limit_window_secs: u64,
        audit_capacity: usize,
    ) -> Self {
        Self {
            autonomy_level,
            workspace_root,
            blocked_dirs,
            rate_limiter: Mutex::new(RateLimiter::new(rate_limit_max, rate_limit_window_secs)),
            audit_log: Mutex::new(VecDeque::with_capacity(audit_capacity)),
            audit_capacity,
        }
    }

    /// Create a default policy: Supervised, common blocked dirs, 60 req/60s, 1000 audit entries.
    pub fn default_policy() -> Self {
        let blocked_dirs = vec![
            PathBuf::from("/etc"),
            PathBuf::from("/boot"),
            PathBuf::from("/sys"),
        ];
        Self::new(AutonomyLevel::Supervised, None, blocked_dirs, 60, 60, 1000)
    }

    /// Classify the risk level of a shell command.
    pub fn classify_command_risk(&self, command: &str) -> RiskLevel {
        let trimmed = command.trim();
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.is_empty() {
            return RiskLevel::High;
        }

        let base_cmd = parts[0];

        // Blocked commands are always High risk
        if BLOCKED_COMMANDS.contains(&base_cmd) {
            return RiskLevel::High;
        }

        // Check low-risk simple commands
        if LOW_RISK_COMMANDS.contains(&base_cmd) {
            return RiskLevel::Low;
        }

        // Check git subcommands
        if base_cmd == "git" {
            if let Some(sub) = parts.get(1) {
                if LOW_RISK_GIT.contains(sub) {
                    return RiskLevel::Low;
                }
                if MEDIUM_RISK_GIT.contains(sub) {
                    return RiskLevel::Medium;
                }
            }
            return RiskLevel::High;
        }

        // Check cargo subcommands
        if base_cmd == "cargo" {
            if let Some(sub) = parts.get(1) {
                if LOW_RISK_CARGO.contains(sub) {
                    return RiskLevel::Low;
                }
                if MEDIUM_RISK_CARGO.contains(sub) {
                    return RiskLevel::Medium;
                }
            }
            return RiskLevel::High;
        }

        // Check medium-risk simple commands
        if MEDIUM_RISK_COMMANDS.contains(&base_cmd) {
            return RiskLevel::Medium;
        }

        // Everything else is high risk
        RiskLevel::High
    }

    /// Validate whether a command is allowed under the current policy.
    pub fn validate_command(&self, command: &str) -> ValidationResult {
        let trimmed = command.trim();

        // Check injection patterns first
        for pattern in INJECTION_PATTERNS {
            if trimmed.contains(pattern) {
                return ValidationResult::Denied(format!("injection pattern detected: {pattern}"));
            }
        }

        // Check ENV=value prefix pattern (e.g., "FOO=bar evil_cmd")
        if let Some(first_token) = trimmed.split_whitespace().next()
            && first_token.contains('=')
            && trimmed.split_whitespace().count() > 1
        {
            return ValidationResult::Denied(
                "environment variable prefix pattern not allowed".to_string(),
            );
        }

        // Check blocked commands
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if let Some(&base_cmd) = parts.first()
            && BLOCKED_COMMANDS.contains(&base_cmd)
        {
            return ValidationResult::Denied(format!("command is blocked: {base_cmd}"));
        }

        // Check rate limit
        {
            let mut limiter = self.rate_limiter.lock();
            if !limiter.check_and_record() {
                return ValidationResult::Denied("rate limited".to_string());
            }
        }

        // Classify and apply autonomy rules
        let risk = self.classify_command_risk(trimmed);

        match self.autonomy_level {
            AutonomyLevel::ReadOnly => match risk {
                RiskLevel::Low => ValidationResult::Allowed,
                RiskLevel::Medium | RiskLevel::High => {
                    ValidationResult::Denied("read-only mode".to_string())
                }
            },
            AutonomyLevel::Supervised => match risk {
                RiskLevel::Low => ValidationResult::Allowed,
                RiskLevel::Medium => ValidationResult::NeedsApproval,
                RiskLevel::High => {
                    ValidationResult::Denied("high risk in supervised mode".to_string())
                }
            },
            AutonomyLevel::Full => match risk {
                RiskLevel::Low | RiskLevel::Medium => ValidationResult::Allowed,
                RiskLevel::High => ValidationResult::NeedsApproval,
            },
        }
    }

    /// Validate whether a file path is allowed under the current policy.
    ///
    /// When the path exists on disk, canonicalize it first to resolve symlinks
    /// and prevent symlink-based traversal attacks. Falls back to the raw path
    /// for write operations to paths that don't exist yet.
    pub fn validate_path(&self, path: &Path) -> ValidationResult {
        let path_str = path.to_string_lossy();

        // Null bytes
        if path_str.contains('\0') {
            return ValidationResult::Denied("null byte in path".to_string());
        }

        // Path traversal
        for component in path.components() {
            if let std::path::Component::ParentDir = component {
                return ValidationResult::Denied("path traversal not allowed".to_string());
            }
        }

        // Canonicalize if path exists to resolve symlinks
        let effective_path = if path.exists() {
            match std::fs::canonicalize(path) {
                Ok(canonical) => canonical,
                Err(_) => path.to_path_buf(),
            }
        } else {
            path.to_path_buf()
        };

        // Blocked directories (check canonical path)
        for blocked in &self.blocked_dirs {
            if effective_path.starts_with(blocked) {
                return ValidationResult::Denied(format!(
                    "path is in blocked directory: {}",
                    blocked.display()
                ));
            }
        }

        // Workspace root enforcement (check canonical path)
        if let Some(root) = &self.workspace_root
            && !effective_path.starts_with(root)
        {
            return ValidationResult::Denied(format!(
                "path is outside workspace root: {}",
                root.display()
            ));
        }

        ValidationResult::Allowed
    }

    /// Validate whether a tool execution is allowed under the current policy.
    ///
    /// Logs the action to the audit log and returns `ValidationResult::Denied`
    /// if the autonomy level is `ReadOnly` and the tool performs write operations.
    pub fn validate_tool_execution(
        &self,
        tool_name: &str,
        _args: &serde_json::Value,
    ) -> ValidationResult {
        // Write-oriented tools that are restricted in ReadOnly mode
        const WRITE_TOOLS: &[&str] = &[
            "file_write",
            "shell",
            "patch",
            "channel_send",
            "scheduler",
            "config",
            "memory",
        ];

        let is_write_tool = WRITE_TOOLS.contains(&tool_name);

        let result = match self.autonomy_level {
            AutonomyLevel::ReadOnly if is_write_tool => ValidationResult::Denied(format!(
                "tool '{tool_name}' requires write access, denied in read-only mode"
            )),
            AutonomyLevel::Supervised if is_write_tool => ValidationResult::NeedsApproval,
            _ => ValidationResult::Allowed,
        };

        let result_str = match &result {
            ValidationResult::Allowed => "allowed",
            ValidationResult::NeedsApproval => "needs_approval",
            ValidationResult::Denied(_) => "denied",
        };
        self.log_action(&format!("tool_execute:{tool_name}"), result_str);

        result
    }

    /// Record an action in the audit log.
    pub fn log_action(&self, action: &str, result: &str) {
        let entry = AuditEntry {
            action: action.to_string(),
            result: result.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        };

        let mut log = self.audit_log.lock();
        if log.len() >= self.audit_capacity {
            log.pop_front();
        }
        log.push_back(entry);
    }

    /// Return a copy of the audit log entries.
    pub fn audit_log(&self) -> Vec<AuditEntry> {
        let log = self.audit_log.lock();
        log.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn supervised_policy() -> SecurityPolicy {
        SecurityPolicy::default_policy()
    }

    fn readonly_policy() -> SecurityPolicy {
        SecurityPolicy::new(AutonomyLevel::ReadOnly, None, vec![], 60, 60, 100)
    }

    fn full_policy() -> SecurityPolicy {
        SecurityPolicy::new(AutonomyLevel::Full, None, vec![], 60, 60, 100)
    }

    // --- Risk classification ---

    #[test]
    fn classify_low_risk_commands() {
        let policy = supervised_policy();
        assert_eq!(policy.classify_command_risk("ls -la"), RiskLevel::Low);
        assert_eq!(policy.classify_command_risk("cat file.txt"), RiskLevel::Low);
        assert_eq!(policy.classify_command_risk("pwd"), RiskLevel::Low);
        assert_eq!(policy.classify_command_risk("whoami"), RiskLevel::Low);
        assert_eq!(policy.classify_command_risk("echo hello"), RiskLevel::Low);
    }

    #[test]
    fn classify_low_risk_git() {
        let policy = supervised_policy();
        assert_eq!(policy.classify_command_risk("git status"), RiskLevel::Low);
        assert_eq!(policy.classify_command_risk("git log"), RiskLevel::Low);
        assert_eq!(
            policy.classify_command_risk("git diff HEAD"),
            RiskLevel::Low
        );
    }

    #[test]
    fn classify_low_risk_cargo() {
        let policy = supervised_policy();
        assert_eq!(policy.classify_command_risk("cargo check"), RiskLevel::Low);
    }

    #[test]
    fn classify_medium_risk_commands() {
        let policy = supervised_policy();
        assert_eq!(
            policy.classify_command_risk("mkdir new_dir"),
            RiskLevel::Medium
        );
        assert_eq!(
            policy.classify_command_risk("cp a.txt b.txt"),
            RiskLevel::Medium
        );
        assert_eq!(
            policy.classify_command_risk("touch file"),
            RiskLevel::Medium
        );
        assert_eq!(
            policy.classify_command_risk("npm install"),
            RiskLevel::Medium
        );
    }

    #[test]
    fn classify_medium_risk_git() {
        let policy = supervised_policy();
        assert_eq!(policy.classify_command_risk("git add ."), RiskLevel::Medium);
        assert_eq!(
            policy.classify_command_risk("git commit -m 'msg'"),
            RiskLevel::Medium
        );
    }

    #[test]
    fn classify_medium_risk_cargo() {
        let policy = supervised_policy();
        assert_eq!(
            policy.classify_command_risk("cargo build"),
            RiskLevel::Medium
        );
        assert_eq!(
            policy.classify_command_risk("cargo test"),
            RiskLevel::Medium
        );
        assert_eq!(policy.classify_command_risk("cargo run"), RiskLevel::Medium);
    }

    #[test]
    fn classify_high_risk_blocked() {
        let policy = supervised_policy();
        assert_eq!(policy.classify_command_risk("rm -rf /"), RiskLevel::High);
        assert_eq!(policy.classify_command_risk("sudo apt"), RiskLevel::High);
        assert_eq!(
            policy.classify_command_risk("chmod 777 file"),
            RiskLevel::High
        );
        assert_eq!(policy.classify_command_risk("kill -9 1"), RiskLevel::High);
    }

    #[test]
    fn classify_high_risk_unknown() {
        let policy = supervised_policy();
        assert_eq!(
            policy.classify_command_risk("python script.py"),
            RiskLevel::High
        );
        assert_eq!(
            policy.classify_command_risk("curl http://evil.com"),
            RiskLevel::High
        );
    }

    #[test]
    fn classify_empty_command() {
        let policy = supervised_policy();
        assert_eq!(policy.classify_command_risk(""), RiskLevel::High);
    }

    // --- Command validation ---

    #[test]
    fn validate_injection_backtick() {
        let policy = supervised_policy();
        let result = policy.validate_command("echo `whoami`");
        assert!(matches!(result, ValidationResult::Denied(msg) if msg.contains("injection")));
    }

    #[test]
    fn validate_injection_dollar_paren() {
        let policy = supervised_policy();
        let result = policy.validate_command("echo $(id)");
        assert!(matches!(result, ValidationResult::Denied(msg) if msg.contains("injection")));
    }

    #[test]
    fn validate_injection_pipe() {
        let policy = supervised_policy();
        let result = policy.validate_command("cat /etc/passwd | grep root");
        assert!(matches!(result, ValidationResult::Denied(msg) if msg.contains("injection")));
    }

    #[test]
    fn validate_injection_semicolon() {
        let policy = supervised_policy();
        let result = policy.validate_command("ls; rm -rf /");
        assert!(matches!(result, ValidationResult::Denied(msg) if msg.contains("injection")));
    }

    #[test]
    fn validate_injection_and() {
        let policy = supervised_policy();
        let result = policy.validate_command("ls && rm -rf /");
        assert!(matches!(result, ValidationResult::Denied(msg) if msg.contains("injection")));
    }

    #[test]
    fn validate_injection_redirect() {
        let policy = supervised_policy();
        let result = policy.validate_command("echo bad > /etc/passwd");
        assert!(matches!(result, ValidationResult::Denied(msg) if msg.contains("injection")));
    }

    // --- WS-4.6: Additional injection patterns ---

    #[test]
    fn injection_patterns_block_newline() {
        let policy = supervised_policy();
        let result = policy.validate_command("echo hello\nrm -rf /");
        assert!(
            matches!(result, ValidationResult::Denied(msg) if msg.contains("injection")),
            "Newline injection should be blocked"
        );
    }

    #[test]
    fn injection_patterns_block_process_substitution() {
        let policy = supervised_policy();
        let result = policy.validate_command("cat <(echo secret)");
        assert!(
            matches!(result, ValidationResult::Denied(msg) if msg.contains("injection")),
            "Process substitution <() should be blocked"
        );
    }

    #[test]
    fn injection_patterns_block_heredoc() {
        let policy = supervised_policy();
        let result = policy.validate_command("cat <<EOF\ndata\nEOF");
        assert!(
            matches!(result, ValidationResult::Denied(msg) if msg.contains("injection")),
            "Heredoc << should be blocked"
        );
    }

    #[test]
    fn injection_patterns_block_output_process_substitution() {
        let policy = supervised_policy();
        let result = policy.validate_command("tee >(cat)");
        assert!(
            matches!(result, ValidationResult::Denied(msg) if msg.contains("injection")),
            "Process substitution >() should be blocked"
        );
    }

    #[test]
    fn validate_env_prefix_blocked() {
        let policy = supervised_policy();
        let result = policy.validate_command("FOO=bar evil_cmd");
        assert!(matches!(result, ValidationResult::Denied(msg) if msg.contains("environment")));
    }

    #[test]
    fn validate_blocked_command_rm() {
        let policy = full_policy();
        let result = policy.validate_command("rm file.txt");
        assert!(matches!(result, ValidationResult::Denied(msg) if msg.contains("blocked")));
    }

    #[test]
    fn validate_blocked_command_sudo() {
        let policy = full_policy();
        let result = policy.validate_command("sudo apt install");
        assert!(matches!(result, ValidationResult::Denied(msg) if msg.contains("blocked")));
    }

    #[test]
    fn validate_readonly_low_risk() {
        let policy = readonly_policy();
        assert_eq!(policy.validate_command("ls"), ValidationResult::Allowed);
        assert_eq!(policy.validate_command("pwd"), ValidationResult::Allowed);
    }

    #[test]
    fn validate_readonly_medium_denied() {
        let policy = readonly_policy();
        assert!(matches!(
            policy.validate_command("mkdir foo"),
            ValidationResult::Denied(_)
        ));
    }

    #[test]
    fn validate_supervised_low_allowed() {
        let policy = supervised_policy();
        assert_eq!(policy.validate_command("ls -la"), ValidationResult::Allowed);
    }

    #[test]
    fn validate_supervised_medium_needs_approval() {
        let policy = supervised_policy();
        assert_eq!(
            policy.validate_command("cargo build"),
            ValidationResult::NeedsApproval
        );
    }

    #[test]
    fn validate_supervised_high_denied() {
        let policy = supervised_policy();
        assert!(matches!(
            policy.validate_command("python script.py"),
            ValidationResult::Denied(_)
        ));
    }

    #[test]
    fn validate_full_low_allowed() {
        let policy = full_policy();
        assert_eq!(policy.validate_command("ls"), ValidationResult::Allowed);
    }

    #[test]
    fn validate_full_medium_allowed() {
        let policy = full_policy();
        assert_eq!(
            policy.validate_command("cargo build"),
            ValidationResult::Allowed
        );
    }

    #[test]
    fn validate_full_high_needs_approval() {
        let policy = full_policy();
        assert_eq!(
            policy.validate_command("python script.py"),
            ValidationResult::NeedsApproval
        );
    }

    // --- Rate limiting ---

    #[test]
    fn rate_limit_denies_when_exceeded() {
        let policy = SecurityPolicy::new(AutonomyLevel::Full, None, vec![], 3, 60, 100);
        assert_eq!(policy.validate_command("ls"), ValidationResult::Allowed);
        assert_eq!(policy.validate_command("ls"), ValidationResult::Allowed);
        assert_eq!(policy.validate_command("ls"), ValidationResult::Allowed);
        assert!(matches!(
            policy.validate_command("ls"),
            ValidationResult::Denied(msg) if msg.contains("rate limited")
        ));
    }

    // --- Path validation ---

    #[test]
    fn validate_path_null_byte() {
        let policy = supervised_policy();
        let path = PathBuf::from("/tmp/file\0evil");
        assert!(matches!(
            policy.validate_path(&path),
            ValidationResult::Denied(msg) if msg.contains("null byte")
        ));
    }

    #[test]
    fn validate_path_traversal() {
        let policy = supervised_policy();
        let path = PathBuf::from("/home/user/../../../etc/passwd");
        assert!(matches!(
            policy.validate_path(&path),
            ValidationResult::Denied(msg) if msg.contains("traversal")
        ));
    }

    #[test]
    fn validate_path_blocked_dir() {
        let policy = supervised_policy();
        let path = PathBuf::from("/etc/passwd");
        assert!(matches!(
            policy.validate_path(&path),
            ValidationResult::Denied(msg) if msg.contains("blocked directory")
        ));
    }

    #[test]
    fn validate_path_blocked_boot() {
        let policy = supervised_policy();
        let path = PathBuf::from("/boot/vmlinuz");
        assert!(matches!(
            policy.validate_path(&path),
            ValidationResult::Denied(msg) if msg.contains("blocked directory")
        ));
    }

    #[test]
    fn validate_path_outside_workspace() {
        let policy = SecurityPolicy::new(
            AutonomyLevel::Supervised,
            Some(PathBuf::from("/home/user/project")),
            vec![],
            60,
            60,
            100,
        );
        let path = PathBuf::from("/tmp/malicious");
        assert!(matches!(
            policy.validate_path(&path),
            ValidationResult::Denied(msg) if msg.contains("outside workspace")
        ));
    }

    #[test]
    fn validate_path_inside_workspace() {
        let policy = SecurityPolicy::new(
            AutonomyLevel::Supervised,
            Some(PathBuf::from("/home/user/project")),
            vec![],
            60,
            60,
            100,
        );
        let path = PathBuf::from("/home/user/project/src/main.rs");
        assert_eq!(policy.validate_path(&path), ValidationResult::Allowed);
    }

    #[test]
    fn validate_path_allowed_no_constraints() {
        let policy = SecurityPolicy::new(AutonomyLevel::Full, None, vec![], 60, 60, 100);
        let path = PathBuf::from("/tmp/some_file.txt");
        assert_eq!(policy.validate_path(&path), ValidationResult::Allowed);
    }

    // --- Symlink traversal (WS-4.4) ---

    #[test]
    fn symlink_traversal_blocked() {
        let tmp = tempfile::tempdir().unwrap();
        let link = tmp.path().join("innocent");
        std::os::unix::fs::symlink("/etc/passwd", &link).unwrap();
        let policy = SecurityPolicy::default_policy();
        let result = policy.validate_path(&link);
        assert!(
            matches!(&result, ValidationResult::Denied(msg) if msg.contains("blocked")),
            "Symlink to blocked path should be rejected, got: {result:?}"
        );
    }

    #[test]
    fn non_symlink_path_still_allowed() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("normal.txt");
        std::fs::write(&file, "hello").unwrap();
        // Use a policy with no blocked dirs to test normal path
        let policy = SecurityPolicy::new(AutonomyLevel::Full, None, vec![], 60, 60, 100);
        assert_eq!(policy.validate_path(&file), ValidationResult::Allowed);
    }

    // --- Audit log ---

    #[test]
    fn audit_log_records_entries() {
        let policy = supervised_policy();
        policy.log_action("run ls", "allowed");
        policy.log_action("run rm", "denied");

        let log = policy.audit_log();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].action, "run ls");
        assert_eq!(log[0].result, "allowed");
        assert_eq!(log[1].action, "run rm");
        assert_eq!(log[1].result, "denied");
    }

    #[test]
    fn audit_log_respects_capacity() {
        let policy = SecurityPolicy::new(AutonomyLevel::Supervised, None, vec![], 60, 60, 3);
        policy.log_action("a", "1");
        policy.log_action("b", "2");
        policy.log_action("c", "3");
        policy.log_action("d", "4");

        let log = policy.audit_log();
        assert_eq!(log.len(), 3);
        assert_eq!(log[0].action, "b");
        assert_eq!(log[2].action, "d");
    }

    #[test]
    fn audit_entry_has_timestamp() {
        let policy = supervised_policy();
        policy.log_action("test", "ok");
        let log = policy.audit_log();
        assert!(!log[0].timestamp.is_empty());
        assert!(log[0].timestamp.contains('T'));
    }

    // --- Default policy ---

    #[test]
    fn default_policy_is_supervised() {
        let policy = SecurityPolicy::default_policy();
        assert_eq!(policy.autonomy_level, AutonomyLevel::Supervised);
        assert!(policy.workspace_root.is_none());
        assert_eq!(policy.blocked_dirs.len(), 3);
    }

    // --- Serde ---

    #[test]
    fn autonomy_level_serde_roundtrip() {
        let level = AutonomyLevel::Supervised;
        let json = serde_json::to_string(&level).unwrap();
        let back: AutonomyLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(back, level);
    }

    #[test]
    fn risk_level_serde_roundtrip() {
        let risk = RiskLevel::Medium;
        let json = serde_json::to_string(&risk).unwrap();
        let back: RiskLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(back, risk);
    }

    // --- Tool execution validation (WS-4.3) ---

    #[test]
    fn tool_execution_readonly_denies_write_tools() {
        let policy = readonly_policy();
        let args = serde_json::json!({});
        let result = policy.validate_tool_execution("file_write", &args);
        assert!(
            matches!(result, ValidationResult::Denied(msg) if msg.contains("read-only")),
            "Write tool should be denied in read-only mode"
        );
    }

    #[test]
    fn tool_execution_readonly_allows_read_tools() {
        let policy = readonly_policy();
        let args = serde_json::json!({});
        assert_eq!(
            policy.validate_tool_execution("file_read", &args),
            ValidationResult::Allowed
        );
        assert_eq!(
            policy.validate_tool_execution("system_info", &args),
            ValidationResult::Allowed
        );
    }

    #[test]
    fn tool_execution_supervised_needs_approval_for_write() {
        let policy = supervised_policy();
        let args = serde_json::json!({});
        assert_eq!(
            policy.validate_tool_execution("shell", &args),
            ValidationResult::NeedsApproval
        );
    }

    #[test]
    fn tool_execution_full_allows_all() {
        let policy = full_policy();
        let args = serde_json::json!({});
        assert_eq!(
            policy.validate_tool_execution("file_write", &args),
            ValidationResult::Allowed
        );
        assert_eq!(
            policy.validate_tool_execution("shell", &args),
            ValidationResult::Allowed
        );
    }

    #[test]
    fn tool_execution_logs_to_audit() {
        let policy = full_policy();
        let args = serde_json::json!({});
        policy.validate_tool_execution("file_read", &args);
        let log = policy.audit_log();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].action, "tool_execute:file_read");
        assert_eq!(log[0].result, "allowed");
    }

    // WS-6.5 — parking_lot::Mutex does not poison after panic
    #[test]
    fn security_policy_mutex_no_poison() {
        let policy = std::sync::Arc::new(SecurityPolicy::default_policy());
        let p = std::sync::Arc::clone(&policy);
        let _ = std::thread::spawn(move || {
            let _guard = p.rate_limiter.lock();
            panic!("test panic");
        })
        .join();
        // After panic, lock should still be acquirable (parking_lot doesn't poison)
        let _guard = policy.rate_limiter.lock();
        // Also verify audit_log lock is not poisoned
        policy.log_action("post_panic", "ok");
        let log = policy.audit_log();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].action, "post_panic");
    }
}
