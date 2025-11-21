//! DHCP client management via crdhcpc
//!
//! This module provides integration with the crdhcpc DHCP client daemon.
//! It handles starting, stopping, renewing, and monitoring DHCP leases on interfaces.

use crate::error::{NetctlError, NetctlResult};
use crate::validation;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

/// Path to crdhcpc binary
const CRDHCPC_BIN: &str = "/usr/local/bin/crdhcpc";
/// Default path to crdhcpc config
const CRDHCPC_CONFIG: &str = "/etc/dhcp-client.toml";

/// DHCP client state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DhcpClientState {
    /// Not running
    Stopped,
    /// Discovering/requesting
    Acquiring,
    /// Lease acquired
    Bound,
    /// Renewing lease
    Renewing,
    /// Rebinding lease
    Rebinding,
    /// Failed to acquire lease
    Failed,
}

/// DHCP lease information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhcpLease {
    /// Interface name
    pub interface: String,
    /// Assigned IP address
    pub ip_address: Option<String>,
    /// Subnet mask
    pub subnet_mask: Option<String>,
    /// Gateway/router
    pub gateway: Option<String>,
    /// DNS servers
    pub dns_servers: Vec<String>,
    /// Lease time in seconds
    pub lease_time: Option<u32>,
    /// Time when lease was acquired
    pub acquired_at: Option<u64>,
    /// DHCP server address
    pub server_address: Option<String>,
}

/// DHCP client controller
pub struct DhcpClientController {
    /// Path to crdhcpc binary
    crdhcpc_bin: PathBuf,
    /// Path to config file
    config_path: PathBuf,
}

impl DhcpClientController {
    /// Create a new DHCP client controller
    pub fn new() -> Self {
        Self {
            crdhcpc_bin: PathBuf::from(CRDHCPC_BIN),
            config_path: PathBuf::from(CRDHCPC_CONFIG),
        }
    }

    /// Create a controller with custom paths
    pub fn with_paths(crdhcpc_bin: PathBuf, config_path: PathBuf) -> Self {
        Self {
            crdhcpc_bin,
            config_path,
        }
    }

    /// Check if crdhcpc is installed
    pub async fn is_installed(&self) -> bool {
        tokio::fs::metadata(&self.crdhcpc_bin).await.is_ok()
    }

    /// Start DHCP client on an interface
    pub async fn start(&self, interface: &str) -> NetctlResult<()> {
        validation::validate_interface_name(interface)?;

        if !self.is_installed().await {
            return Err(NetctlError::NotFound(format!(
                "crdhcpc not found at {}. Please install crdhcpc from ../crdhpcd",
                self.crdhcpc_bin.display()
            )));
        }

        info!("Starting DHCP client on interface {}", interface);

        let output = Command::new(&self.crdhcpc_bin)
            .arg("-c")
            .arg(&self.config_path)
            .arg("start")
            .arg(interface)
            .output()
            .await
            .map_err(|e| NetctlError::CommandFailed {
                cmd: format!("crdhcpc start {}", interface),
                code: None,
                stderr: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to start DHCP client on {}: {}", interface, stderr);
            return Err(NetctlError::CommandFailed {
                cmd: format!("crdhcpc start {}", interface),
                code: output.status.code(),
                stderr: stderr.to_string(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        debug!("DHCP client started on {}: {}", interface, stdout);
        Ok(())
    }

    /// Stop DHCP client on an interface
    pub async fn stop(&self, interface: &str) -> NetctlResult<()> {
        validation::validate_interface_name(interface)?;

        if !self.is_installed().await {
            // Not an error if not installed
            debug!("crdhcpc not installed, skipping stop");
            return Ok(());
        }

        info!("Stopping DHCP client on interface {}", interface);

        let output = Command::new(&self.crdhcpc_bin)
            .arg("-c")
            .arg(&self.config_path)
            .arg("stop")
            .arg(interface)
            .output()
            .await
            .map_err(|e| NetctlError::CommandFailed {
                cmd: format!("crdhcpc stop {}", interface),
                code: None,
                stderr: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to stop DHCP client on {}: {}", interface, stderr);
            // Don't return error for stop failures
        }

        Ok(())
    }

    /// Release DHCP lease on an interface
    pub async fn release(&self, interface: &str) -> NetctlResult<()> {
        validation::validate_interface_name(interface)?;

        if !self.is_installed().await {
            return Ok(());
        }

        info!("Releasing DHCP lease on interface {}", interface);

        let output = Command::new(&self.crdhcpc_bin)
            .arg("-c")
            .arg(&self.config_path)
            .arg("release")
            .arg(interface)
            .output()
            .await
            .map_err(|e| NetctlError::CommandFailed {
                cmd: format!("crdhcpc release {}", interface),
                code: None,
                stderr: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to release DHCP lease on {}: {}", interface, stderr);
        }

        Ok(())
    }

    /// Renew DHCP lease on an interface
    pub async fn renew(&self, interface: &str) -> NetctlResult<()> {
        validation::validate_interface_name(interface)?;

        if !self.is_installed().await {
            return Err(NetctlError::NotFound(
                "crdhcpc not installed".to_string()
            ));
        }

        info!("Renewing DHCP lease on interface {}", interface);

        let output = Command::new(&self.crdhcpc_bin)
            .arg("-c")
            .arg(&self.config_path)
            .arg("renew")
            .arg(interface)
            .output()
            .await
            .map_err(|e| NetctlError::CommandFailed {
                cmd: format!("crdhcpc renew {}", interface),
                code: None,
                stderr: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(NetctlError::CommandFailed {
                cmd: format!("crdhcpc renew {}", interface),
                code: output.status.code(),
                stderr: stderr.to_string(),
            });
        }

        Ok(())
    }

    /// Get DHCP status for an interface
    pub async fn status(&self, interface: &str) -> NetctlResult<Option<DhcpLease>> {
        validation::validate_interface_name(interface)?;

        if !self.is_installed().await {
            return Ok(None);
        }

        let output = Command::new(&self.crdhcpc_bin)
            .arg("-c")
            .arg(&self.config_path)
            .arg("status")
            .arg(interface)
            .output()
            .await
            .map_err(|e| NetctlError::CommandFailed {
                cmd: format!("crdhcpc status {}", interface),
                code: None,
                stderr: e.to_string(),
            })?;

        if !output.status.success() {
            // Interface may not have DHCP running
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Parse the output to extract lease information
        // For now, return a basic structure - actual parsing depends on crdhcpc output format
        Ok(Some(DhcpLease {
            interface: interface.to_string(),
            ip_address: None,
            subnet_mask: None,
            gateway: None,
            dns_servers: Vec::new(),
            lease_time: None,
            acquired_at: None,
            server_address: None,
        }))
    }

    /// Check if DHCP client is running on an interface
    pub async fn is_running(&self, interface: &str) -> bool {
        self.status(interface).await.ok().flatten().is_some()
    }

    /// Start DHCP daemon (manages all interfaces)
    pub async fn start_daemon(&self) -> NetctlResult<()> {
        if !self.is_installed().await {
            return Err(NetctlError::NotFound(
                "crdhcpc not installed".to_string()
            ));
        }

        info!("Starting DHCP client daemon");

        let mut child = Command::new(&self.crdhcpc_bin)
            .arg("-c")
            .arg(&self.config_path)
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| NetctlError::CommandFailed {
                cmd: "crdhcpc daemon".to_string(),
                code: None,
                stderr: e.to_string(),
            })?;

        // Give it a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Check if it's still running
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Err(NetctlError::CommandFailed {
                        cmd: "crdhcpc daemon".to_string(),
                        code: status.code(),
                        stderr: "Daemon exited immediately".to_string(),
                    });
                }
            }
            Ok(None) => {
                // Still running, good
                debug!("DHCP client daemon started successfully");
            }
            Err(e) => {
                return Err(NetctlError::CommandFailed {
                    cmd: "crdhcpc daemon".to_string(),
                    code: None,
                    stderr: e.to_string(),
                });
            }
        }

        Ok(())
    }
}

impl Default for DhcpClientController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dhcp_client_controller_creation() {
        let controller = DhcpClientController::new();
        assert_eq!(controller.crdhcpc_bin, PathBuf::from(CRDHCPC_BIN));
        assert_eq!(controller.config_path, PathBuf::from(CRDHCPC_CONFIG));
    }

    #[test]
    fn test_custom_paths() {
        let controller = DhcpClientController::with_paths(
            PathBuf::from("/custom/bin/crdhcpc"),
            PathBuf::from("/custom/config.toml"),
        );
        assert_eq!(controller.crdhcpc_bin, PathBuf::from("/custom/bin/crdhcpc"));
        assert_eq!(controller.config_path, PathBuf::from("/custom/config.toml"));
    }
}
