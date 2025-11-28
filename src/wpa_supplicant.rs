//! WPA Supplicant control
//!
//! This module provides control over wpa_supplicant for WiFi connections.
//! It uses wpa_cli for runtime control when wpa_supplicant is already running,
//! and can start/stop wpa_supplicant as needed.

use crate::error::{NetctlError, NetctlResult};
use crate::validation;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Control interface directory for wpa_supplicant
const CTRL_INTERFACE: &str = "/var/run/wpa_supplicant";

/// WPA Supplicant connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WpaState {
    Disconnected,
    Scanning,
    Associating,
    Associated,
    FourWayHandshake,
    GroupHandshake,
    Completed,
    Unknown,
}

impl From<&str> for WpaState {
    fn from(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "DISCONNECTED" => WpaState::Disconnected,
            "SCANNING" => WpaState::Scanning,
            "ASSOCIATING" => WpaState::Associating,
            "ASSOCIATED" => WpaState::Associated,
            "4WAY_HANDSHAKE" => WpaState::FourWayHandshake,
            "GROUP_HANDSHAKE" => WpaState::GroupHandshake,
            "COMPLETED" => WpaState::Completed,
            _ => WpaState::Unknown,
        }
    }
}

/// WiFi security type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WpaSecurityType {
    None,
    Wep,
    WpaPsk,
    Wpa2Psk,
    Wpa3Sae,
    WpaEap,
    Wpa2Eap,
}

/// Connection status from wpa_supplicant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WpaStatus {
    pub state: WpaState,
    pub ssid: Option<String>,
    pub bssid: Option<String>,
    pub frequency: Option<u32>,
    pub ip_address: Option<String>,
    pub key_mgmt: Option<String>,
    pub pairwise_cipher: Option<String>,
    pub group_cipher: Option<String>,
}

/// Scan result from wpa_supplicant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WpaScanResult {
    pub bssid: String,
    pub frequency: u32,
    pub signal_level: i32,
    pub flags: String,
    pub ssid: String,
}

impl WpaScanResult {
    /// Get security type from flags
    pub fn security_type(&self) -> WpaSecurityType {
        if self.flags.contains("WPA3") || self.flags.contains("SAE") {
            WpaSecurityType::Wpa3Sae
        } else if self.flags.contains("WPA2-EAP") {
            WpaSecurityType::Wpa2Eap
        } else if self.flags.contains("WPA-EAP") {
            WpaSecurityType::WpaEap
        } else if self.flags.contains("WPA2-PSK") {
            WpaSecurityType::Wpa2Psk
        } else if self.flags.contains("WPA-PSK") {
            WpaSecurityType::WpaPsk
        } else if self.flags.contains("WEP") {
            WpaSecurityType::Wep
        } else {
            WpaSecurityType::None
        }
    }

    /// Convert signal level (dBm) to percentage (0-100)
    pub fn signal_percent(&self) -> u8 {
        // Typical range is -90 dBm (weak) to -30 dBm (strong)
        let clamped = self.signal_level.clamp(-90, -30);
        ((clamped + 90) * 100 / 60) as u8
    }
}

/// WPA Supplicant controller
pub struct WpaSupplicantController {
    /// Path to wpa_supplicant binary
    wpa_bin: PathBuf,
    /// Path to wpa_cli binary
    wpa_cli_bin: PathBuf,
    /// Configuration directory
    config_dir: PathBuf,
    /// Control interface directory
    ctrl_interface: PathBuf,
}

impl WpaSupplicantController {
    /// Create a new WPA Supplicant controller
    pub fn new() -> Self {
        Self {
            wpa_bin: PathBuf::from("/usr/sbin/wpa_supplicant"),
            wpa_cli_bin: PathBuf::from("/usr/sbin/wpa_cli"),
            config_dir: PathBuf::from("/etc/wpa_supplicant"),
            ctrl_interface: PathBuf::from(CTRL_INTERFACE),
        }
    }

    /// Check if wpa_supplicant is installed
    pub async fn is_installed(&self) -> bool {
        tokio::fs::metadata(&self.wpa_bin).await.is_ok()
    }

    /// Check if wpa_cli is installed
    pub async fn is_cli_installed(&self) -> bool {
        tokio::fs::metadata(&self.wpa_cli_bin).await.is_ok()
    }

    /// Check if wpa_supplicant is running on an interface
    pub async fn is_running(&self, interface: &str) -> bool {
        if validation::validate_interface_name(interface).is_err() {
            return false;
        }

        // Check if control socket exists
        let socket_path = self.ctrl_interface.join(interface);
        if tokio::fs::metadata(&socket_path).await.is_ok() {
            // Verify it's responsive
            match self.wpa_cli(interface, &["status"]).await {
                Ok(_) => true,
                Err(_) => false,
            }
        } else {
            // Fallback to pgrep
            let output = Command::new("pgrep")
                .arg("-f")
                .arg(&format!("wpa_supplicant.*-i\\s*{}", interface))
                .output()
                .await;

            match output {
                Ok(output) => output.status.success(),
                Err(_) => false,
            }
        }
    }

    /// Start wpa_supplicant on an interface
    pub async fn start(&self, interface: &str) -> NetctlResult<()> {
        validation::validate_interface_name(interface)?;

        if !self.is_installed().await {
            return Err(NetctlError::NotFound(
                "wpa_supplicant not installed".to_string(),
            ));
        }

        if self.is_running(interface).await {
            debug!("wpa_supplicant already running on {}", interface);
            return Ok(());
        }

        info!("Starting wpa_supplicant on {}", interface);

        // Ensure config directory exists
        fs::create_dir_all(&self.config_dir).await?;

        // Create minimal config if it doesn't exist
        let config_path = self.config_dir.join(format!("{}.conf", interface));
        if !config_path.exists() {
            let config = self.generate_base_config();
            fs::write(&config_path, config).await?;
        }

        // Ensure control interface directory exists
        fs::create_dir_all(&self.ctrl_interface).await?;

        // Start wpa_supplicant
        let output = Command::new(&self.wpa_bin)
            .arg("-B") // Background/daemon mode
            .arg("-D")
            .arg("nl80211,wext") // Try nl80211 first, fallback to wext
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

        // Wait for control socket to be ready
        for _ in 0..10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            if self.is_running(interface).await {
                info!("wpa_supplicant started on {}", interface);
                return Ok(());
            }
        }

        Err(NetctlError::ServiceError(format!(
            "wpa_supplicant started but control socket not ready on {}",
            interface
        )))
    }

    /// Stop wpa_supplicant on an interface
    pub async fn stop(&self, interface: &str) -> NetctlResult<()> {
        validation::validate_interface_name(interface)?;

        if !self.is_running(interface).await {
            debug!("wpa_supplicant not running on {}", interface);
            return Ok(());
        }

        info!("Stopping wpa_supplicant on {}", interface);

        // Try graceful termination via wpa_cli first
        let _ = self.wpa_cli(interface, &["terminate"]).await;

        // Wait a moment
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // If still running, force kill
        if self.is_running(interface).await {
            warn!("wpa_supplicant didn't terminate gracefully, forcing kill");
            let _ = Command::new("pkill")
                .arg("-f")
                .arg(&format!("wpa_supplicant.*-i\\s*{}", interface))
                .output()
                .await;
        }

        Ok(())
    }

    /// Get connection status
    pub async fn status(&self, interface: &str) -> NetctlResult<WpaStatus> {
        validation::validate_interface_name(interface)?;

        let output = self.wpa_cli(interface, &["status"]).await?;

        let mut status = WpaStatus {
            state: WpaState::Unknown,
            ssid: None,
            bssid: None,
            frequency: None,
            ip_address: None,
            key_mgmt: None,
            pairwise_cipher: None,
            group_cipher: None,
        };

        for line in output.lines() {
            if let Some((key, value)) = line.split_once('=') {
                match key {
                    "wpa_state" => status.state = WpaState::from(value),
                    "ssid" => status.ssid = Some(value.to_string()),
                    "bssid" => status.bssid = Some(value.to_string()),
                    "freq" => status.frequency = value.parse().ok(),
                    "ip_address" => status.ip_address = Some(value.to_string()),
                    "key_mgmt" => status.key_mgmt = Some(value.to_string()),
                    "pairwise_cipher" => status.pairwise_cipher = Some(value.to_string()),
                    "group_cipher" => status.group_cipher = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        Ok(status)
    }

    /// Trigger a network scan
    pub async fn scan(&self, interface: &str) -> NetctlResult<()> {
        validation::validate_interface_name(interface)?;

        // Ensure wpa_supplicant is running
        if !self.is_running(interface).await {
            self.start(interface).await?;
        }

        self.wpa_cli(interface, &["scan"]).await?;
        Ok(())
    }

    /// Get scan results
    pub async fn scan_results(&self, interface: &str) -> NetctlResult<Vec<WpaScanResult>> {
        validation::validate_interface_name(interface)?;

        let output = self.wpa_cli(interface, &["scan_results"]).await?;

        let mut results = Vec::new();

        // Skip header line
        for line in output.lines().skip(1) {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 5 {
                results.push(WpaScanResult {
                    bssid: parts[0].to_string(),
                    frequency: parts[1].parse().unwrap_or(0),
                    signal_level: parts[2].parse().unwrap_or(-100),
                    flags: parts[3].to_string(),
                    ssid: parts[4..].join("\t"), // SSID might contain tabs
                });
            }
        }

        Ok(results)
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
                "wpa_supplicant not installed".to_string(),
            ));
        }

        info!("Connecting to WiFi network '{}' on {}", ssid, interface);

        // Ensure wpa_supplicant is running
        if !self.is_running(interface).await {
            self.start(interface).await?;
        }

        // Add network
        let output = self.wpa_cli(interface, &["add_network"]).await?;
        let network_id = output.trim().to_string();

        debug!("Added network with id: {}", network_id);

        // Set SSID
        self.wpa_cli(
            interface,
            &["set_network", &network_id, "ssid", &format!("\"{}\"", ssid)],
        )
        .await?;

        // Set PSK or open network
        if let Some(password) = psk {
            self.wpa_cli(
                interface,
                &[
                    "set_network",
                    &network_id,
                    "psk",
                    &format!("\"{}\"", password),
                ],
            )
            .await?;
        } else {
            self.wpa_cli(
                interface,
                &["set_network", &network_id, "key_mgmt", "NONE"],
            )
            .await?;
        }

        // Enable network
        self.wpa_cli(interface, &["enable_network", &network_id])
            .await?;

        // Select network (disconnect from any current and connect to this one)
        self.wpa_cli(interface, &["select_network", &network_id])
            .await?;

        // Save configuration
        let _ = self.wpa_cli(interface, &["save_config"]).await;

        // Wait for connection with timeout
        let timeout = tokio::time::Duration::from_secs(30);
        let start = tokio::time::Instant::now();

        while start.elapsed() < timeout {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            match self.status(interface).await {
                Ok(status) => {
                    debug!("Connection state: {:?}", status.state);
                    match status.state {
                        WpaState::Completed => {
                            info!("Successfully connected to '{}'", ssid);
                            return Ok(());
                        }
                        WpaState::Disconnected => {
                            // Check if we were trying to connect
                            if start.elapsed() > tokio::time::Duration::from_secs(5) {
                                // Might be auth failure
                                break;
                            }
                        }
                        _ => {
                            // Still trying to connect
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to get status: {}", e);
                }
            }
        }

        // Connection failed, clean up
        let _ = self
            .wpa_cli(interface, &["remove_network", &network_id])
            .await;

        Err(NetctlError::ConnectionFailed {
            reason: format!(
                "Failed to connect to '{}' within timeout. Check password and signal.",
                ssid
            ),
        })
    }

    /// Disconnect from current network
    pub async fn disconnect(&self, interface: &str) -> NetctlResult<()> {
        validation::validate_interface_name(interface)?;

        if !self.is_running(interface).await {
            debug!("wpa_supplicant not running on {}", interface);
            return Ok(());
        }

        info!("Disconnecting WiFi on {}", interface);

        self.wpa_cli(interface, &["disconnect"]).await?;

        Ok(())
    }

    /// List configured networks
    pub async fn list_networks(&self, interface: &str) -> NetctlResult<Vec<(String, String, String)>> {
        validation::validate_interface_name(interface)?;

        let output = self.wpa_cli(interface, &["list_networks"]).await?;

        let mut networks = Vec::new();

        // Skip header line
        for line in output.lines().skip(1) {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                networks.push((
                    parts[0].to_string(), // network id
                    parts[1].to_string(), // ssid
                    parts[3].to_string(), // flags (like [CURRENT])
                ));
            }
        }

        Ok(networks)
    }

    /// Remove a configured network
    pub async fn remove_network(&self, interface: &str, network_id: &str) -> NetctlResult<()> {
        validation::validate_interface_name(interface)?;

        self.wpa_cli(interface, &["remove_network", network_id])
            .await?;
        let _ = self.wpa_cli(interface, &["save_config"]).await;

        Ok(())
    }

    /// Reconnect to the current network
    pub async fn reconnect(&self, interface: &str) -> NetctlResult<()> {
        validation::validate_interface_name(interface)?;

        self.wpa_cli(interface, &["reconnect"]).await?;
        Ok(())
    }

    /// Reassociate with current AP
    pub async fn reassociate(&self, interface: &str) -> NetctlResult<()> {
        validation::validate_interface_name(interface)?;

        self.wpa_cli(interface, &["reassociate"]).await?;
        Ok(())
    }

    /// Get signal strength (poll-based)
    pub async fn signal_poll(&self, interface: &str) -> NetctlResult<i32> {
        validation::validate_interface_name(interface)?;

        let output = self.wpa_cli(interface, &["signal_poll"]).await?;

        for line in output.lines() {
            if let Some(rssi) = line.strip_prefix("RSSI=") {
                return rssi
                    .parse()
                    .map_err(|_| NetctlError::ParseError("Failed to parse RSSI".to_string()));
            }
        }

        Err(NetctlError::NotFound("RSSI not available".to_string()))
    }

    // === Helper functions ===

    /// Run wpa_cli command
    async fn wpa_cli(&self, interface: &str, args: &[&str]) -> NetctlResult<String> {
        let mut cmd = Command::new(&self.wpa_cli_bin);
        cmd.arg("-i").arg(interface);
        cmd.args(args);

        let cmd_str = format!("wpa_cli -i {} {}", interface, args.join(" "));
        debug!("Running: {}", cmd_str);

        let output = cmd.output().await.map_err(|e| NetctlError::CommandFailed {
            cmd: cmd_str.clone(),
            code: None,
            stderr: e.to_string(),
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            return Err(NetctlError::CommandFailed {
                cmd: cmd_str,
                code: output.status.code(),
                stderr,
            });
        }

        // wpa_cli returns "FAIL" on errors even with exit code 0
        if stdout.trim() == "FAIL" {
            return Err(NetctlError::CommandFailed {
                cmd: cmd_str,
                code: None,
                stderr: "wpa_cli returned FAIL".to_string(),
            });
        }

        Ok(stdout)
    }

    /// Generate base wpa_supplicant configuration
    fn generate_base_config(&self) -> String {
        format!(
            "ctrl_interface={}\n\
             update_config=1\n\
             country=US\n",
            CTRL_INTERFACE
        )
    }

    /// Generate network block for config file
    pub fn generate_network_config(
        &self,
        ssid: &str,
        psk: Option<&str>,
        hidden: bool,
    ) -> String {
        let mut config = String::new();
        config.push_str("network={\n");
        config.push_str(&format!("    ssid=\"{}\"\n", ssid));

        if hidden {
            config.push_str("    scan_ssid=1\n");
        }

        if let Some(password) = psk {
            config.push_str(&format!("    psk=\"{}\"\n", password));
            config.push_str("    key_mgmt=WPA-PSK\n");
        } else {
            config.push_str("    key_mgmt=NONE\n");
        }

        config.push_str("}\n");
        config
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
    fn test_wpa_state_from_str() {
        assert_eq!(WpaState::from("COMPLETED"), WpaState::Completed);
        assert_eq!(WpaState::from("DISCONNECTED"), WpaState::Disconnected);
        assert_eq!(WpaState::from("4WAY_HANDSHAKE"), WpaState::FourWayHandshake);
        assert_eq!(WpaState::from("unknown"), WpaState::Unknown);
    }

    #[test]
    fn test_scan_result_security() {
        let result = WpaScanResult {
            bssid: "00:11:22:33:44:55".to_string(),
            frequency: 2437,
            signal_level: -50,
            flags: "[WPA2-PSK-CCMP][ESS]".to_string(),
            ssid: "TestNetwork".to_string(),
        };
        assert_eq!(result.security_type(), WpaSecurityType::Wpa2Psk);
    }

    #[test]
    fn test_scan_result_signal_percent() {
        let mut result = WpaScanResult {
            bssid: "00:11:22:33:44:55".to_string(),
            frequency: 2437,
            signal_level: -30,
            flags: "".to_string(),
            ssid: "Test".to_string(),
        };
        assert_eq!(result.signal_percent(), 100);

        result.signal_level = -90;
        assert_eq!(result.signal_percent(), 0);

        result.signal_level = -60;
        assert_eq!(result.signal_percent(), 50);
    }

    #[test]
    fn test_generate_network_config() {
        let controller = WpaSupplicantController::new();

        let config = controller.generate_network_config("TestSSID", Some("password123"), false);
        assert!(config.contains("ssid=\"TestSSID\""));
        assert!(config.contains("psk=\"password123\""));
        assert!(config.contains("key_mgmt=WPA-PSK"));
        assert!(!config.contains("scan_ssid=1"));

        let config_hidden =
            controller.generate_network_config("HiddenNetwork", Some("secret"), true);
        assert!(config_hidden.contains("scan_ssid=1"));

        let config_open = controller.generate_network_config("OpenNetwork", None, false);
        assert!(config_open.contains("key_mgmt=NONE"));
        assert!(!config_open.contains("psk="));
    }
}
