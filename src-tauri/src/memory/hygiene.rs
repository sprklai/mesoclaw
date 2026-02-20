//! Memory hygiene service — archive old daily diary files and purge stale archives.
//!
//! # File layout
//! ```text
//! ~/.mesoclaw/memory/
//!   2026-02-18.md          ← current daily diary
//!   2026-02-01.md          ← old diary (will be archived)
//!   MEMORY.md              ← curated long-term memory (never touched)
//!   archive/
//!     2026-01-15.md        ← already archived (will be purged after 30 days)
//! ```
//!
//! # Policy
//! - **Archive**: Daily diary files (`YYYY-MM-DD.md`) in the main directory
//!   whose date is strictly older than `archive_days` days are moved to the
//!   `archive/` sub-directory.  `MEMORY.md` is never archived.
//! - **Purge**: Files already in `archive/` whose date is strictly older than
//!   `purge_days` days are permanently deleted.
//!
//! File age is derived from the filename date (not `mtime`) so behaviour is
//! reproducible regardless of filesystem metadata.
//!
//! # Scheduler integration
//! `MemoryHygiene::run()` is intended to be called once daily via the
//! `TokioScheduler`.  A typical job would use a `cron` schedule of `0 3 * * *`
//! (03:00 every night) to minimise interference with interactive sessions.

use log::info;
use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::{Duration, Local, NaiveDate};

// ─── HygieneConfig ────────────────────────────────────────────────────────────

/// Configuration knobs for the memory hygiene service.
#[derive(Debug, Clone)]
pub struct HygieneConfig {
    /// Number of days after which a daily diary file is moved to `archive/`.
    ///
    /// Corresponds to config key `memory.hygiene.archive_days`.  Default: 7.
    pub archive_days: u64,

    /// Number of days after which an archived file is permanently deleted.
    ///
    /// Corresponds to config key `memory.hygiene.purge_days`.  Default: 30.
    pub purge_days: u64,

    /// When `false` the service is a no-op.
    ///
    /// Corresponds to config key `memory.hygiene.enabled`.  Default: `true`.
    pub enabled: bool,
}

impl Default for HygieneConfig {
    fn default() -> Self {
        Self {
            archive_days: 7,
            purge_days: 30,
            enabled: true,
        }
    }
}

// ─── HygieneReport ────────────────────────────────────────────────────────────

/// Summary of the actions taken during a [`MemoryHygiene::run()`] call.
#[derive(Debug, Default)]
pub struct HygieneReport {
    /// Date strings (`YYYY-MM-DD`) of files that were moved to the archive.
    pub archived: Vec<String>,
    /// Date strings (`YYYY-MM-DD`) of archive files that were deleted.
    pub purged: Vec<String>,
    /// Non-fatal error messages collected during the run.
    pub errors: Vec<String>,
}

impl HygieneReport {
    fn push_error(&mut self, msg: impl Into<String>) {
        self.errors.push(msg.into());
    }
}

// ─── MemoryHygiene ────────────────────────────────────────────────────────────

/// Runs archive and purge policies on the `~/.mesoclaw/memory/` directory.
///
/// # Example
/// ```rust,ignore
/// use mesoclaw::memory::hygiene::{HygieneConfig, MemoryHygiene};
///
/// let report = MemoryHygiene::new(memory_dir, HygieneConfig::default()).run();
/// log::info!("hygiene: archived={}, purged={}", report.archived.len(), report.purged.len());
/// ```
pub struct MemoryHygiene {
    config: HygieneConfig,
    memory_dir: PathBuf,
}

impl MemoryHygiene {
    /// Create a new hygiene service for the given memory directory.
    pub fn new(memory_dir: PathBuf, config: HygieneConfig) -> Self {
        Self { config, memory_dir }
    }

    /// Path to the `archive/` sub-directory inside the memory directory.
    fn archive_dir(&self) -> PathBuf {
        self.memory_dir.join("archive")
    }

    /// Parse a `YYYY-MM-DD` date from a file's stem (e.g. `"2026-02-01"`).
    ///
    /// Returns `None` for files that don't match the pattern (e.g. `MEMORY.md`).
    fn parse_date_from_filename(path: &Path) -> Option<NaiveDate> {
        let stem = path.file_stem()?.to_str()?;
        NaiveDate::parse_from_str(stem, "%Y-%m-%d").ok()
    }

    /// Run the full hygiene cycle: archive old files, then purge stale archives.
    ///
    /// Individual file errors are collected in [`HygieneReport::errors`] rather
    /// than aborting the whole run so that one unreadable file does not block
    /// the rest of the cleanup.
    pub fn run(&self) -> HygieneReport {
        let mut report = HygieneReport::default();

        if !self.config.enabled {
            return report;
        }

        // Ensure the archive directory exists before we try to move files there.
        if let Err(e) = fs::create_dir_all(self.archive_dir()) {
            report.push_error(format!("failed to create archive dir: {e}"));
            return report;
        }

        self.archive_old_files(&mut report);
        self.purge_old_archives(&mut report);

        report
    }

    // ─── Private helpers ─────────────────────────────────────────────────────

    /// Move diary files older than `config.archive_days` to the archive dir.
    fn archive_old_files(&self, report: &mut HygieneReport) {
        let today = Local::now().naive_local().date();
        // threshold = today - archive_days.  Files with date < threshold are archived.
        let threshold =
            match today.checked_sub_signed(Duration::days(self.config.archive_days as i64)) {
                Some(t) => t,
                None => return,
            };

        let entries = match fs::read_dir(&self.memory_dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
            Err(e) => {
                report.push_error(format!("cannot read memory dir: {e}"));
                return;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();

            // Only process regular files with .md extension at the top level.
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }

            // Skip files without a parseable date stem (e.g. MEMORY.md).
            let file_date = match Self::parse_date_from_filename(&path) {
                Some(d) => d,
                None => continue,
            };

            // File is within the keep window — leave it.
            if file_date >= threshold {
                continue;
            }

            let dest = self
                .archive_dir()
                .join(path.file_name().unwrap_or_default());
            match fs::rename(&path, &dest) {
                Ok(()) => {
                    report.archived.push(file_date.to_string());
                    info!("hygiene: archived {:?} → {:?}", path, dest);
                }
                Err(e) => {
                    report.push_error(format!("failed to archive '{file_date}': {e}"));
                }
            }
        }
    }

    /// Delete archived files older than `config.purge_days`.
    fn purge_old_archives(&self, report: &mut HygieneReport) {
        let today = Local::now().naive_local().date();
        let threshold =
            match today.checked_sub_signed(Duration::days(self.config.purge_days as i64)) {
                Some(t) => t,
                None => return,
            };

        let entries = match fs::read_dir(self.archive_dir()) {
            Ok(e) => e,
            // Archive dir may not exist yet — that is fine.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
            Err(e) => {
                report.push_error(format!("cannot read archive dir: {e}"));
                return;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }

            let file_date = match Self::parse_date_from_filename(&path) {
                Some(d) => d,
                None => continue,
            };

            if file_date >= threshold {
                continue;
            }

            match fs::remove_file(&path) {
                Ok(()) => {
                    report.purged.push(file_date.to_string());
                    info!("hygiene: purged {:?}", path);
                }
                Err(e) => {
                    report.push_error(format!("failed to purge archive '{file_date}': {e}"));
                }
            }
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn hygiene(tmp: &TempDir, config: HygieneConfig) -> MemoryHygiene {
        MemoryHygiene::new(tmp.path().to_path_buf(), config)
    }

    fn write_diary(dir: &Path, date: &str) {
        let path = dir.join(format!("{date}.md"));
        fs::write(path, format!("## 12:00\nContent for {date}\n\n")).unwrap();
    }

    /// Return the `YYYY-MM-DD` string for `n` days ago.
    fn days_ago(n: i64) -> String {
        (Local::now().naive_local().date() - Duration::days(n))
            .format("%Y-%m-%d")
            .to_string()
    }

    // ── disabled mode ─────────────────────────────────────────────────────────

    #[test]
    fn disabled_hygiene_is_noop() {
        let tmp = TempDir::new().unwrap();
        write_diary(tmp.path(), &days_ago(10));

        let config = HygieneConfig {
            enabled: false,
            ..HygieneConfig::default()
        };
        let report = hygiene(&tmp, config).run();

        assert!(report.archived.is_empty());
        assert!(report.purged.is_empty());
        assert!(report.errors.is_empty());
        assert!(tmp.path().join(format!("{}.md", days_ago(10))).exists());
    }

    // ── archive policy ────────────────────────────────────────────────────────

    #[test]
    fn recent_files_are_not_archived() {
        let tmp = TempDir::new().unwrap();
        write_diary(tmp.path(), &days_ago(0)); // today
        write_diary(tmp.path(), &days_ago(3)); // 3 days ago

        let report = hygiene(&tmp, HygieneConfig::default()).run();
        assert!(
            report.archived.is_empty(),
            "files within 7-day window should stay"
        );
    }

    #[test]
    fn old_files_are_archived() {
        let tmp = TempDir::new().unwrap();
        let old = days_ago(8); // 8 days old, threshold is 7
        write_diary(tmp.path(), &old);

        let report = hygiene(&tmp, HygieneConfig::default()).run();

        assert_eq!(report.archived.len(), 1);
        assert_eq!(report.archived[0], old);
        assert!(report.errors.is_empty());
        // Original file removed from main dir.
        assert!(!tmp.path().join(format!("{old}.md")).exists());
        // File present in archive.
        assert!(
            tmp.path()
                .join("archive")
                .join(format!("{old}.md"))
                .exists()
        );
    }

    #[test]
    fn boundary_file_not_archived() {
        // A file exactly `archive_days` days old should NOT be archived
        // (threshold = today - archive_days; file_date == threshold → skip).
        let tmp = TempDir::new().unwrap();
        let boundary = days_ago(7); // exactly at the boundary
        write_diary(tmp.path(), &boundary);

        let report = hygiene(&tmp, HygieneConfig::default()).run();
        assert!(
            report.archived.is_empty(),
            "boundary file should not be archived"
        );
    }

    #[test]
    fn memory_md_is_never_archived() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("MEMORY.md"), "Long-term memory facts.").unwrap();
        write_diary(tmp.path(), &days_ago(10)); // also write an old diary

        let report = hygiene(&tmp, HygieneConfig::default()).run();

        assert!(
            tmp.path().join("MEMORY.md").exists(),
            "MEMORY.md must not be archived"
        );
        assert_eq!(report.archived.len(), 1, "only the diary file is archived");
    }

    #[test]
    fn custom_archive_days_respected() {
        let tmp = TempDir::new().unwrap();
        write_diary(tmp.path(), &days_ago(3)); // 3 days old

        // With archive_days = 2, a 3-day-old file should be archived.
        let config = HygieneConfig {
            archive_days: 2,
            ..HygieneConfig::default()
        };
        let report = hygiene(&tmp, config).run();

        assert_eq!(report.archived.len(), 1);
    }

    #[test]
    fn multiple_old_files_all_archived() {
        let tmp = TempDir::new().unwrap();
        write_diary(tmp.path(), &days_ago(10));
        write_diary(tmp.path(), &days_ago(15));
        write_diary(tmp.path(), &days_ago(20));
        write_diary(tmp.path(), &days_ago(3)); // recent — should stay

        let report = hygiene(&tmp, HygieneConfig::default()).run();

        assert_eq!(report.archived.len(), 3);
        assert!(report.errors.is_empty());
        // Recent file untouched.
        assert!(tmp.path().join(format!("{}.md", days_ago(3))).exists());
    }

    // ── purge policy ─────────────────────────────────────────────────────────

    #[test]
    fn archived_files_are_purged_after_threshold() {
        let tmp = TempDir::new().unwrap();
        let archive_dir = tmp.path().join("archive");
        fs::create_dir_all(&archive_dir).unwrap();

        let old = days_ago(35); // 35 days, purge threshold is 30
        fs::write(archive_dir.join(format!("{old}.md")), "old archive content").unwrap();

        let report = hygiene(&tmp, HygieneConfig::default()).run();

        assert_eq!(report.purged.len(), 1);
        assert_eq!(report.purged[0], old);
        assert!(!archive_dir.join(format!("{old}.md")).exists());
    }

    #[test]
    fn recently_archived_files_not_purged() {
        let tmp = TempDir::new().unwrap();
        let archive_dir = tmp.path().join("archive");
        fs::create_dir_all(&archive_dir).unwrap();

        let recent_archive = days_ago(10); // 10 days in archive, purge is 30
        fs::write(
            archive_dir.join(format!("{recent_archive}.md")),
            "recent archive",
        )
        .unwrap();

        let report = hygiene(&tmp, HygieneConfig::default()).run();
        assert!(report.purged.is_empty());
    }

    #[test]
    fn custom_purge_days_respected() {
        let tmp = TempDir::new().unwrap();
        let archive_dir = tmp.path().join("archive");
        fs::create_dir_all(&archive_dir).unwrap();

        let old = days_ago(10); // 10 days old
        fs::write(archive_dir.join(format!("{old}.md")), "old content").unwrap();

        // With purge_days = 7, a 10-day-old archive file should be purged.
        let config = HygieneConfig {
            purge_days: 7,
            ..HygieneConfig::default()
        };
        let report = hygiene(&tmp, config).run();

        assert_eq!(report.purged.len(), 1);
    }

    // ── idempotency ───────────────────────────────────────────────────────────

    #[test]
    fn idempotent_second_run_archives_nothing_new() {
        let tmp = TempDir::new().unwrap();
        let old = days_ago(10);
        write_diary(tmp.path(), &old);

        let h = hygiene(&tmp, HygieneConfig::default());
        let first = h.run();
        let second = h.run();

        assert_eq!(first.archived.len(), 1);
        // Second run: file already moved; nothing left to archive.
        assert!(second.archived.is_empty());
    }

    #[test]
    fn missing_archive_dir_does_not_error_on_purge() {
        let tmp = TempDir::new().unwrap();
        // No archive dir and no diary files — both phases should be silent.
        let report = hygiene(&tmp, HygieneConfig::default()).run();
        assert!(report.errors.is_empty());
        assert!(report.archived.is_empty());
        assert!(report.purged.is_empty());
    }
}
