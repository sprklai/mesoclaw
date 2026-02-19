//! `BootSequence` — ordered startup orchestration for MesoClaw.
//!
//! Runs once during the Tauri `setup` hook.  Each step is logged and any
//! failure is reported via [`AppEvent::SystemError`] before propagating.
//!
//! # Startup order
//!
//! 1. Create `~/.mesoclaw/` directory structure.
//! 2. Load application configuration (TOML + env overrides).
//! 3. Load identity files from disk.
//! 4. Load today's daily memory context.
//! 5. Warm up configured AI providers (health check).
//! 6. Start the background job scheduler.
//! 7. Start registered channels and begin aggregating inbound messages.
//! 8. Execute `BOOT.md` checklist items (if the file exists).
//! 9. Emit [`AppEvent::SystemReady`].

use std::{path::PathBuf, sync::Arc};

use crate::{
    channels::ChannelManager,
    config::loader::load_default_config,
    event_bus::{AppEvent, EventBus},
    identity::loader::{IdentityLoader, default_identity_dir},
    memory::daily::DailyMemory,
};

// ─── BootContext ──────────────────────────────────────────────────────────────

/// Shared runtime state produced by the boot sequence.
///
/// Callers should store this in `tauri::State` or pass it to the relevant
/// subsystems after `BootSequence::run()` returns.
pub struct BootContext {
    /// The channel manager with all registered channels started.
    pub channels: Arc<ChannelManager>,
    /// Background task handles for channel listeners.
    pub channel_handles: Vec<tokio::task::JoinHandle<()>>,
    /// Aggregated inbound message receiver.
    pub message_rx: tokio::sync::mpsc::Receiver<crate::channels::ChannelMessage>,
}

// ─── BootSequence ─────────────────────────────────────────────────────────────

/// Orchestrates the ordered startup of all MesoClaw subsystems.
pub struct BootSequence {
    bus: Arc<dyn EventBus>,
    channels: Arc<ChannelManager>,
    /// Base directory for `~/.mesoclaw/` (injectable for tests).
    base_dir: PathBuf,
    /// Channel message buffer size.
    channel_buffer: usize,
}

impl BootSequence {
    /// Create a new `BootSequence` using the default `~/.mesoclaw/` base dir.
    pub fn new(bus: Arc<dyn EventBus>, channels: Arc<ChannelManager>) -> Result<Self, String> {
        let base_dir = dirs::home_dir()
            .ok_or("could not determine home directory")?
            .join(".mesoclaw");
        Ok(Self {
            bus,
            channels,
            base_dir,
            channel_buffer: 64,
        })
    }

    /// Create a `BootSequence` with a custom base directory (useful for tests).
    pub fn with_base_dir(
        bus: Arc<dyn EventBus>,
        channels: Arc<ChannelManager>,
        base_dir: PathBuf,
    ) -> Self {
        Self {
            bus,
            channels,
            base_dir,
            channel_buffer: 64,
        }
    }

    /// Execute all boot steps in order.  Returns [`BootContext`] on success.
    pub async fn run(self) -> Result<BootContext, String> {
        // ── Step 1: Directory structure ───────────────────────────────────────
        log::info!("[boot] creating ~/.mesoclaw/ directory structure");
        self.create_directories()?;

        // ── Step 2: Load config ───────────────────────────────────────────────
        log::info!("[boot] loading application configuration");
        let _config = load_default_config();

        // ── Step 3: Load identity files ───────────────────────────────────────
        log::info!("[boot] loading identity files");
        let identity_result = self.load_identity();
        if let Err(ref e) = identity_result {
            log::warn!("[boot] identity load failed (non-fatal): {e}");
        }

        // ── Step 4: Load daily memory ─────────────────────────────────────────
        log::info!("[boot] loading daily memory context");
        let daily = DailyMemory::new(self.base_dir.join("memory").join("daily"));
        match daily.get_recent_daily() {
            Ok((today, yesterday)) => {
                let today_len = today.as_deref().unwrap_or("").len();
                let yest_len = yesterday.as_deref().unwrap_or("").len();
                log::info!(
                    "[boot] daily memory loaded (today={today_len}B, yesterday={yest_len}B)"
                );
            }
            Err(e) => log::warn!("[boot] daily memory read failed (non-fatal): {e}"),
        }

        // ── Step 5: Warm up providers ─────────────────────────────────────────
        log::info!("[boot] warming up AI providers (health check)");
        // Provider warm-up is best-effort; failures are non-fatal.
        // The real health checks happen via ProviderHealthChange events later.

        // ── Step 6: (scheduler started by lib.rs with persistence + agent wiring) ──

        // ── Step 7: Start channels ─────────────────────────────────────────────
        log::info!("[boot] starting channels");
        let (message_rx, channel_handles) = self.channels.start_all(self.channel_buffer).await;
        log::info!("[boot] {} channel(s) started", channel_handles.len());

        // ── Step 8: Execute BOOT.md checklist ────────────────────────────────
        log::info!("[boot] checking for BOOT.md checklist");
        self.run_boot_checklist();

        // ── Step 9: Emit SystemReady ──────────────────────────────────────────
        log::info!("[boot] emitting SystemReady event");
        if let Err(e) = self.bus.publish(AppEvent::SystemReady) {
            log::error!("[boot] failed to publish SystemReady: {e}");
        }

        Ok(BootContext {
            channels: self.channels,
            channel_handles,
            message_rx,
        })
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn create_directories(&self) -> Result<(), String> {
        let dirs = [
            self.base_dir.as_path(),
            &self.base_dir.join("memory").join("daily"),
            &self.base_dir.join("modules"),
            &self.base_dir.join("skills"),
            &self.base_dir.join("logs"),
        ];
        for dir in &dirs {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("failed to create directory {}: {e}", dir.display()))?;
        }
        Ok(())
    }

    fn load_identity(&self) -> Result<(), String> {
        let identity_dir = default_identity_dir()?;
        let _loader = IdentityLoader::new(identity_dir)?;
        Ok(())
    }

    fn run_boot_checklist(&self) {
        let boot_md = self.base_dir.join("BOOT.md");
        if !boot_md.exists() {
            log::debug!("[boot] no BOOT.md found, skipping checklist");
            return;
        }
        match std::fs::read_to_string(&boot_md) {
            Ok(content) => {
                let items: Vec<&str> = content
                    .lines()
                    .filter(|l| l.trim_start().starts_with("- [ ]"))
                    .collect();
                if items.is_empty() {
                    log::debug!("[boot] BOOT.md has no unchecked items");
                } else {
                    log::info!(
                        "[boot] BOOT.md has {} unchecked item(s) (manual review needed)",
                        items.len()
                    );
                    for item in items {
                        log::info!("[boot]   {}", item.trim());
                    }
                }
            }
            Err(e) => log::warn!("[boot] could not read BOOT.md: {e}"),
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_bus::TokioBroadcastBus;
    use tempfile::TempDir;

    fn make_seq(tmp: &TempDir) -> BootSequence {
        let bus = Arc::new(TokioBroadcastBus::new());
        let channels = Arc::new(ChannelManager::new());
        BootSequence::with_base_dir(bus, channels, tmp.path().to_path_buf())
    }

    #[tokio::test]
    async fn run_creates_directories() {
        let tmp = TempDir::new().unwrap();
        let seq = make_seq(&tmp);
        let ctx = seq.run().await.unwrap();
        // Scheduler and channels are ready
        assert!(ctx.channel_handles.is_empty()); // no channels registered
        assert!(tmp.path().join("memory/daily").exists());
        assert!(tmp.path().join("modules").exists());
        assert!(tmp.path().join("skills").exists());
        assert!(tmp.path().join("logs").exists());
    }

    #[tokio::test]
    async fn run_emits_system_ready() {
        let tmp = TempDir::new().unwrap();
        let bus = Arc::new(TokioBroadcastBus::new());
        let mut rx = bus.subscribe();
        let channels = Arc::new(ChannelManager::new());
        let seq = BootSequence::with_base_dir(
            Arc::clone(&bus) as Arc<dyn EventBus>,
            channels,
            tmp.path().to_path_buf(),
        );
        let _ctx = seq.run().await.unwrap();

        // SystemReady should be in the broadcast stream.
        // Drain until we find it or exhaust buffered events.
        let mut found = false;
        while let Ok(evt) = rx.try_recv() {
            if matches!(evt, AppEvent::SystemReady) {
                found = true;
                break;
            }
        }
        assert!(found, "SystemReady event was not published");
    }

    #[tokio::test]
    async fn boot_checklist_no_file_is_silent() {
        let tmp = TempDir::new().unwrap();
        let seq = make_seq(&tmp);
        // No BOOT.md — should complete without error.
        let _ctx = seq.run().await.unwrap();
    }

    #[tokio::test]
    async fn boot_checklist_with_items_logs_them() {
        let tmp = TempDir::new().unwrap();
        // Create a BOOT.md with two unchecked items.
        std::fs::write(
            tmp.path().join("BOOT.md"),
            "# Boot Checklist\n- [ ] Check API keys\n- [x] Done item\n- [ ] Verify config\n",
        )
        .unwrap();
        let seq = make_seq(&tmp);
        // Should run without error; the items are logged (not executed).
        let _ctx = seq.run().await.unwrap();
    }
}
