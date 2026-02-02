//! Cron service for scheduled tasks

use chrono::Local;
use serde::{Deserialize, Serialize};

use std::path::{Path, PathBuf};

use tracing::{debug, info};
use uuid::Uuid;

/// Cron job schedule
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum Schedule {
    /// Run at a specific time (one-shot)
    #[serde(rename = "at")]
    At { at_ms: i64 },
    /// Run every N milliseconds
    #[serde(rename = "every")]
    Every { every_ms: i64 },
    /// Run on a cron expression
    #[serde(rename = "cron")]
    Cron { expr: String },
}

/// Cron job payload
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Payload {
    /// Message to send to the agent
    pub message: String,
    /// Whether to deliver response to a channel
    #[serde(default)]
    pub deliver: bool,
    /// Target channel for delivery
    #[serde(default)]
    pub channel: Option<String>,
    /// Target recipient
    #[serde(default)]
    pub to: Option<String>,
}

impl Payload {
    /// Create a new payload
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            deliver: false,
            channel: None,
            to: None,
        }
    }

    /// Set deliver flag
    pub fn with_deliver(mut self, deliver: bool) -> Self {
        self.deliver = deliver;
        self
    }

    /// Set channel
    pub fn with_channel(mut self, channel: impl Into<String>) -> Self {
        self.channel = Some(channel.into());
        self
    }

    /// Set recipient
    pub fn with_to(mut self, to: impl Into<String>) -> Self {
        self.to = Some(to.into());
        self
    }
}

/// Cron job state
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct JobState {
    /// Next scheduled run time (ms since epoch)
    pub next_run_at_ms: Option<i64>,
    /// Last run time
    pub last_run_at_ms: Option<i64>,
    /// Last status
    #[serde(default)]
    pub last_status: Option<String>,
    /// Last error message
    #[serde(default)]
    pub last_error: Option<String>,
}

impl JobState {
    /// Create a new job state
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a job state with next run time
    pub fn with_next_run(next_run_at_ms: i64) -> Self {
        Self {
            next_run_at_ms: Some(next_run_at_ms),
            last_run_at_ms: None,
            last_status: None,
            last_error: None,
        }
    }
}

/// A scheduled job
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Job {
    /// Job ID
    pub id: String,
    /// Job name
    pub name: String,
    /// Whether the job is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Schedule
    pub schedule: Schedule,
    /// Payload
    pub payload: Payload,
    /// Job state
    #[serde(default)]
    pub state: JobState,
    /// Created at (ms since epoch)
    pub created_at_ms: i64,
    /// Updated at
    pub updated_at_ms: i64,
    /// Delete after one run
    #[serde(default)]
    pub delete_after_run: bool,
}

fn default_true() -> bool {
    true
}

impl Job {
    /// Create a new job
    pub fn new(name: impl Into<String>, schedule: Schedule, payload: Payload) -> Self {
        let now = Local::now().timestamp_millis();
        Self {
            id: Uuid::new_v4().to_string()[..8].to_string(),
            name: name.into(),
            enabled: true,
            schedule,
            payload,
            state: JobState::default(),
            created_at_ms: now,
            updated_at_ms: now,
            delete_after_run: false,
        }
    }

    /// Create a one-shot job that runs at a specific time
    pub fn one_shot(
        name: impl Into<String>,
        at_ms: i64,
        payload: Payload,
        delete_after_run: bool,
    ) -> Self {
        let mut job = Self::new(name, Schedule::At { at_ms }, payload);
        job.delete_after_run = delete_after_run;
        job
    }

    /// Create a recurring job that runs every N milliseconds
    pub fn recurring(name: impl Into<String>, every_ms: i64, payload: Payload) -> Self {
        Self::new(name, Schedule::Every { every_ms }, payload)
    }

    /// Compute next run time
    pub fn compute_next_run(&self) -> Option<i64> {
        let now = Local::now().timestamp_millis();

        match &self.schedule {
            Schedule::At { at_ms } => {
                if *at_ms > now {
                    Some(*at_ms)
                } else {
                    None
                }
            }
            Schedule::Every { every_ms } => Some(now + every_ms),
            Schedule::Cron { expr } => {
                // Parse cron expression and get next occurrence
                // For simplicity, using a basic implementation
                if let Ok(schedule) = cron_parser::parse(expr, Local::now()) {
                    Some(schedule.timestamp_millis())
                } else {
                    None
                }
            }
        }
    }

    /// Check if job is due to run
    pub fn is_due(&self) -> bool {
        if !self.enabled {
            return false;
        }

        let now = Local::now().timestamp_millis();
        self.state.next_run_at_ms.map(|t| now >= t).unwrap_or(false)
    }

    /// Check if job is due at a specific time (for testing)
    pub fn is_due_at(&self, timestamp_ms: i64) -> bool {
        if !self.enabled {
            return false;
        }
        self.state
            .next_run_at_ms
            .map(|t| timestamp_ms >= t)
            .unwrap_or(false)
    }

    /// Set enabled state
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled {
            self.state.next_run_at_ms = self.compute_next_run();
        } else {
            self.state.next_run_at_ms = None;
        }
        self.updated_at_ms = Local::now().timestamp_millis();
    }
}

/// Job store
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct JobStore {
    pub version: u32,
    pub jobs: Vec<Job>,
}

impl JobStore {
    pub fn new() -> Self {
        Self {
            version: 1,
            jobs: Vec::new(),
        }
    }

    /// Add a job to the store
    pub fn add_job(&mut self, mut job: Job) {
        job.state.next_run_at_ms = job.compute_next_run();
        self.jobs.push(job);
    }

    /// Remove a job by ID
    pub fn remove_job(&mut self, id: &str) -> bool {
        let before = self.jobs.len();
        self.jobs.retain(|j| j.id != id);
        self.jobs.len() < before
    }

    /// Find a job by ID
    pub fn find_job(&self, id: &str) -> Option<&Job> {
        self.jobs.iter().find(|j| j.id == id)
    }

    /// Find a job by ID (mutable)
    pub fn find_job_mut(&mut self, id: &str) -> Option<&mut Job> {
        self.jobs.iter_mut().find(|j| j.id == id)
    }

    /// Get all jobs
    pub fn list_jobs(&self, include_disabled: bool) -> Vec<&Job> {
        self.jobs
            .iter()
            .filter(|j| include_disabled || j.enabled)
            .collect()
    }

    /// Get due jobs
    pub fn get_due_jobs(&self) -> Vec<&Job> {
        self.jobs.iter().filter(|j| j.is_due()).collect()
    }

    /// Get the number of jobs
    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }
}

/// Cron service for managing scheduled tasks
pub struct CronService {
    store_path: PathBuf,
    store: JobStore,
}

impl CronService {
    /// Create a new cron service
    pub fn new(store_path: impl AsRef<Path>) -> Self {
        let store_path = store_path.as_ref().to_path_buf();
        let store = JobStore::new();

        Self { store_path, store }
    }

    /// Load jobs from disk
    pub async fn load(&mut self) -> std::io::Result<()> {
        if !self.store_path.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&self.store_path).await?;
        self.store = serde_json::from_str(&content)?;
        info!("Loaded {} cron jobs", self.store.jobs.len());
        Ok(())
    }

    /// Save jobs to disk
    pub async fn save(&self) -> std::io::Result<()> {
        if let Some(parent) = self.store_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(&self.store)?;
        tokio::fs::write(&self.store_path, content).await?;
        debug!("Saved {} cron jobs", self.store.jobs.len());
        Ok(())
    }

    /// Add a new job
    pub async fn add_job(&mut self, mut job: Job) -> &Job {
        job.state.next_run_at_ms = job.compute_next_run();
        self.store.jobs.push(job);
        let _ = self.save().await;
        self.store.jobs.last().unwrap()
    }

    /// Remove a job by ID
    pub async fn remove_job(&mut self, id: &str) -> bool {
        let before = self.store.jobs.len();
        self.store.jobs.retain(|j| j.id != id);
        let removed = self.store.jobs.len() < before;
        if removed {
            let _ = self.save().await;
        }
        removed
    }

    /// Get all jobs
    pub fn list_jobs(&self, include_disabled: bool) -> Vec<&Job> {
        self.store
            .jobs
            .iter()
            .filter(|j| include_disabled || j.enabled)
            .collect()
    }

    /// Enable/disable a job
    pub async fn enable_job(&mut self, id: &str, enabled: bool) -> Option<Job> {
        let job_index = self.store.jobs.iter().position(|j| j.id == id)?;
        {
            let job = &mut self.store.jobs[job_index];
            job.enabled = enabled;
            if enabled {
                job.state.next_run_at_ms = job.compute_next_run();
            } else {
                job.state.next_run_at_ms = None;
            }
            job.updated_at_ms = Local::now().timestamp_millis();
        }
        let job = self.store.jobs[job_index].clone();
        let _ = self.save().await;
        Some(job)
    }

    /// Get due jobs
    pub fn get_due_jobs(&self) -> Vec<&Job> {
        self.store.jobs.iter().filter(|j| j.is_due()).collect()
    }

    /// Update job after execution
    pub async fn update_after_run(&mut self, id: &str, status: &str, error: Option<&str>) {
        let now = Local::now().timestamp_millis();

        if let Some(job) = self.store.jobs.iter_mut().find(|j| j.id == id) {
            job.state.last_run_at_ms = Some(now);
            job.state.last_status = Some(status.to_string());
            job.state.last_error = error.map(|e| e.to_string());
            job.updated_at_ms = now;

            // Compute next run
            if matches!(job.schedule, Schedule::At { .. }) {
                if job.delete_after_run {
                    self.store.jobs.retain(|j| j.id != id);
                } else {
                    job.enabled = false;
                    job.state.next_run_at_ms = None;
                }
            } else {
                job.state.next_run_at_ms = job.compute_next_run();
            }

            let _ = self.save().await;
        }
    }

    /// Get a reference to the store
    pub fn store(&self) -> &JobStore {
        &self.store
    }

    /// Get a mutable reference to the store
    pub fn store_mut(&mut self) -> &mut JobStore {
        &mut self.store
    }

    /// Get store path
    pub fn store_path(&self) -> &Path {
        &self.store_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tokio::fs;

    // ============ Schedule Tests ============

    #[test]
    fn test_schedule_at_creation() {
        let at_ms = 1_700_000_000_000i64;
        let schedule = Schedule::At { at_ms };

        match schedule {
            Schedule::At { at_ms: val } => assert_eq!(val, at_ms),
            _ => panic!("Expected At schedule"),
        }
    }

    #[test]
    fn test_schedule_every_creation() {
        let every_ms = 3_600_000i64; // 1 hour
        let schedule = Schedule::Every { every_ms };

        match schedule {
            Schedule::Every { every_ms: val } => assert_eq!(val, every_ms),
            _ => panic!("Expected Every schedule"),
        }
    }

    #[test]
    fn test_schedule_cron_creation() {
        let expr = "0 0 * * *".to_string(); // Daily at midnight
        let schedule = Schedule::Cron { expr: expr.clone() };

        match schedule {
            Schedule::Cron { expr: val } => assert_eq!(val, expr),
            _ => panic!("Expected Cron schedule"),
        }
    }

    #[test]
    fn test_schedule_serialization() {
        let schedule = Schedule::Every { every_ms: 5000 };
        let json = serde_json::to_string(&schedule).unwrap();
        assert!(json.contains("\"kind\":\"every\""));
        assert!(json.contains("\"every_ms\":5000"));

        let deserialized: Schedule = serde_json::from_str(&json).unwrap();
        assert_eq!(schedule, deserialized);
    }

    #[test]
    fn test_schedule_deserialization_all_variants() {
        // At schedule
        let at_json = r#"{"kind":"at","at_ms":1700000000000}"#;
        let at: Schedule = serde_json::from_str(at_json).unwrap();
        assert!(matches!(
            at,
            Schedule::At {
                at_ms: 1700000000000
            }
        ));

        // Every schedule
        let every_json = r#"{"kind":"every","every_ms":3600000}"#;
        let every: Schedule = serde_json::from_str(every_json).unwrap();
        assert!(matches!(every, Schedule::Every { every_ms: 3600000 }));

        // Cron schedule
        let cron_json = r#"{"kind":"cron","expr":"0 0 * * *"}"#;
        let cron: Schedule = serde_json::from_str(cron_json).unwrap();
        assert!(matches!(cron, Schedule::Cron { expr } if expr == "0 0 * * *"));
    }

    // ============ Payload Tests ============

    #[test]
    fn test_payload_creation() {
        let payload = Payload::new("test message");
        assert_eq!(payload.message, "test message");
        assert!(!payload.deliver);
        assert!(payload.channel.is_none());
        assert!(payload.to.is_none());
    }

    #[test]
    fn test_payload_builder() {
        let payload = Payload::new("hello")
            .with_deliver(true)
            .with_channel("general")
            .with_to("user123");

        assert_eq!(payload.message, "hello");
        assert!(payload.deliver);
        assert_eq!(payload.channel, Some("general".to_string()));
        assert_eq!(payload.to, Some("user123".to_string()));
    }

    #[test]
    fn test_payload_serialization() {
        let payload = Payload::new("test").with_deliver(true).with_channel("ch1");

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("\"deliver\":true"));
        assert!(json.contains("ch1"));

        let deserialized: Payload = serde_json::from_str(&json).unwrap();
        assert_eq!(payload, deserialized);
    }

    #[test]
    fn test_payload_default() {
        let payload: Payload = serde_json::from_str(r#"{"message":"simple"}"#).unwrap();
        assert_eq!(payload.message, "simple");
        assert!(!payload.deliver);
        assert!(payload.channel.is_none());
        assert!(payload.to.is_none());
    }

    // ============ JobState Tests ============

    #[test]
    fn test_job_state_default() {
        let state = JobState::default();
        assert!(state.next_run_at_ms.is_none());
        assert!(state.last_run_at_ms.is_none());
        assert!(state.last_status.is_none());
        assert!(state.last_error.is_none());
    }

    #[test]
    fn test_job_state_new() {
        let state = JobState::new();
        assert!(state.next_run_at_ms.is_none());
        assert!(state.last_run_at_ms.is_none());
    }

    #[test]
    fn test_job_state_with_next_run() {
        let state = JobState::with_next_run(1_700_000_000_000);
        assert_eq!(state.next_run_at_ms, Some(1_700_000_000_000));
        assert!(state.last_run_at_ms.is_none());
    }

    #[test]
    fn test_job_state_serialization() {
        let state = JobState {
            next_run_at_ms: Some(1_700_000_000_000),
            last_run_at_ms: Some(1_699_999_000_000),
            last_status: Some("success".to_string()),
            last_error: Some("error msg".to_string()),
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: JobState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, deserialized);
    }

    // ============ Job Tests ============

    #[test]
    fn test_job_new() {
        let schedule = Schedule::Every { every_ms: 5000 };
        let payload = Payload::new("test");
        let job = Job::new("my_job", schedule.clone(), payload.clone());

        assert_eq!(job.name, "my_job");
        assert_eq!(job.schedule, schedule);
        assert_eq!(job.payload, payload);
        assert!(job.enabled);
        assert!(!job.delete_after_run);
        assert!(job.state.next_run_at_ms.is_none()); // Not set until compute_next_run
        assert_eq!(job.id.len(), 8); // UUID prefix length
    }

    #[test]
    fn test_job_one_shot() {
        let at_ms = 1_700_000_000_000i64;
        let payload = Payload::new("one time");
        let job = Job::one_shot("one_shot_job", at_ms, payload, true);

        assert_eq!(job.name, "one_shot_job");
        assert!(matches!(job.schedule, Schedule::At { at_ms: val } if val == at_ms));
        assert!(job.delete_after_run);
    }

    #[test]
    fn test_job_one_shot_keep() {
        let at_ms = 1_700_000_000_000i64;
        let payload = Payload::new("one time");
        let job = Job::one_shot("one_shot_job", at_ms, payload, false);

        assert!(!job.delete_after_run);
    }

    #[test]
    fn test_job_recurring() {
        let payload = Payload::new("recurring task");
        let job = Job::recurring("recurring_job", 3_600_000, payload);

        assert_eq!(job.name, "recurring_job");
        assert!(matches!(
            job.schedule,
            Schedule::Every { every_ms: 3600000 }
        ));
        assert!(!job.delete_after_run);
    }

    #[test]
    fn test_job_set_enabled() {
        let mut job = Job::new(
            "test",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );

        // Initially enabled with no next_run
        assert!(job.enabled);
        assert!(job.state.next_run_at_ms.is_none());

        // Disable the job
        job.set_enabled(false);
        assert!(!job.enabled);
        assert!(job.state.next_run_at_ms.is_none());

        // Re-enable the job
        job.set_enabled(true);
        assert!(job.enabled);
        assert!(job.state.next_run_at_ms.is_some());
    }

    // ============ Job.compute_next_run() Tests ============

    #[test]
    fn test_compute_next_run_at_future() {
        let future = Local::now().timestamp_millis() + 3_600_000; // 1 hour from now
        let job = Job::new("test", Schedule::At { at_ms: future }, Payload::new("msg"));

        let next_run = job.compute_next_run();
        assert_eq!(next_run, Some(future));
    }

    #[test]
    fn test_compute_next_run_at_past() {
        let past = Local::now().timestamp_millis() - 3_600_000; // 1 hour ago
        let job = Job::new("test", Schedule::At { at_ms: past }, Payload::new("msg"));

        let next_run = job.compute_next_run();
        assert!(next_run.is_none()); // Past time returns None
    }

    #[test]
    fn test_compute_next_run_every() {
        let every_ms = 5000i64;
        let job = Job::new("test", Schedule::Every { every_ms }, Payload::new("msg"));

        let before = Local::now().timestamp_millis();
        let next_run = job.compute_next_run();
        let after = Local::now().timestamp_millis();

        assert!(next_run.is_some());
        let next_run = next_run.unwrap();
        assert!(next_run >= before + every_ms);
        assert!(next_run <= after + every_ms);
    }

    #[test]
    fn test_compute_next_run_cron() {
        // Cron expression for "every minute"
        let job = Job::new(
            "test",
            Schedule::Cron {
                expr: "* * * * *".to_string(),
            },
            Payload::new("msg"),
        );

        let now = Local::now().timestamp_millis();
        let next_run = job.compute_next_run();

        assert!(next_run.is_some());
        // Next run should be within the next 60 seconds
        assert!(next_run.unwrap() > now);
        assert!(next_run.unwrap() <= now + 60_000);
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn test_compute_next_run_cron_invalid() {
        // cron-parser library panics on invalid expressions
        let job = Job::new(
            "test",
            Schedule::Cron {
                expr: "invalid".to_string(),
            },
            Payload::new("msg"),
        );

        let _next_run = job.compute_next_run();
    }

    // ============ Job.is_due() Tests ============

    #[test]
    fn test_is_due_disabled() {
        let mut job = Job::new(
            "test",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );
        job.state.next_run_at_ms = Some(Local::now().timestamp_millis() - 1000);
        job.enabled = false;

        assert!(!job.is_due());
    }

    #[test]
    fn test_is_due_no_next_run() {
        let job = Job::new(
            "test",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );
        // next_run_at_ms is None

        assert!(!job.is_due());
    }

    #[test]
    fn test_is_due_future() {
        let mut job = Job::new(
            "test",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );
        job.state.next_run_at_ms = Some(Local::now().timestamp_millis() + 3_600_000); // 1 hour from now

        assert!(!job.is_due());
    }

    #[test]
    fn test_is_due_now() {
        let mut job = Job::new(
            "test",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );
        job.state.next_run_at_ms = Some(Local::now().timestamp_millis());

        assert!(job.is_due());
    }

    #[test]
    fn test_is_due_past() {
        let mut job = Job::new(
            "test",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );
        job.state.next_run_at_ms = Some(Local::now().timestamp_millis() - 1000); // 1 second ago

        assert!(job.is_due());
    }

    #[test]
    fn test_is_due_at_specific_time() {
        let mut job = Job::new(
            "test",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );
        job.state.next_run_at_ms = Some(1000);

        assert!(!job.is_due_at(500)); // Before due time
        assert!(job.is_due_at(1000)); // At due time
        assert!(job.is_due_at(1500)); // After due time
    }

    // ============ JobStore Tests ============

    #[test]
    fn test_job_store_new() {
        let store = JobStore::new();
        assert_eq!(store.version, 1);
        assert!(store.jobs.is_empty());
    }

    #[test]
    fn test_job_store_add_job() {
        let mut store = JobStore::new();
        let job = Job::new(
            "test",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );

        store.add_job(job);
        assert_eq!(store.len(), 1);
        assert!(store.jobs[0].state.next_run_at_ms.is_some());
    }

    #[test]
    fn test_job_store_remove_job() {
        let mut store = JobStore::new();
        let job = Job::new(
            "test",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );
        let id = job.id.clone();

        store.add_job(job);
        assert_eq!(store.len(), 1);

        let removed = store.remove_job(&id);
        assert!(removed);
        assert!(store.is_empty());

        // Removing non-existent job
        let removed = store.remove_job("nonexistent");
        assert!(!removed);
    }

    #[test]
    fn test_job_store_find_job() {
        let mut store = JobStore::new();
        let job = Job::new(
            "test",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );
        let id = job.id.clone();

        store.add_job(job);

        let found = store.find_job(&id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test");

        let not_found = store.find_job("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_job_store_find_job_mut() {
        let mut store = JobStore::new();
        let job = Job::new(
            "test",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );
        let id = job.id.clone();

        store.add_job(job);

        if let Some(job) = store.find_job_mut(&id) {
            job.name = "modified".to_string();
        }

        assert_eq!(store.find_job(&id).unwrap().name, "modified");
    }

    #[test]
    fn test_job_store_list_jobs() {
        let mut store = JobStore::new();

        // Add enabled job
        let job1 = Job::new(
            "job1",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg1"),
        );
        store.add_job(job1);

        // Add disabled job
        let mut job2 = Job::new(
            "job2",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg2"),
        );
        job2.enabled = false;
        store.add_job(job2);

        // List only enabled
        let enabled = store.list_jobs(false);
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].name, "job1");

        // List all
        let all = store.list_jobs(true);
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_job_store_get_due_jobs() {
        let mut store = JobStore::new();

        // Add due job (past next_run)
        let mut job1 = Job::new(
            "due",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg1"),
        );
        job1.state.next_run_at_ms = Some(Local::now().timestamp_millis() - 1000);
        store.jobs.push(job1);

        // Add non-due job (future next_run)
        let mut job2 = Job::new(
            "not_due",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg2"),
        );
        job2.state.next_run_at_ms = Some(Local::now().timestamp_millis() + 3_600_000);
        store.jobs.push(job2);

        // Add disabled job (also past next_run)
        let mut job3 = Job::new(
            "disabled",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg3"),
        );
        job3.state.next_run_at_ms = Some(Local::now().timestamp_millis() - 1000);
        job3.enabled = false;
        store.jobs.push(job3);

        let due = store.get_due_jobs();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].name, "due");
    }

    #[test]
    fn test_job_store_serialization() {
        let mut store = JobStore::new();
        let job = Job::new(
            "test",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );
        store.add_job(job);

        let json = serde_json::to_string_pretty(&store).unwrap();
        let deserialized: JobStore = serde_json::from_str(&json).unwrap();

        assert_eq!(store.version, deserialized.version);
        assert_eq!(store.len(), deserialized.len());
    }

    // ============ CronService Tests ============

    #[tokio::test]
    async fn test_cron_service_new() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store_path = temp_dir.path().join("cron.json");

        let service = CronService::new(&store_path);
        assert_eq!(service.store_path(), store_path);
        assert!(service.store().is_empty());
    }

    #[tokio::test]
    async fn test_cron_service_save_and_load() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store_path = temp_dir.path().join("cron.json");

        // Create and save
        {
            let mut service = CronService::new(&store_path);
            let job = Job::new(
                "test_job",
                Schedule::Every { every_ms: 5000 },
                Payload::new("msg"),
            );
            service.store_mut().add_job(job);
            service.save().await.unwrap();
        }

        // Verify file exists
        assert!(store_path.exists());

        // Load and verify
        {
            let mut service = CronService::new(&store_path);
            service.load().await.unwrap();
            assert_eq!(service.store().len(), 1);
            assert_eq!(service.store().jobs[0].name, "test_job");
        }
    }

    #[tokio::test]
    async fn test_cron_service_load_nonexistent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store_path = temp_dir.path().join("nonexistent").join("cron.json");

        let mut service = CronService::new(&store_path);
        let result = service.load().await;
        assert!(result.is_ok()); // Should succeed if file doesn't exist
        assert!(service.store().is_empty());
    }

    #[tokio::test]
    async fn test_cron_service_add_job() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store_path = temp_dir.path().join("cron.json");

        let mut service = CronService::new(&store_path);
        let job = Job::new(
            "new_job",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );

        let added = service.add_job(job).await;
        assert_eq!(added.name, "new_job");
        assert!(added.state.next_run_at_ms.is_some());
        assert_eq!(service.store().len(), 1);

        // Verify saved to disk
        let content = fs::read_to_string(&store_path).await.unwrap();
        assert!(content.contains("new_job"));
    }

    #[tokio::test]
    async fn test_cron_service_remove_job() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store_path = temp_dir.path().join("cron.json");

        let mut service = CronService::new(&store_path);
        let job = Job::new(
            "to_remove",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );
        let id = job.id.clone();

        service.add_job(job).await;
        assert_eq!(service.store().len(), 1);

        let removed = service.remove_job(&id).await;
        assert!(removed);
        assert!(service.store().is_empty());

        // Try removing again
        let removed = service.remove_job(&id).await;
        assert!(!removed);
    }

    #[tokio::test]
    async fn test_cron_service_list_jobs() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store_path = temp_dir.path().join("cron.json");

        let mut service = CronService::new(&store_path);

        // Add enabled job
        let job1 = Job::new(
            "enabled_job",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg1"),
        );
        service.add_job(job1).await;

        // Add disabled job
        let mut job2 = Job::new(
            "disabled_job",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg2"),
        );
        job2.enabled = false;
        service.add_job(job2).await;

        let enabled = service.list_jobs(false);
        assert_eq!(enabled.len(), 1);

        let all = service.list_jobs(true);
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_cron_service_enable_job() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store_path = temp_dir.path().join("cron.json");

        let mut service = CronService::new(&store_path);
        let mut job = Job::new(
            "test",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );
        job.enabled = false;
        job.state.next_run_at_ms = None;
        let id = job.id.clone();

        service.add_job(job).await;

        // Enable the job
        let enabled = service.enable_job(&id, true).await;
        assert!(enabled.is_some());
        assert!(enabled.unwrap().enabled);
        assert!(service.store().jobs[0].state.next_run_at_ms.is_some());

        // Disable the job
        let disabled = service.enable_job(&id, false).await;
        assert!(disabled.is_some());
        assert!(!disabled.unwrap().enabled);
        assert!(service.store().jobs[0].state.next_run_at_ms.is_none());

        // Try enabling non-existent job
        let not_found = service.enable_job("nonexistent", true).await;
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_cron_service_get_due_jobs() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store_path = temp_dir.path().join("cron.json");

        let mut service = CronService::new(&store_path);

        // Add due job
        let mut job1 = Job::new(
            "due",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg1"),
        );
        job1.state.next_run_at_ms = Some(Local::now().timestamp_millis() - 1000);
        service.store_mut().jobs.push(job1);

        // Add non-due job
        let mut job2 = Job::new(
            "not_due",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg2"),
        );
        job2.state.next_run_at_ms = Some(Local::now().timestamp_millis() + 3_600_000);
        service.store_mut().jobs.push(job2);

        let due = service.get_due_jobs();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].name, "due");
    }

    #[tokio::test]
    async fn test_cron_service_update_after_run_recurring() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store_path = temp_dir.path().join("cron.json");

        let mut service = CronService::new(&store_path);
        let job = Job::new(
            "recurring",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );
        let id = job.id.clone();

        service.add_job(job).await;
        let original_next_run = service.store().jobs[0].state.next_run_at_ms;

        // Update after successful run
        service.update_after_run(&id, "success", None).await;

        let job = &service.store().jobs[0];
        assert_eq!(job.state.last_status, Some("success".to_string()));
        assert!(job.state.last_run_at_ms.is_some());
        assert!(job.state.last_error.is_none());
        assert!(job.enabled); // Recurring jobs stay enabled
        assert!(job.state.next_run_at_ms.is_some());
        // Next run should be >= original (may be equal if test runs fast)
        assert!(job.state.next_run_at_ms.unwrap() >= original_next_run.unwrap());
    }

    #[tokio::test]
    async fn test_cron_service_update_after_run_one_shot_keep() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store_path = temp_dir.path().join("cron.json");

        let mut service = CronService::new(&store_path);
        let future = Local::now().timestamp_millis() + 3_600_000;
        let job = Job::one_shot("one_shot_keep", future, Payload::new("msg"), false);
        let id = job.id.clone();

        service.add_job(job).await;
        service.update_after_run(&id, "success", None).await;

        // Job should be disabled but not deleted
        let job = &service.store().jobs[0];
        assert!(!job.enabled);
        assert!(job.state.next_run_at_ms.is_none());
    }

    #[tokio::test]
    async fn test_cron_service_update_after_run_one_shot_delete() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store_path = temp_dir.path().join("cron.json");

        let mut service = CronService::new(&store_path);
        let future = Local::now().timestamp_millis() + 3_600_000;
        let job = Job::one_shot("one_shot_delete", future, Payload::new("msg"), true);
        let id = job.id.clone();

        service.add_job(job).await;
        assert_eq!(service.store().len(), 1);

        service.update_after_run(&id, "success", None).await;

        // Job should be deleted
        assert!(service.store().is_empty());
    }

    #[tokio::test]
    async fn test_cron_service_update_after_run_with_error() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store_path = temp_dir.path().join("cron.json");

        let mut service = CronService::new(&store_path);
        let job = Job::new(
            "test",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );
        let id = job.id.clone();

        service.add_job(job).await;
        service
            .update_after_run(&id, "failed", Some("error message"))
            .await;

        let job = &service.store().jobs[0];
        assert_eq!(job.state.last_status, Some("failed".to_string()));
        assert_eq!(job.state.last_error, Some("error message".to_string()));
    }

    #[tokio::test]
    async fn test_cron_service_update_after_run_nonexistent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store_path = temp_dir.path().join("cron.json");

        let mut service = CronService::new(&store_path);
        let job = Job::new(
            "test",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );
        service.add_job(job).await;

        // Should not panic or fail
        service
            .update_after_run("nonexistent", "success", None)
            .await;
        assert_eq!(service.store().len(), 1);
    }

    // ============ Integration Tests ============

    #[tokio::test]
    async fn test_full_workflow() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store_path = temp_dir.path().join("cron.json");

        // Create service and add jobs
        let mut service = CronService::new(&store_path);

        // Add recurring job
        let recurring = Job::recurring(
            "daily_backup",
            24 * 3_600_000, // Every day
            Payload::new("backup database"),
        );
        let recurring_id = recurring.id.clone();
        service.add_job(recurring).await;

        // Add one-shot job
        let future = Local::now().timestamp_millis() + 60_000; // 1 minute from now
        let one_shot = Job::one_shot(
            "send_notification",
            future,
            Payload::new("hello user").with_deliver(true),
            false,
        );
        let one_shot_id = one_shot.id.clone();
        service.add_job(one_shot).await;

        // Verify jobs were added
        assert_eq!(service.store().len(), 2);

        // Simulate job execution
        service
            .update_after_run(&recurring_id, "success", None)
            .await;
        service
            .update_after_run(&one_shot_id, "success", None)
            .await;

        // Recurring job should still exist with new next_run
        let recurring_job = service.store().find_job(&recurring_id);
        assert!(recurring_job.is_some());
        assert!(recurring_job.unwrap().enabled);

        // One-shot job should be disabled
        let one_shot_job = service.store().find_job(&one_shot_id);
        assert!(one_shot_job.is_some());
        assert!(!one_shot_job.unwrap().enabled);

        // Save and reload
        service.save().await.unwrap();

        let mut new_service = CronService::new(&store_path);
        new_service.load().await.unwrap();
        assert_eq!(new_service.store().len(), 2);
    }

    #[test]
    fn test_job_equality() {
        let job1 = Job::new(
            "test",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );
        let mut job2 = job1.clone();

        assert_eq!(job1, job2);

        job2.name = "different".to_string();
        assert_ne!(job1, job2);
    }

    #[test]
    fn test_schedule_equality() {
        let s1 = Schedule::Every { every_ms: 5000 };
        let s2 = Schedule::Every { every_ms: 5000 };
        let s3 = Schedule::Every { every_ms: 10000 };

        assert_eq!(s1, s2);
        assert_ne!(s1, s3);
    }

    #[test]
    fn test_payload_equality() {
        let p1 = Payload::new("test").with_deliver(true);
        let p2 = Payload::new("test").with_deliver(true);
        let p3 = Payload::new("different");

        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_default_job_enabled() {
        let job = Job::new(
            "test",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );
        assert!(job.enabled);
    }

    #[test]
    fn test_job_timestamps() {
        let before = Local::now().timestamp_millis();
        let job = Job::new(
            "test",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );
        let after = Local::now().timestamp_millis();

        assert!(job.created_at_ms >= before);
        assert!(job.created_at_ms <= after);
        assert_eq!(job.created_at_ms, job.updated_at_ms);
    }

    #[tokio::test]
    async fn test_concurrent_add_and_remove() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store_path = temp_dir.path().join("cron.json");

        let mut service = CronService::new(&store_path);
        let mut ids = Vec::new();

        // Add multiple jobs
        for i in 0..10 {
            let job = Job::new(
                format!("job_{}", i),
                Schedule::Every {
                    every_ms: 1000 * (i + 1) as i64,
                },
                Payload::new(format!("msg {}", i)),
            );
            ids.push(job.id.clone());
            service.add_job(job).await;
        }

        assert_eq!(service.store().len(), 10);

        // Remove half of them
        for id in ids.iter().take(5) {
            service.remove_job(id).await;
        }

        assert_eq!(service.store().len(), 5);

        // Verify remaining jobs
        for id in ids.iter().take(10).skip(5) {
            assert!(service.store().find_job(id).is_some());
        }
    }

    #[tokio::test]
    async fn test_cron_service_persists_version() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store_path = temp_dir.path().join("cron.json");

        let mut service = CronService::new(&store_path);
        service.store_mut().version = 42;
        let job = Job::new(
            "test",
            Schedule::Every { every_ms: 5000 },
            Payload::new("msg"),
        );
        service.add_job(job).await;

        let mut new_service = CronService::new(&store_path);
        new_service.load().await.unwrap();

        assert_eq!(new_service.store().version, 42);
    }
}
