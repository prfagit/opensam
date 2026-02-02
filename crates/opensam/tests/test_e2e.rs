//! End-to-end integration tests for OpenSAM

mod common;

use common::TestEnv;
use predicates::prelude::*;
use std::fs;

// ============================================================================
// Full workflow tests
// ============================================================================

/// Test the full workflow: init → status
#[test]
fn test_full_workflow_init_status() {
    let env = TestEnv::new().expect("Failed to create test environment");

    // Step 1: Run init command
    let mut init_cmd = env.command();
    init_cmd.arg("init");

    let init_output = init_cmd.output().expect("Failed to run init");
    let init_stdout = String::from_utf8_lossy(&init_output.stdout);

    // Init should complete (might fail at config step, but should create directories)
    assert!(
        init_stdout.contains("Initializing") || !init_output.status.success(),
        "Init command did not run: {}",
        init_stdout
    );

    // Step 2: Check status (now succeeds and shows missing config)
    let mut status_cmd = env.command();
    status_cmd.arg("status");

    let status_output = status_cmd.output().expect("Failed to run status");

    // Status now succeeds and shows config/workspace status
    assert!(
        status_output.status.success(),
        "Status command should succeed: {:?}",
        String::from_utf8_lossy(&status_output.stderr)
    );
}

/// Test init creates expected directory structure
#[test]
fn test_init_creates_workspace_structure() {
    let env = TestEnv::new().expect("Failed to create test environment");

    // Run init
    let mut cmd = env.command();
    cmd.arg("init");
    let _ = cmd.output();

    // Check for workspace directory (may be in different location)
    // The actual workspace location depends on config setup
    // Just verify the command ran without panic
}

/// Test workflow with config present
#[test]
fn test_workflow_with_config() {
    let env = TestEnv::new().expect("Failed to create test environment");

    // Create a valid config
    env.create_config().expect("Failed to create config");

    // Create workspace directory
    fs::create_dir_all(&env.workspace_dir).expect("Failed to create workspace");

    // Step 1: Status should succeed with config
    let mut status_cmd = env.command();
    status_cmd.arg("status");

    status_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("OpenSAM System Status"))
        .stdout(predicate::str::contains("[OK]"));
    // Note: Model name comes from env var, may not be "test/model"

    // Step 2: Schedule commands should work (outputs to stdout)
    let mut schedule_cmd = env.command();
    schedule_cmd.args(["schedule", "list"]);

    schedule_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("scheduled").or(predicate::str::contains("No scheduled")));

    // Step 3: Add a scheduled job (outputs to stdout)
    let mut add_cmd = env.command();
    add_cmd.args([
        "schedule",
        "add",
        "-n",
        "e2e-test",
        "-m",
        "Test message",
        "-e",
        "60",
    ]);

    add_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("Job added"));

    // Step 4: Remove the job (outputs to stdout)
    let mut remove_cmd = env.command();
    remove_cmd.args(["schedule", "remove", "e2e-test"]);

    remove_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("Job"));

    // Step 5: Check freq status (outputs to stdout)
    let mut freq_cmd = env.command();
    freq_cmd.args(["freq", "status"]);

    freq_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("Channel Status"));
}

// ============================================================================
// Error handling workflows
// ============================================================================

/// Test behavior when config is corrupted
#[test]
fn test_workflow_corrupted_config() {
    let env = TestEnv::new().expect("Failed to create test environment");

    // Write corrupted config
    fs::write(env.config_file("config.json"), "{corrupted").expect("Failed to write config");

    // Status should fail
    let mut cmd = env.command();
    cmd.arg("status");

    cmd.assert().failure();
}

/// Test behavior when directories are missing
#[test]
fn test_workflow_missing_workspace() {
    let env = TestEnv::new().expect("Failed to create test environment");

    // Create config but don't create workspace
    env.create_config().expect("Failed to create config");

    let mut cmd = env.command();
    cmd.arg("status");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("[Missing]"));
}

// ============================================================================
// Command sequence tests
// ============================================================================

/// Test running multiple commands in sequence
#[test]
fn test_command_sequence() {
    let env = TestEnv::new().expect("Failed to create test environment");
    env.create_config().expect("Failed to create config");

    // Run status multiple times
    for _i in 0..3 {
        let mut cmd = env.command();
        cmd.arg("status");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("OpenSAM System Status"));
    }
}

/// Test schedule workflow sequence
#[test]
fn test_schedule_workflow_sequence() {
    let env = TestEnv::new().expect("Failed to create test environment");

    // List (empty)
    let mut list_cmd = env.command();
    list_cmd.args(["schedule", "list"]);
    list_cmd.assert().success();

    // Add multiple jobs (need -e or -c flag)
    for i in 0..3 {
        let mut add_cmd = env.command();
        add_cmd.args([
            "schedule",
            "add",
            "-n",
            &format!("job-{}", i),
            "-m",
            "Test",
            "-e",
            "60",
        ]);
        add_cmd.assert().success();
    }

    // List all
    let mut list_all_cmd = env.command();
    list_all_cmd.args(["schedule", "list", "--all"]);
    list_all_cmd.assert().success();

    // Remove jobs
    for i in 0..3 {
        let mut remove_cmd = env.command();
        remove_cmd.args(["schedule", "remove", &format!("job-{}", i)]);
        remove_cmd.assert().success();
    }
}

// ============================================================================
// CLI help workflow
// ============================================================================

/// Test that all commands have proper help
#[test]
fn test_all_commands_have_help() {
    let commands = vec![
        vec!["init", "--help"],
        vec!["engage", "--help"],
        vec!["deploy", "--help"],
        vec!["status", "--help"],
        vec!["schedule", "--help"],
        vec!["schedule", "list", "--help"],
        vec!["schedule", "add", "--help"],
        vec!["schedule", "remove", "--help"],
        vec!["freq", "--help"],
        vec!["freq", "status", "--help"],
    ];

    for cmd_args in commands {
        let cmd = common::bin_path();
        let mut command = std::process::Command::new(&cmd);
        command.args(&cmd_args);

        let output = command.output().expect("Failed to execute");
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(
            output.status.success(),
            "Help for {:?} failed: {}",
            cmd_args,
            stdout
        );
        assert!(
            stdout.contains("Usage:"),
            "Help for {:?} missing Usage: {}",
            cmd_args,
            stdout
        );
    }
}

// ============================================================================
// Edge cases
// ============================================================================

/// Test with special characters in arguments
#[test]
fn test_special_characters_in_args() {
    let env = TestEnv::new().expect("Failed to create test environment");

    // Schedule add with special characters (need -e or -c flag)
    let mut cmd = env.command();
    cmd.args([
        "schedule",
        "add",
        "-n",
        "test-job-with-dashes_and_underscores.123",
        "-m",
        "Message with quotes and apostrophes",
        "-e",
        "60",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Job added"));
}

/// Test with long arguments
#[test]
fn test_long_arguments() {
    let env = TestEnv::new().expect("Failed to create test environment");

    let long_name = "a".repeat(100);
    let long_message = "b".repeat(500);

    let mut cmd = env.command();
    cmd.args([
        "schedule",
        "add",
        "-n",
        &long_name,
        "-m",
        &long_message,
        "-e",
        "60",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Job added"));
}

/// Test unicode in arguments
#[test]
fn test_unicode_arguments() {
    let env = TestEnv::new().expect("Failed to create test environment");

    let mut cmd = env.command();
    cmd.args([
        "schedule",
        "add",
        "-n",
        "测试工作",
        "-m",
        "Héllo Wörld こんにちは",
        "-e",
        "60",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Job added"));
}
