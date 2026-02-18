use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

// ─── Public types ──────────────────────────────────────────────────────────

/// Controls which operations the agent may perform autonomously.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyLevel {
    /// Only read operations are allowed without approval.
    ReadOnly,
    /// Reads are automatic; medium/high-risk writes require user approval.
    Supervised,
    /// All operations are allowed, subject to the rate limiter.
    Full,
}

/// Risk tier of a shell command or file operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// Decision returned by `validate_command` / `validate_path`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    /// The operation may proceed immediately.
    Allowed,
    /// The operation requires explicit user approval before proceeding.
    NeedsApproval,
    /// The operation is prohibited; the reason explains why.
    Denied(String),
}

/// One entry in the immutable audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tool_name: String,
    pub args: serde_json::Value,
    pub risk_level: RiskLevel,
    pub decision: String,
    pub result: Option<String>,
}

// ─── Rate limiter ──────────────────────────────────────────────────────────

struct SlidingWindow {
    window: Duration,
    max_actions: usize,
    timestamps: Mutex<VecDeque<Instant>>,
}

impl SlidingWindow {
    fn new(window_secs: u64, max_actions: usize) -> Self {
        Self {
            window: Duration::from_secs(window_secs),
            max_actions,
            timestamps: Mutex::new(VecDeque::new()),
        }
    }

    /// Returns `true` if the action is within the rate limit and records it.
    /// Returns `false` if the limit has been exceeded.
    fn try_record(&self) -> bool {
        let mut ts = self.timestamps.lock().expect("rate-limiter lock poisoned");
        let now = Instant::now();
        // Drop expired entries.
        ts.retain(|&t| now.duration_since(t) < self.window);
        if ts.len() >= self.max_actions {
            return false;
        }
        ts.push_back(now);
        true
    }

    /// Current number of actions in the window (used by tests).
    #[cfg(test)]
    fn count(&self) -> usize {
        let mut ts = self.timestamps.lock().expect("lock");
        let now = Instant::now();
        ts.retain(|&t| now.duration_since(t) < self.window);
        ts.len()
    }
}

// ─── SecurityPolicy ────────────────────────────────────────────────────────

/// Enforces access control rules for all tool executions.
///
/// `SecurityPolicy` is designed to be shared across threads via `Arc<SecurityPolicy>`.
/// All mutable state (rate limiter, audit log) is protected by interior `Mutex`es.
pub struct SecurityPolicy {
    pub autonomy_level: AutonomyLevel,
    /// If `Some`, paths outside this root are denied.
    pub workspace_root: Option<PathBuf>,
    /// Directories from which access is always denied.
    pub blocked_dirs: Vec<PathBuf>,
    rate_limiter: SlidingWindow,
    action_log: Mutex<Vec<AuditEntry>>,
}

impl SecurityPolicy {
    /// Create a policy with explicit settings.
    pub fn new(
        autonomy_level: AutonomyLevel,
        workspace_root: Option<PathBuf>,
        blocked_dirs: Vec<PathBuf>,
        rate_window_secs: u64,
        max_actions_per_window: usize,
    ) -> Self {
        Self {
            autonomy_level,
            workspace_root,
            blocked_dirs,
            rate_limiter: SlidingWindow::new(rate_window_secs, max_actions_per_window),
            action_log: Mutex::new(Vec::new()),
        }
    }

    /// Sensible production defaults: Supervised mode, no workspace restriction,
    /// standard blocked dirs, 20 actions per hour.
    pub fn default_policy() -> Self {
        Self::new(
            AutonomyLevel::Supervised,
            None,
            default_blocked_dirs(),
            3600,
            20,
        )
    }

    // ── Risk classification ──────────────────────────────────────────────

    /// Classify the risk level of a shell command by examining its first token.
    pub fn classify_command_risk(&self, command: &str) -> RiskLevel {
        let executable = extract_executable(command);
        classify_executable_risk(&executable)
    }

    // ── Validation ───────────────────────────────────────────────────────

    /// Validate a shell command against the current policy.
    pub fn validate_command(&self, command: &str) -> ValidationResult {
        // 1. Check for shell injection patterns.
        if let Some(reason) = detect_injection(command) {
            return ValidationResult::Denied(reason);
        }

        // 2. Check for blocked executables.
        let executable = extract_executable(command);
        if BLOCKED_EXECUTABLES.contains(&executable.as_str()) {
            return ValidationResult::Denied(format!(
                "executable '{executable}' is not permitted"
            ));
        }

        let risk = classify_executable_risk(&executable);
        self.apply_autonomy(&risk)
    }

    /// Validate a filesystem path against the current policy.
    pub fn validate_path(&self, path: &Path) -> ValidationResult {
        let path_str = path.to_string_lossy();

        // 1. Null bytes.
        if path_str.contains('\0') {
            return ValidationResult::Denied("path contains null byte".into());
        }

        // 2. Raw `..` check (catches cases where the file doesn't exist yet).
        if path_str.contains("..") {
            return ValidationResult::Denied("path traversal ('..') is not allowed".into());
        }

        // 3. Resolve symlinks and canonicalize if the path exists.
        let canonical = match path.canonicalize() {
            Ok(p) => p,
            // Non-existent path: use the raw path for further checks.
            Err(_) => path.to_path_buf(),
        };

        let canonical_str = canonical.to_string_lossy();

        // 4. Blocked directories.
        for blocked in &self.blocked_dirs {
            if canonical.starts_with(blocked) || canonical_str.starts_with(&*blocked.to_string_lossy()) {
                return ValidationResult::Denied(format!(
                    "access to '{}' is blocked",
                    blocked.display()
                ));
            }
        }

        // 5. Workspace confinement.
        if let Some(ref root) = self.workspace_root {
            if !canonical.starts_with(root) {
                return ValidationResult::Denied(format!(
                    "path is outside the workspace root '{}'",
                    root.display()
                ));
            }
        }

        ValidationResult::Allowed
    }

    // ── Audit ────────────────────────────────────────────────────────────

    /// Append an entry to the in-memory audit log.
    pub fn log_action(
        &self,
        tool_name: &str,
        args: serde_json::Value,
        risk_level: RiskLevel,
        decision: &str,
        result: Option<&str>,
    ) {
        let entry = AuditEntry {
            timestamp: chrono::Utc::now(),
            tool_name: tool_name.to_string(),
            args,
            risk_level,
            decision: decision.to_string(),
            result: result.map(str::to_string),
        };
        self.action_log
            .lock()
            .expect("audit-log lock poisoned")
            .push(entry);
    }

    /// Return a snapshot of the audit log (newest last).
    pub fn audit_log(&self) -> Vec<AuditEntry> {
        self.action_log
            .lock()
            .expect("audit-log lock poisoned")
            .clone()
    }

    // ── Internal helpers ─────────────────────────────────────────────────

    fn apply_autonomy(&self, risk: &RiskLevel) -> ValidationResult {
        match (&self.autonomy_level, risk) {
            (AutonomyLevel::ReadOnly, RiskLevel::Low) => ValidationResult::Allowed,
            (AutonomyLevel::ReadOnly, _) => {
                ValidationResult::Denied("ReadOnly mode blocks non-read operations".into())
            }
            (AutonomyLevel::Supervised, RiskLevel::Low) => ValidationResult::Allowed,
            (AutonomyLevel::Supervised, _) => ValidationResult::NeedsApproval,
            (AutonomyLevel::Full, _) => {
                if self.rate_limiter.try_record() {
                    ValidationResult::Allowed
                } else {
                    ValidationResult::Denied("rate limit exceeded".into())
                }
            }
        }
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Extract the first token (the executable name) from a shell command string.
fn extract_executable(command: &str) -> String {
    command
        .trim()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string()
}

/// Classify the risk level of an executable by name.
fn classify_executable_risk(executable: &str) -> RiskLevel {
    const LOW: &[&str] = &[
        "ls", "cat", "grep", "git", "echo", "pwd", "which", "file", "head",
        "tail", "wc", "sort", "uniq", "diff", "find", "stat", "type", "env",
        "printenv", "date", "uptime",
    ];
    const MEDIUM: &[&str] = &[
        "touch", "mkdir", "cp", "mv", "npm", "yarn", "pnpm", "bun", "pip",
        "pip3", "cargo", "make", "cmake", "gcc", "clang", "rustc", "python",
        "python3", "node", "tee", "ln",
    ];

    if LOW.contains(&executable) {
        RiskLevel::Low
    } else if MEDIUM.contains(&executable) {
        RiskLevel::Medium
    } else {
        RiskLevel::High
    }
}

/// Executables that are never allowed regardless of autonomy level.
const BLOCKED_EXECUTABLES: &[&str] = &[
    "rm", "sudo", "su", "shutdown", "reboot", "halt", "poweroff", "dd",
    "mkfs", "fdisk", "parted", "format", "del", "rmdir",
];

/// Detect shell injection patterns; returns a reason string if found.
fn detect_injection(command: &str) -> Option<String> {
    let patterns: &[(&str, &str)] = &[
        ("`", "backtick command substitution"),
        ("$(", "command substitution $()"),
        ("${", "variable substitution ${}"),
        (" >> ", "output append redirection"),
        (" > ", "output redirection"),
        (">", "output redirection"),
        ("&&", "command chaining &&"),
        ("||", "command chaining ||"),
        (";", "command separator ;"),
        ("|", "pipe operator"),
    ];
    for (pat, desc) in patterns {
        if command.contains(pat) {
            return Some(format!("shell injection pattern detected: {desc}"));
        }
    }
    None
}

/// Platform-appropriate list of directories that should never be accessible.
fn default_blocked_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/etc"),
        PathBuf::from("/proc"),
        PathBuf::from("/sys"),
        PathBuf::from("/dev"),
    ];
    // Add user-specific sensitive dirs.
    if let Some(home) = dirs::home_dir() {
        for sub in &[".ssh", ".aws", ".gnupg", ".config/gcloud"] {
            dirs.push(home.join(sub));
        }
        #[cfg(unix)]
        dirs.push(PathBuf::from("/root"));
    }
    dirs
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn supervised() -> SecurityPolicy {
        SecurityPolicy::new(AutonomyLevel::Supervised, None, vec![], 3600, 20)
    }

    fn readonly() -> SecurityPolicy {
        SecurityPolicy::new(AutonomyLevel::ReadOnly, None, vec![], 3600, 20)
    }

    fn full() -> SecurityPolicy {
        SecurityPolicy::new(AutonomyLevel::Full, None, vec![], 3600, 20)
    }

    // ── Risk classification ─────────────────────────────────────────────

    #[test]
    fn low_risk_ls() {
        let p = supervised();
        assert_eq!(p.classify_command_risk("ls -la"), RiskLevel::Low);
    }

    #[test]
    fn low_risk_cat() {
        let p = supervised();
        assert_eq!(p.classify_command_risk("cat README.md"), RiskLevel::Low);
    }

    #[test]
    fn low_risk_grep() {
        let p = supervised();
        assert_eq!(p.classify_command_risk("grep -r foo src/"), RiskLevel::Low);
    }

    #[test]
    fn low_risk_git_status() {
        let p = supervised();
        assert_eq!(p.classify_command_risk("git status"), RiskLevel::Low);
    }

    #[test]
    fn low_risk_echo() {
        assert_eq!(supervised().classify_command_risk("echo hello"), RiskLevel::Low);
    }

    #[test]
    fn low_risk_pwd() {
        assert_eq!(supervised().classify_command_risk("pwd"), RiskLevel::Low);
    }

    #[test]
    fn low_risk_which() {
        assert_eq!(supervised().classify_command_risk("which cargo"), RiskLevel::Low);
    }

    #[test]
    fn low_risk_file() {
        assert_eq!(supervised().classify_command_risk("file /usr/bin/ls"), RiskLevel::Low);
    }

    #[test]
    fn medium_risk_mkdir() {
        assert_eq!(supervised().classify_command_risk("mkdir -p foo/bar"), RiskLevel::Medium);
    }

    #[test]
    fn medium_risk_cp() {
        assert_eq!(supervised().classify_command_risk("cp src dst"), RiskLevel::Medium);
    }

    #[test]
    fn medium_risk_mv() {
        assert_eq!(supervised().classify_command_risk("mv a b"), RiskLevel::Medium);
    }

    #[test]
    fn medium_risk_npm_install() {
        assert_eq!(supervised().classify_command_risk("npm install"), RiskLevel::Medium);
    }

    #[test]
    fn medium_risk_cargo_build() {
        assert_eq!(supervised().classify_command_risk("cargo build"), RiskLevel::Medium);
    }

    #[test]
    fn medium_risk_touch() {
        assert_eq!(supervised().classify_command_risk("touch newfile.txt"), RiskLevel::Medium);
    }

    #[test]
    fn high_risk_unknown() {
        assert_eq!(supervised().classify_command_risk("xyzunknown"), RiskLevel::High);
    }

    #[test]
    fn high_risk_curl() {
        assert_eq!(supervised().classify_command_risk("curl https://example.com"), RiskLevel::High);
    }

    #[test]
    fn high_risk_wget() {
        assert_eq!(supervised().classify_command_risk("wget http://x.com"), RiskLevel::High);
    }

    #[test]
    fn high_risk_chmod() {
        assert_eq!(supervised().classify_command_risk("chmod +x script.sh"), RiskLevel::High);
    }

    #[test]
    fn high_risk_chown() {
        assert_eq!(supervised().classify_command_risk("chown user:group file"), RiskLevel::High);
    }

    #[test]
    fn high_risk_kill() {
        assert_eq!(supervised().classify_command_risk("kill -9 1234"), RiskLevel::High);
    }

    // ── Injection detection ─────────────────────────────────────────────

    #[test]
    fn injection_backtick_denied() {
        let r = supervised().validate_command("ls `whoami`");
        assert!(matches!(r, ValidationResult::Denied(_)));
    }

    #[test]
    fn injection_dollar_paren_denied() {
        let r = supervised().validate_command("echo $(cat /etc/passwd)");
        assert!(matches!(r, ValidationResult::Denied(_)));
    }

    #[test]
    fn injection_dollar_brace_denied() {
        let r = supervised().validate_command("echo ${HOME}");
        assert!(matches!(r, ValidationResult::Denied(_)));
    }

    #[test]
    fn injection_redirect_denied() {
        let r = supervised().validate_command("echo hello > /tmp/out");
        assert!(matches!(r, ValidationResult::Denied(_)));
    }

    #[test]
    fn injection_append_denied() {
        let r = supervised().validate_command("echo hello >> /tmp/log");
        assert!(matches!(r, ValidationResult::Denied(_)));
    }

    #[test]
    fn injection_pipe_denied() {
        let r = supervised().validate_command("ls | grep foo");
        assert!(matches!(r, ValidationResult::Denied(_)));
    }

    #[test]
    fn injection_semicolon_denied() {
        let r = supervised().validate_command("ls; rm -rf /");
        assert!(matches!(r, ValidationResult::Denied(_)));
    }

    // ── Blocked executables ─────────────────────────────────────────────

    #[test]
    fn blocked_rm_denied() {
        let r = supervised().validate_command("rm -rf /tmp/foo");
        assert!(matches!(r, ValidationResult::Denied(_)));
    }

    #[test]
    fn blocked_sudo_denied() {
        let r = supervised().validate_command("sudo apt install vim");
        assert!(matches!(r, ValidationResult::Denied(_)));
    }

    #[test]
    fn blocked_dd_denied() {
        let r = full().validate_command("dd if=/dev/zero of=/dev/sda");
        assert!(matches!(r, ValidationResult::Denied(_)));
    }

    #[test]
    fn blocked_mkfs_denied() {
        // Use the bare `mkfs` binary (which is in BLOCKED_EXECUTABLES).
        let r = full().validate_command("mkfs /dev/sdb1");
        assert!(matches!(r, ValidationResult::Denied(_)));
    }

    // ── Autonomy level behaviour ────────────────────────────────────────

    #[test]
    fn readonly_allows_low_risk() {
        let r = readonly().validate_command("ls -la");
        assert_eq!(r, ValidationResult::Allowed);
    }

    #[test]
    fn readonly_denies_medium_risk() {
        let r = readonly().validate_command("mkdir new_dir");
        assert!(matches!(r, ValidationResult::Denied(_)));
    }

    #[test]
    fn readonly_denies_high_risk() {
        let r = readonly().validate_command("wget http://example.com");
        assert!(matches!(r, ValidationResult::Denied(_)));
    }

    #[test]
    fn supervised_needs_approval_for_medium() {
        let r = supervised().validate_command("mkdir new_dir");
        assert_eq!(r, ValidationResult::NeedsApproval);
    }

    #[test]
    fn supervised_needs_approval_for_high() {
        let r = supervised().validate_command("wget http://example.com");
        assert_eq!(r, ValidationResult::NeedsApproval);
    }

    #[test]
    fn supervised_allows_low() {
        let r = supervised().validate_command("ls");
        assert_eq!(r, ValidationResult::Allowed);
    }

    #[test]
    fn full_allows_high_risk_within_limit() {
        let r = full().validate_command("wget http://example.com");
        assert_eq!(r, ValidationResult::Allowed);
    }

    // ── Rate limiting ───────────────────────────────────────────────────

    #[test]
    fn rate_limit_enforced() {
        let policy = SecurityPolicy::new(AutonomyLevel::Full, None, vec![], 60, 3);
        // First 3 succeed.
        assert_eq!(policy.validate_command("wget a"), ValidationResult::Allowed);
        assert_eq!(policy.validate_command("wget b"), ValidationResult::Allowed);
        assert_eq!(policy.validate_command("wget c"), ValidationResult::Allowed);
        // Fourth is denied.
        assert!(matches!(
            policy.validate_command("wget d"),
            ValidationResult::Denied(_)
        ));
    }

    #[test]
    fn rate_limiter_count_tracks_correctly() {
        let policy = SecurityPolicy::new(AutonomyLevel::Full, None, vec![], 60, 5);
        policy.validate_command("wget a");
        policy.validate_command("wget b");
        assert_eq!(policy.rate_limiter.count(), 2);
    }

    // ── Path validation ─────────────────────────────────────────────────

    #[test]
    fn path_null_byte_denied() {
        let p = supervised();
        let bad = PathBuf::from("/tmp/fi\0le");
        assert!(matches!(p.validate_path(&bad), ValidationResult::Denied(_)));
    }

    #[test]
    fn path_traversal_denied() {
        let p = supervised();
        let bad = PathBuf::from("/tmp/../etc/passwd");
        assert!(matches!(p.validate_path(&bad), ValidationResult::Denied(_)));
    }

    #[test]
    fn path_traversal_relative_denied() {
        let p = supervised();
        let bad = PathBuf::from("../../secret");
        assert!(matches!(p.validate_path(&bad), ValidationResult::Denied(_)));
    }

    #[test]
    fn path_blocked_dir_denied() {
        let tmp = TempDir::new().unwrap();
        let blocked = tmp.path().to_path_buf();
        let policy = SecurityPolicy::new(
            AutonomyLevel::Full,
            None,
            vec![blocked.clone()],
            60,
            100,
        );
        let target = blocked.join("file.txt");
        fs::write(&target, "x").unwrap();
        assert!(matches!(
            policy.validate_path(&target),
            ValidationResult::Denied(_)
        ));
    }

    #[test]
    fn path_outside_workspace_denied() {
        let workspace = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        let policy = SecurityPolicy::new(
            AutonomyLevel::Full,
            Some(workspace.path().to_path_buf()),
            vec![],
            60,
            100,
        );
        let target = outside.path().join("file.txt");
        fs::write(&target, "x").unwrap();
        assert!(matches!(
            policy.validate_path(&target),
            ValidationResult::Denied(_)
        ));
    }

    #[test]
    fn path_inside_workspace_allowed() {
        let workspace = TempDir::new().unwrap();
        let policy = SecurityPolicy::new(
            AutonomyLevel::Full,
            Some(workspace.path().to_path_buf()),
            vec![],
            60,
            100,
        );
        let target = workspace.path().join("file.txt");
        fs::write(&target, "x").unwrap();
        assert_eq!(policy.validate_path(&target), ValidationResult::Allowed);
    }

    // ── Audit log ───────────────────────────────────────────────────────

    #[test]
    fn audit_log_records_entries() {
        let p = supervised();
        p.log_action(
            "shell",
            serde_json::json!({"command": "ls"}),
            RiskLevel::Low,
            "allowed",
            Some("ok"),
        );
        let log = p.audit_log();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].tool_name, "shell");
        assert_eq!(log[0].decision, "allowed");
    }
}
