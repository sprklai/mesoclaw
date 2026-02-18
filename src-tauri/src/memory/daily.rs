//! Filesystem-backed daily memory files for the memory subsystem.
//!
//! Daily memory entries are stored as Markdown files in
//! `~/.mesoclaw/memory/`, one file per calendar day:
//!
//! ```text
//! ~/.mesoclaw/memory/
//!   2026-02-18.md     ← today's diary
//!   2026-02-17.md     ← yesterday's diary
//!   MEMORY.md         ← curated long-term memory (user-editable)
//! ```
//!
//! Each entry in a daily file is formatted as:
//! ```text
//! ## HH:MM
//! <content>
//!
//! ```

use std::{fs, path::PathBuf};

use chrono::{Local, NaiveDate};

// ─── DailyMemory ─────────────────────────────────────────────────────────────

/// Manages the `~/.mesoclaw/memory/` directory for daily diary entries and
/// the curated `MEMORY.md` long-term memory file.
pub struct DailyMemory {
    dir: PathBuf,
}

impl DailyMemory {
    /// Create a `DailyMemory` that manages files in `dir`.
    ///
    /// The directory is created on first use — not in the constructor.
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Return the default daily memory directory: `~/.mesoclaw/memory/`.
    pub fn default_dir() -> Result<PathBuf, String> {
        dirs::home_dir()
            .map(|h| h.join(".mesoclaw").join("memory"))
            .ok_or_else(|| "could not determine home directory".to_string())
    }

    // ─── Internal helpers ────────────────────────────────────────────────────

    fn ensure_dir(&self) -> Result<(), String> {
        fs::create_dir_all(&self.dir)
            .map_err(|e| format!("failed to create memory dir {:?}: {e}", self.dir))
    }

    fn day_path(&self, date: &str) -> PathBuf {
        self.dir.join(format!("{date}.md"))
    }

    /// Path to the curated `MEMORY.md` long-term memory file.
    pub fn memory_md_path(&self) -> PathBuf {
        self.dir.join("MEMORY.md")
    }

    // ─── Daily entry methods ─────────────────────────────────────────────────

    /// Append a diary entry to today's file.
    ///
    /// Entry format:
    /// ```text
    /// ## HH:MM
    /// <content>
    ///
    /// ```
    pub fn store_daily(&self, content: &str) -> Result<(), String> {
        self.ensure_dir()?;
        let now = Local::now();
        let date = now.format("%Y-%m-%d").to_string();
        let time = now.format("%H:%M").to_string();
        let entry = format!("## {time}\n{content}\n\n");
        use std::io::Write as _;
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.day_path(&date))
            .and_then(|mut f| f.write_all(entry.as_bytes()))
            .map_err(|e| format!("failed to write daily entry for '{date}': {e}"))
    }

    /// Read the diary content for `date` (format `YYYY-MM-DD`).
    ///
    /// Returns `None` if no entry exists for that date.
    pub fn recall_daily(&self, date: &str) -> Result<Option<String>, String> {
        let path = self.day_path(date);
        match fs::read_to_string(&path) {
            Ok(content) => Ok(Some(content)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("failed to read daily file '{date}': {e}")),
        }
    }

    /// Load today's and yesterday's diary contents.
    ///
    /// Returns `(today_content, yesterday_content)` where each is `None` if
    /// no file exists for that date.
    pub fn get_recent_daily(&self) -> Result<(Option<String>, Option<String>), String> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let yesterday = (Local::now() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        let today_content = self.recall_daily(&today)?;
        let yesterday_content = self.recall_daily(&yesterday)?;
        Ok((today_content, yesterday_content))
    }

    // ─── MEMORY.md long-term memory ──────────────────────────────────────────

    /// Read the curated `MEMORY.md` long-term memory file.
    ///
    /// Returns `None` if the file doesn't exist yet.
    pub fn read_memory_md(&self) -> Result<Option<String>, String> {
        match fs::read_to_string(self.memory_md_path()) {
            Ok(content) => Ok(Some(content)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("failed to read MEMORY.md: {e}")),
        }
    }

    /// Append a section to `MEMORY.md` (for agent-driven updates).
    pub fn append_memory_md(&self, content: &str) -> Result<(), String> {
        self.ensure_dir()?;
        use std::io::Write as _;
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.memory_md_path())
            .and_then(|mut f| f.write_all(format!("\n\n{content}").as_bytes()))
            .map_err(|e| format!("failed to write MEMORY.md: {e}"))
    }

    /// Build the daily-memory context block suitable for injection into a
    /// system prompt.  Includes today's and yesterday's entries (if any) and
    /// the MEMORY.md content (if any).
    pub fn build_daily_context(&self) -> String {
        let mut sections: Vec<String> = Vec::new();

        if let Ok(Some(mem)) = self.read_memory_md()
            && !mem.trim().is_empty()
        {
            sections.push(format!("## Long-term Memory\n\n{mem}"));
        }

        let today = Local::now().format("%Y-%m-%d").to_string();
        let yesterday = (Local::now() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        if let Ok(Some(content)) = self.recall_daily(&today)
            && !content.trim().is_empty()
        {
            sections.push(format!("## Today's Diary ({today})\n\n{content}"));
        }
        if let Ok(Some(content)) = self.recall_daily(&yesterday)
            && !content.trim().is_empty()
        {
            sections.push(format!("## Yesterday's Diary ({yesterday})\n\n{content}"));
        }

        sections.join("\n\n---\n\n")
    }

    // ─── List helpers ─────────────────────────────────────────────────────────

    /// List all daily diary dates (as `YYYY-MM-DD` strings) in descending order.
    pub fn list_dates(&self) -> Result<Vec<String>, String> {
        match fs::read_dir(&self.dir) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(format!("failed to read memory dir: {e}")),
            Ok(rd) => {
                let mut dates: Vec<String> = rd
                    .filter_map(|entry| entry.ok())
                    .filter_map(|entry| {
                        let name = entry.file_name().to_string_lossy().to_string();
                        // Match YYYY-MM-DD.md pattern.
                        if name.ends_with(".md") && name != "MEMORY.md" {
                            let date = name.trim_end_matches(".md");
                            NaiveDate::parse_from_str(date, "%Y-%m-%d")
                                .ok()
                                .map(|_| date.to_owned())
                        } else {
                            None
                        }
                    })
                    .collect();
                dates.sort_by(|a, b| b.cmp(a)); // Descending.
                Ok(dates)
            }
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_daily(tmp: &TempDir) -> DailyMemory {
        DailyMemory::new(tmp.path().to_path_buf())
    }

    #[test]
    fn store_daily_creates_file() {
        let tmp = TempDir::new().unwrap();
        let daily = make_daily(&tmp);
        daily.store_daily("Worked on memory system.").unwrap();

        let today = Local::now().format("%Y-%m-%d").to_string();
        assert!(
            tmp.path().join(format!("{today}.md")).exists(),
            "daily file should exist"
        );
    }

    #[test]
    fn recall_daily_returns_content() {
        let tmp = TempDir::new().unwrap();
        let daily = make_daily(&tmp);
        daily.store_daily("First entry.").unwrap();

        let today = Local::now().format("%Y-%m-%d").to_string();
        let content = daily.recall_daily(&today).unwrap();
        assert!(content.is_some(), "should recall today's entry");
        assert!(content.unwrap().contains("First entry."));
    }

    #[test]
    fn recall_daily_nonexistent_date_returns_none() {
        let tmp = TempDir::new().unwrap();
        let daily = make_daily(&tmp);
        let result = daily.recall_daily("1970-01-01").unwrap();
        assert!(result.is_none(), "no file for ancient date → None");
    }

    #[test]
    fn store_daily_appends_multiple_entries() {
        let tmp = TempDir::new().unwrap();
        let daily = make_daily(&tmp);
        daily.store_daily("First.").unwrap();
        daily.store_daily("Second.").unwrap();

        let today = Local::now().format("%Y-%m-%d").to_string();
        let content = daily.recall_daily(&today).unwrap().unwrap();
        assert!(content.contains("First."), "first entry should be present");
        assert!(
            content.contains("Second."),
            "second entry should be present"
        );
    }

    #[test]
    fn daily_entry_has_time_header() {
        let tmp = TempDir::new().unwrap();
        let daily = make_daily(&tmp);
        daily.store_daily("Content with header.").unwrap();

        let today = Local::now().format("%Y-%m-%d").to_string();
        let content = daily.recall_daily(&today).unwrap().unwrap();
        assert!(content.contains("## "), "entry should have ## HH:MM header");
    }

    #[test]
    fn get_recent_daily_returns_today() {
        let tmp = TempDir::new().unwrap();
        let daily = make_daily(&tmp);
        daily.store_daily("Today's work.").unwrap();

        let (today, _yesterday) = daily.get_recent_daily().unwrap();
        assert!(today.is_some(), "today's content should be available");
        assert!(today.unwrap().contains("Today's work."));
    }

    #[test]
    fn read_memory_md_returns_none_when_missing() {
        let tmp = TempDir::new().unwrap();
        let daily = make_daily(&tmp);
        // Don't create MEMORY.md
        let result = daily.read_memory_md().unwrap();
        assert!(result.is_none(), "MEMORY.md absent → None");
    }

    #[test]
    fn append_memory_md_creates_file() {
        let tmp = TempDir::new().unwrap();
        let daily = make_daily(&tmp);
        daily
            .append_memory_md("# Long-term facts\n\nUser prefers concise answers.")
            .unwrap();
        assert!(
            daily.memory_md_path().exists(),
            "MEMORY.md should exist after append"
        );
    }

    #[test]
    fn read_memory_md_returns_content() {
        let tmp = TempDir::new().unwrap();
        let daily = make_daily(&tmp);
        daily.append_memory_md("User name: Alice.").unwrap();
        let content = daily.read_memory_md().unwrap().unwrap();
        assert!(
            content.contains("Alice"),
            "MEMORY.md should contain appended content"
        );
    }

    #[test]
    fn list_dates_excludes_memory_md() {
        let tmp = TempDir::new().unwrap();
        let daily = make_daily(&tmp);
        daily.store_daily("Test.").unwrap();
        daily.append_memory_md("Fact.").unwrap();

        let dates = daily.list_dates().unwrap();
        assert!(
            !dates.contains(&"MEMORY".to_string()),
            "MEMORY.md should not appear in dates"
        );
    }

    #[test]
    fn list_dates_returns_todays_date() {
        let tmp = TempDir::new().unwrap();
        let daily = make_daily(&tmp);
        daily.store_daily("Test.").unwrap();

        let dates = daily.list_dates().unwrap();
        let today = Local::now().format("%Y-%m-%d").to_string();
        assert!(
            dates.contains(&today),
            "today's date should appear in list_dates"
        );
    }

    #[test]
    fn build_daily_context_empty_when_no_files() {
        let tmp = TempDir::new().unwrap();
        let daily = make_daily(&tmp);
        let context = daily.build_daily_context();
        assert!(context.is_empty(), "no files → empty context string");
    }

    #[test]
    fn build_daily_context_includes_today_entry() {
        let tmp = TempDir::new().unwrap();
        let daily = make_daily(&tmp);
        daily.store_daily("Context content.").unwrap();

        let context = daily.build_daily_context();
        assert!(
            context.contains("Context content."),
            "context should include today's diary"
        );
    }
}
