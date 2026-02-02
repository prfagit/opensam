//! Common test utilities for OpenSAM integration tests
#![allow(dead_code)]

use assert_cmd::Command;
use std::path::PathBuf;
use tempfile::{tempdir, TempDir};

/// Path to the opensam binary
pub fn bin_path() -> PathBuf {
    env!("CARGO_BIN_EXE_opensam").into()
}

/// Create a test environment with isolated config directory
pub struct TestEnv {
    pub temp_dir: TempDir,
    pub config_dir: PathBuf,
    pub workspace_dir: PathBuf,
}

impl TestEnv {
    /// Create a new test environment
    pub fn new() -> anyhow::Result<Self> {
        let temp_dir = tempdir()?;
        let config_dir = temp_dir.path().join(".opensam");
        let workspace_dir = temp_dir.path().join("workspace");

        std::fs::create_dir_all(&config_dir)?;

        Ok(Self {
            temp_dir,
            config_dir,
            workspace_dir,
        })
    }

    /// Get the path to a file in the config directory
    pub fn config_file(&self, name: &str) -> PathBuf {
        self.config_dir.join(name)
    }

    /// Get the path to a file in the workspace directory
    pub fn workspace_file(&self, name: &str) -> PathBuf {
        self.workspace_dir.join(name)
    }

    /// Create a command with environment variables set to use the test environment
    pub fn command(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_opensam"));
        cmd.env("HOME", self.temp_dir.path());
        cmd.env("XDG_CONFIG_HOME", &self.config_dir);
        cmd
    }

    /// Create a basic config file
    pub fn create_config(&self) -> anyhow::Result<()> {
        let config = r#"{
  "api_key": "test-api-key",
  "default_model": "test/model",
  "workspace": "workspace"
}"#;
        std::fs::write(self.config_file("config.json"), config)?;
        Ok(())
    }
}

impl Default for TestEnv {
    fn default() -> Self {
        Self::new().expect("Failed to create test environment")
    }
}
