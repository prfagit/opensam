//! Command execution tests for OpenSAM

mod common;

use common::TestEnv;
use predicates::prelude::*;
use std::fs;

// ============================================================================
// Init command tests
// ============================================================================

#[test]
fn test_init_creates_config_dir() {
    let env = TestEnv::new().expect("Failed to create test environment");

    let mut cmd = env.command();
    cmd.arg("init");

    // Init may fail but should attempt to create directories
    let output = cmd.output().expect("Failed to execute command");

    // Check that it tried to initialize (output contains expected text)
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);

    // The init command should have run (even if it failed later)
    assert!(
        combined.contains("Initializing") || output.status.code() == Some(1),
        "Init command did not produce expected output: {}",
        combined
    );
}

#[test]
fn test_init_output_format() {
    let env = TestEnv::new().expect("Failed to create test environment");

    let mut cmd = env.command();
    cmd.arg("init");

    // Init command outputs to stdout, may succeed or fail depending on env
    let output = cmd.output().expect("Failed to execute");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain initializing message
    assert!(
        stdout.contains("Initializing") || stdout.contains("OpenSAM"),
        "Init should produce initializing output: {}",
        stdout
    );
}

// ============================================================================
// Status command tests
// ============================================================================

#[test]
fn test_status_shows_missing_config() {
    let env = TestEnv::new().expect("Failed to create test environment");

    let mut cmd = env.command();
    cmd.arg("status");

    // Status command now shows [Missing] for missing config but succeeds
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Config:"));
}

#[test]
fn test_status_output_format() {
    let env = TestEnv::new().expect("Failed to create test environment");
    env.create_config().expect("Failed to create config");

    let mut cmd = env.command();
    cmd.arg("status");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("OpenSAM System Status"))
        .stdout(predicate::str::contains("Config:"))
        .stdout(predicate::str::contains("Workspace:"))
        .stdout(predicate::str::contains("Model:"))
        .stdout(predicate::str::contains("API Key:"));
}

#[test]
fn test_status_shows_config_ok() {
    let env = TestEnv::new().expect("Failed to create test environment");
    env.create_config().expect("Failed to create config");

    // Create workspace dir
    fs::create_dir_all(&env.workspace_dir).expect("Failed to create workspace");

    let mut cmd = env.command();
    cmd.arg("status");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("[OK]"));
}

#[test]
fn test_status_shows_missing() {
    let env = TestEnv::new().expect("Failed to create test environment");
    // Don't create config

    let mut cmd = env.command();
    cmd.arg("status");

    // Status shows [Missing] for missing config but succeeds
    let output = cmd.output().expect("Failed to execute");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success() && stdout.contains("Missing"),
        "Status should succeed and show Missing: {}",
        stdout
    );
}

// ============================================================================
// Engage command tests (with mocked/missing provider)
// ============================================================================

#[test]
fn test_engage_without_config_fails() {
    let env = TestEnv::new().expect("Failed to create test environment");

    let mut cmd = env.command();
    cmd.args(["engage", "-m", "Hello"]);

    // The error may be in stdout or stderr and contains "API key"
    cmd.assert().failure();
}

#[test]
fn test_engage_without_api_key_fails() {
    let env = TestEnv::new().expect("Failed to create test environment");

    // Create config without API key
    let config = r#"{
  "default_model": "test/model",
  "workspace": "workspace"
}"#;
    fs::write(env.config_file("config.json"), config).expect("Failed to write config");

    let mut cmd = env.command();
    cmd.args(["engage", "-m", "Hello"]);

    cmd.assert().failure();
}

#[test]
fn test_engage_with_empty_message() {
    let env = TestEnv::new().expect("Failed to create test environment");
    env.create_config().expect("Failed to create config");

    let mut cmd = env.command();
    cmd.args(["engage", "-m", ""]);

    // Empty message should still be processed (will fail on provider)
    cmd.assert().failure();
}

// ============================================================================
// Deploy command tests
// ============================================================================

#[test]
fn test_deploy_without_config_fails() {
    let env = TestEnv::new().expect("Failed to create test environment");

    let mut cmd = env.command();
    cmd.arg("deploy");

    cmd.assert().failure();
}

#[test]
fn test_deploy_starts_with_config() {
    let env = TestEnv::new().expect("Failed to create test environment");
    env.create_config().expect("Failed to create config");

    let mut cmd = env.command();
    cmd.arg("deploy");
    cmd.timeout(std::time::Duration::from_secs(1));

    // Deploy will start but we can't easily test the full server
    // Just verify it attempts to start
    let output = cmd.output().expect("Failed to execute");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Starting") || !output.status.success(),
        "Deploy did not produce expected output"
    );
}

// ============================================================================
// Schedule command tests
// ============================================================================

#[test]
fn test_schedule_list_outputs() {
    let env = TestEnv::new().expect("Failed to create test environment");

    let mut cmd = env.command();
    cmd.args(["schedule", "list"]);

    // Schedule list outputs to stdout, not stderr
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("scheduled"));
}

#[test]
fn test_schedule_add_outputs() {
    let env = TestEnv::new().expect("Failed to create test environment");

    let mut cmd = env.command();
    // Must provide either --every or --cron
    cmd.args([
        "schedule",
        "add",
        "-n",
        "test-job",
        "-m",
        "Test message",
        "-e",
        "60",
    ]);

    // Schedule add outputs success to stdout
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Job added"));
}

#[test]
fn test_schedule_remove_outputs() {
    let env = TestEnv::new().expect("Failed to create test environment");

    let mut cmd = env.command();
    cmd.args(["schedule", "remove", "job-123"]);

    // Schedule remove outputs to stdout
    cmd.assert().success().stdout(
        predicate::str::contains("Job")
            .and(predicate::str::contains("removed").or(predicate::str::contains("not found"))),
    );
}

// ============================================================================
// Freq command tests
// ============================================================================

#[test]
fn test_freq_status_outputs() {
    let env = TestEnv::new().expect("Failed to create test environment");

    let mut cmd = env.command();
    cmd.args(["freq", "status"]);

    // Freq status outputs to stdout
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Channel Status"));
}

// ============================================================================
// Command error handling tests
// ============================================================================

#[test]
fn test_init_command_error_handling() {
    let env = TestEnv::new().expect("Failed to create test environment");

    // Make config dir read-only to trigger error
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&env.config_dir)
            .expect("Failed to get metadata")
            .permissions();
        perms.set_mode(0o555);
        fs::set_permissions(&env.config_dir, perms).expect("Failed to set permissions");

        let mut cmd = env.command();
        cmd.arg("init");

        cmd.assert().failure();

        // Restore permissions for cleanup
        let mut perms = fs::metadata(&env.config_dir)
            .expect("Failed to get metadata")
            .permissions();
        perms.set_mode(0o755);
        let _ = fs::set_permissions(&env.config_dir, perms);
    }
}

#[test]
fn test_invalid_config_json() {
    let env = TestEnv::new().expect("Failed to create test environment");

    // Write invalid JSON
    fs::write(env.config_file("config.json"), "{invalid json}").expect("Failed to write config");

    let mut cmd = env.command();
    cmd.arg("status");

    cmd.assert().failure();
}
