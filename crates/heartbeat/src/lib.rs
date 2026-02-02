//! Heartbeat service for periodic agent wake-up

use std::path::{Path, PathBuf};
use tokio::time::{interval, Duration};
use tracing::{debug, info};

const DEFAULT_INTERVAL_S: u64 = 30 * 60; // 30 minutes
const HEARTBEAT_PROMPT: &str = "Read HEARTBEAT.md in your workspace (if it exists).
Follow any instructions or tasks listed there.
If nothing needs attention, reply with just: HEARTBEAT_OK";

const HEARTBEAT_OK_TOKEN: &str = "HEARTBEAT_OK";

/// Heartbeat service for periodic tasks
pub struct HeartbeatService {
    workspace: PathBuf,
    interval_s: u64,
    enabled: bool,
}

impl HeartbeatService {
    /// Create a new heartbeat service
    pub fn new(workspace: impl AsRef<Path>, interval_s: Option<u64>, enabled: bool) -> Self {
        Self {
            workspace: workspace.as_ref().to_path_buf(),
            interval_s: interval_s.unwrap_or(DEFAULT_INTERVAL_S),
            enabled,
        }
    }

    /// Check if HEARTBEAT.md has actionable content
    async fn has_actionable_content(&self) -> bool {
        let path = self.workspace.join("HEARTBEAT.md");
        if !path.exists() {
            return false;
        }

        match tokio::fs::read_to_string(&path).await {
            Ok(content) => {
                // Check if there's anything besides headers and empty lines
                content.lines().any(|line| {
                    let trimmed = line.trim();
                    !trimmed.is_empty()
                        && !trimmed.starts_with('#')
                        && !trimmed.starts_with("<!--")
                        && !trimmed.starts_with("- [ ]")
                        && !trimmed.starts_with("* [ ]")
                })
            }
            Err(_) => false,
        }
    }

    /// Run the heartbeat service
    pub async fn run<F, Fut>(&self, mut on_heartbeat: F)
    where
        F: FnMut(String) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = String> + Send + 'static,
    {
        if !self.enabled {
            info!("Heartbeat service disabled");
            return;
        }

        info!("Heartbeat service started (every {}s)", self.interval_s);

        let mut interval = interval(Duration::from_secs(self.interval_s));

        loop {
            interval.tick().await;

            if self.has_actionable_content().await {
                info!("Heartbeat: checking for tasks...");
                let response = on_heartbeat(HEARTBEAT_PROMPT.to_string()).await;

                if response.to_uppercase().contains(HEARTBEAT_OK_TOKEN) {
                    debug!("Heartbeat: OK (no action needed)");
                } else {
                    info!("Heartbeat: completed task");
                }
            } else {
                debug!("Heartbeat: no tasks (HEARTBEAT.md empty)");
            }
        }
    }
}
