//! WPA Supplicant control
//!
//! This module provides control over wpa_supplicant for WiFi connections

use crate::error::{NetctlError, NetctlResult};
use crate::validation;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tokio::fs;
use tracing::{debug, info, warn};

/// WPA Supplicant controller
pub struct WpaSupplicantController {
    /// Path to wpa_supplicant binary
    wpa_bin: PathBuf,
    /// Path to wpa_cli binary
    wpa_cli_bin: PathBuf,
    /// Configuration directory
    config_dir: PathBuf,
}

impl WpaSupplicantController {
    /// Create a new WPA Supplicant controller
    pub fn new() -> Self {
        Self {
            wpa_bin: PathBuf::from("/usr/sbin/wpa_supplicant"),
            wpa_cli_bin: PathBuf::from("/usr/bin/wpa_cli"),
            config_dir: PathBuf::from("/etc/wpa_supplicant"),
        }
    }

    /// Check if wpa_supplicant is installed
    pub async fn is_installed(&self) -> bool {
        tokio::fs::metadata(&self.wpa_bin).await.is_ok()
    }

    /// Connect to a WiFi network
    pub async fn connect(
        &self,
        interface: &str,
        ssid: &str,
        psk: Option<&str>,
    ) -> NetctlResult<()> {
        validation::validate_interface_name(interface)?;

        if !self.is_installed().await {
            return Err(NetctlError::NotFound(
                "wpa_supplicant not installed".to_string()
            ));
        }

        info!("Connecting to WiFi network '{}' on {}", ssid, interface);

        // Generate wpa_supplicant config
        let config = self.generate_config(ssid, psk)?;
        let config_path = self.config_dir.join(format!("{}.conf", interface));

        // Write config
        fs::write(&config_path, config).await?;

        // Start wpa_supplicant
        let output = Command::new(&self.wpa_bin)
            .arg("-B") // Background
            .arg("-i")
            .arg(interface)
            .arg("-c")
            .arg(&config_path)
            .output()
            .await
            .map_err(|e| NetctlError::CommandFailed {
                cmd: format!("wpa_supplicant -i {}", interface),
                code: None,
                stderr: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(NetctlError::CommandFailed {
                cmd: format!("wpa_supplicant -i {}", interface),
                code: output.status.code(),
                stderr: stderr.to_string(),
            });
        }

        debug!("wpa_supplicant started on {}", interface);

        // Wait a moment for connection to establish
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        Ok(())
    }

    /// Disconnect from WiFi network
    pub async fn disconnect(&self, interface: &str) -> NetctlResult<()> {
        validation::validate_interface_name(interface)?;

        if !self.is_installed().await {
            return Ok(());
        }

        info!("Disconnecting WiFi on {}", interface);

        // Kill wpa_supplicant for this interface
        let output = Command::new("pkill")
            .arg("-f")
            .arg(&format!("wpa_supplicant.*{}", interface))
            .output()
            .await
            .map_err(|e| NetctlError::CommandFailed {
                cmd: format!("pkill wpa_supplicant {}", interface),
                code: None,
                stderr: e.to_string(),
            })?;

        // Don't error on failure - process might not be running
        if !output.status.success() {
            debug!("wpa_supplicant may not have been running on {}", interface);
        }

        Ok(())
    }

    /// Generate wpa_supplicant configuration
    fn generate_config(&self, ssid: &str, psk: Option<&str>) -> NetctlResult<String> {
        let mut config = String::new();
        config.push_str("ctrl_interface=/var/run/wpa_supplicant\n");
        config.push_str("update_config=1\n\n");
        config.push_str("network={\n");
        config.push_str(&format!("    ssid=\"{}\"\n", ssid));

        if let Some(password) = psk {
            config.push_str(&format!("    psk=\"{}\"\n", password));
            config.push_str("    key_mgmt=WPA-PSK\n");
        } else {
            config.push_str("    key_mgmt=NONE\n");
        }

        config.push_str("}\n");

        Ok(config)
    }

    /// Check if wpa_supplicant is running on an interface
    pub async fn is_running(&self, interface: &str) -> bool {
        if validation::validate_interface_name(interface).is_err() {
            return false;
        }

        let output = Command::new("pgrep")
            .arg("-f")
            .arg(&format!("wpa_supplicant.*{}", interface))
            .output()
            .await;

        match output {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }
}

impl Default for WpaSupplicantController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_config_with_psk() {
        let controller = WpaSupplicantController::new();
        let config = controller.generate_config("TestSSID", Some("password123")).unwrap();
        assert!(config.contains("ssid=\"TestSSID\""));
        assert!(config.contains("psk=\"password123\""));
        assert!(config.contains("key_mgmt=WPA-PSK"));
    }

    #[test]
    fn test_generate_config_open() {
        let controller = WpaSupplicantController::new();
        let config = controller.generate_config("OpenNetwork", None).unwrap();
        assert!(config.contains("ssid=\"OpenNetwork\""));
        assert!(config.contains("key_mgmt=NONE"));
        assert!(!config.contains("psk="));
    }
}
