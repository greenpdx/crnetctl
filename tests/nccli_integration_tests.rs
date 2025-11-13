//! Integration tests for nccli
//!
//! These tests verify the CLI commands work correctly

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test nccli command
fn nccli() -> Command {
    Command::cargo_bin("nccli").unwrap()
}

#[test]
fn test_help_command() {
    nccli()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Network Control CLI"));
}

#[test]
fn test_general_status() {
    // This test may require network access or system capabilities
    let output = nccli()
        .arg("general")
        .arg("status")
        .output()
        .expect("Failed to execute command");

    // If failed due to permissions or system limitations, skip test
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Permission denied")
            || stderr.contains("Operation not permitted")
            || stderr.contains("Not supported")
            || stderr.contains("/sys/class/net not available") {
            eprintln!("Test skipped: requires system access - {}", stderr);
            return;
        }
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("STATE"), "Output should contain STATE");
}

#[test]
fn test_general_status_terse() {
    // This test may require network access
    let result = nccli()
        .arg("-t")
        .arg("general")
        .arg("status")
        .output();

    // If we can't run the command due to permissions, skip
    if let Ok(output) = result {
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("Permission denied") || stderr.contains("Operation not permitted") || stderr.contains("Not supported") || stderr.contains("/sys/class/net") {
                eprintln!("Test skipped: requires elevated privileges");
                return;
            }
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("running") || stdout.contains("enabled"),
                "Output should contain status information");
    }
}

#[test]
fn test_general_permissions() {
    nccli()
        .arg("general")
        .arg("permissions")
        .assert()
        .success()
        .stdout(predicate::str::contains("network.control"));
}

#[test]
fn test_general_permissions_terse() {
    nccli()
        .arg("-t")
        .arg("general")
        .arg("permissions")
        .assert()
        .success()
        .stdout(predicate::str::contains("network.control:yes"));
}

#[test]
fn test_general_logging() {
    nccli()
        .arg("general")
        .arg("logging")
        .assert()
        .success()
        .stdout(predicate::str::contains("LEVEL"));
}

#[test]
fn test_radio_all() {
    nccli()
        .arg("radio")
        .arg("all")
        .assert()
        .success()
        .stdout(predicate::str::contains("WIFI"));
}

#[test]
fn test_radio_all_terse() {
    nccli()
        .arg("-t")
        .arg("radio")
        .arg("all")
        .assert()
        .success()
        .stdout(predicate::str::contains("enabled:enabled"));
}

#[test]
fn test_connection_show_no_connections() {
    nccli()
        .arg("connection")
        .arg("show")
        .assert()
        .success();
}

#[test]
fn test_connection_show_terse() {
    nccli()
        .arg("-t")
        .arg("connection")
        .arg("show")
        .assert()
        .success();
}

#[test]
fn test_connection_show_nonexistent() {
    nccli()
        .arg("connection")
        .arg("show")
        .arg("nonexistent-connection")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_device_status() {
    // Device status may require network access
    let output = nccli()
        .arg("device")
        .arg("status")
        .output()
        .expect("Failed to execute command");

    // If failed due to permissions, skip test
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Permission denied") || stderr.contains("Operation not permitted") || stderr.contains("Not supported") || stderr.contains("/sys/class/net") {
            eprintln!("Test skipped: requires elevated privileges");
            return;
        }
        panic!("Command failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("DEVICE") || stdout.contains("lo"),
            "Output should contain device information");
}

#[test]
fn test_device_status_terse() {
    // Device status may require network access
    let output = nccli()
        .arg("-t")
        .arg("device")
        .arg("status")
        .output()
        .expect("Failed to execute command");

    // If failed due to permissions, skip test
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Permission denied") || stderr.contains("Operation not permitted") || stderr.contains("Not supported") || stderr.contains("/sys/class/net") {
            eprintln!("Test skipped: requires elevated privileges");
            return;
        }
    }

    // Terse output should succeed if we have permissions
    assert!(output.status.success(), "Command should succeed");
}

#[test]
fn test_invalid_command() {
    nccli()
        .arg("invalid-command")
        .assert()
        .failure();
}

#[test]
fn test_connection_up_nonexistent() {
    nccli()
        .arg("connection")
        .arg("up")
        .arg("nonexistent")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_connection_down_nonexistent() {
    nccli()
        .arg("connection")
        .arg("down")
        .arg("nonexistent")
        .assert()
        .success(); // Down doesn't check if connection exists
}

#[test]
fn test_device_show_loopback() {
    // Loopback should be available on all systems
    let output = nccli()
        .arg("device")
        .arg("show")
        .arg("lo")
        .output()
        .expect("Failed to execute command");

    // If failed due to permissions or system limitations, skip test
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        if stderr.contains("Permission denied")
            || stderr.contains("Operation not permitted")
            || stderr.contains("Not supported")
            || stderr.contains("/sys/class/net")
            || stderr.contains("not found") {
            eprintln!("Test skipped: requires system access - {}", stderr);
            return;
        }
    }

    // If we got output, verify it contains device information
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("GENERAL") || stdout.contains("lo"),
                "Output should contain loopback device information, got: {}", stdout);
    } else {
        // If it failed but not due to known issues, the assertion message will show
        panic!("Command failed unexpectedly: {}", stderr);
    }
}

#[test]
fn test_networking_connectivity() {
    nccli()
        .arg("networking")
        .arg("connectivity")
        .assert()
        .success()
        .stdout(predicate::str::contains("full"));
}

#[test]
fn test_default_command_is_status() {
    // Running nccli with no args should show status
    let output = nccli()
        .output()
        .expect("Failed to execute command");

    // If failed due to permissions, skip test
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Permission denied") || stderr.contains("Operation not permitted") || stderr.contains("Not supported") || stderr.contains("/sys/class/net") {
            eprintln!("Test skipped: requires elevated privileges");
            return;
        }
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("STATE") || stdout.contains("running"),
            "Default command should show status");
}

#[test]
fn test_version_from_help() {
    nccli()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("nccli"));
}

// Connection management tests with temp directory
#[test]
fn test_connection_add_ethernet() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("connections");
    fs::create_dir_all(&config_dir).unwrap();

    // Note: This test would need proper environment setup or mocking
    // to actually create connections in the test config directory
    // For now, we just verify the command structure
    nccli()
        .arg("connection")
        .arg("add")
        .arg("--type")
        .arg("ethernet")
        .arg("--con-name")
        .arg("test-eth")
        .arg("--ifname")
        .arg("eth0")
        .arg("--ip4")
        .arg("auto")
        .assert()
        .success();
}

#[test]
fn test_connection_add_missing_name() {
    nccli()
        .arg("connection")
        .arg("add")
        .arg("--type")
        .arg("ethernet")
        .arg("--ip4")
        .arg("auto")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_output_mode_tabular() {
    let output = nccli()
        .arg("-m")
        .arg("tabular")
        .arg("device")
        .arg("status")
        .output()
        .expect("Failed to execute command");

    // If failed due to permissions, skip test
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Permission denied") || stderr.contains("Operation not permitted") || stderr.contains("Not supported") || stderr.contains("/sys/class/net") {
            eprintln!("Test skipped: requires elevated privileges");
            return;
        }
    }

    assert!(output.status.success(), "Tabular mode should work");
}

#[test]
fn test_output_mode_terse() {
    let output = nccli()
        .arg("-m")
        .arg("terse")
        .arg("-t")
        .arg("device")
        .arg("status")
        .output()
        .expect("Failed to execute command");

    // If failed due to permissions, skip test
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Permission denied") || stderr.contains("Operation not permitted") || stderr.contains("Not supported") || stderr.contains("/sys/class/net") {
            eprintln!("Test skipped: requires elevated privileges");
            return;
        }
    }

    assert!(output.status.success(), "Terse mode should work");
}

#[test]
fn test_wifi_commands_help() {
    nccli()
        .arg("device")
        .arg("wifi")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("list"));
}

#[test]
fn test_connection_commands_help() {
    nccli()
        .arg("connection")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("show"))
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("modify"))
        .stdout(predicate::str::contains("delete"));
}

#[test]
fn test_general_commands_help() {
    nccli()
        .arg("general")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("hostname"))
        .stdout(predicate::str::contains("permissions"));
}

#[test]
fn test_device_commands_help() {
    nccli()
        .arg("device")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("show"))
        .stdout(predicate::str::contains("connect"))
        .stdout(predicate::str::contains("disconnect"));
}
