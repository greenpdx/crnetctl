//! WiFi device control
//!
//! Low-level WiFi management using iw command

use crate::error::{NetctlError, NetctlResult};
use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiDeviceInfo {
    pub interface: String,
    pub phy: Option<String>,
    pub type_: Option<String>,
    pub wiphy: Option<u32>,
    pub channel: Option<u32>,
    pub frequency: Option<u32>,
    pub txpower: Option<String>,
    pub ssid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiCapabilities {
    pub supported_bands: Vec<String>,
    pub supported_freqs: Vec<u32>,
    pub supported_ciphers: Vec<String>,
    pub ht_supported: bool,
    pub vht_supported: bool,
    pub he_supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegDomain {
    pub country: Option<String>,
    pub dfs_region: Option<String>,
}

/// WiFi controller
pub struct WifiController {
}

impl WifiController {
    pub fn new() -> Self {
        Self {}
    }

    /// Get WiFi device information
    pub async fn get_dev_info(&self, interface: &str) -> NetctlResult<WifiDeviceInfo> {
        let output = self.run_iw(&["dev", interface, "info"]).await?;

        let mut info = WifiDeviceInfo {
            interface: interface.to_string(),
            phy: None,
            type_: None,
            wiphy: None,
            channel: None,
            frequency: None,
            txpower: None,
            ssid: None,
        };

        for line in output.lines() {
            let line = line.trim();
            if line.starts_with("wiphy ") {
                info.wiphy = line.strip_prefix("wiphy ").and_then(|s| s.parse().ok());
            } else if line.starts_with("type ") {
                info.type_ = line.strip_prefix("type ").map(|s| s.to_string());
            } else if line.starts_with("channel ") {
                // Parse "channel 6 (2437 MHz), width: 20 MHz"
                if let Some(parts) = line.strip_prefix("channel ") {
                    if let Some(ch_str) = parts.split_whitespace().next() {
                        info.channel = ch_str.parse().ok();
                    }
                    if let Some(start) = parts.find('(') {
                        if let Some(end) = parts.find(" MHz") {
                            if let Some(freq_str) = parts.get(start + 1..end) {
                                info.frequency = freq_str.parse().ok();
                            }
                        }
                    }
                }
            } else if line.starts_with("txpower ") {
                info.txpower = line.strip_prefix("txpower ").map(|s| s.to_string());
            } else if line.starts_with("ssid ") {
                info.ssid = line.strip_prefix("ssid ").map(|s| s.to_string());
            }
        }

        Ok(info)
    }

    /// Get physical device name (phy)
    pub async fn get_phy(&self, interface: &str) -> NetctlResult<String> {
        let info = self.get_dev_info(interface).await?;
        info.phy.ok_or_else(|| NetctlError::NotSupported("Cannot determine phy".to_string()))
    }

    /// Get regulatory domain
    pub async fn get_reg_domain(&self) -> NetctlResult<RegDomain> {
        let output = self.run_iw(&["reg", "get"]).await?;

        let mut reg = RegDomain {
            country: None,
            dfs_region: None,
        };

        for line in output.lines() {
            if line.starts_with("country ") {
                if let Some(parts) = line.strip_prefix("country ") {
                    if let Some(country) = parts.split(':').next() {
                        reg.country = Some(country.trim().to_string());
                    }
                    if line.contains("DFS-") {
                        if let Some(dfs) = line.split("DFS-").nth(1) {
                            if let Some(region) = dfs.split_whitespace().next() {
                                reg.dfs_region = Some(format!("DFS-{}", region));
                            }
                        }
                    }
                }
            }
        }

        Ok(reg)
    }

    /// Set regulatory domain
    pub async fn set_reg_domain(&self, country: &str) -> NetctlResult<()> {
        if country.len() != 2 {
            return Err(NetctlError::InvalidParameter(
                "Country code must be 2 characters".to_string()
            ));
        }

        self.run_iw_no_output(&["reg", "set", country]).await
    }

    /// Get transmit power
    pub async fn get_txpower(&self, interface: &str) -> NetctlResult<String> {
        let info = self.get_dev_info(interface).await?;
        info.txpower.ok_or_else(|| NetctlError::NotSupported("TX power not available".to_string()))
    }

    /// Set transmit power (in dBm or mW)
    pub async fn set_txpower(&self, interface: &str, power: &str) -> NetctlResult<()> {
        // power can be like "20dBm" or "fixed 100mW" or "auto"
        self.run_iw_no_output(&["dev", interface, "set", "txpower", power]).await
    }

    /// Set power save mode
    pub async fn set_power_save(&self, interface: &str, enable: bool) -> NetctlResult<()> {
        let mode = if enable { "on" } else { "off" };
        self.run_iw_no_output(&["dev", interface, "set", "power_save", mode]).await
    }

    /// Get power save status
    pub async fn get_power_save(&self, interface: &str) -> NetctlResult<bool> {
        let output = self.run_iw(&["dev", interface, "get", "power_save"]).await?;
        Ok(output.contains("Power save: on"))
    }

    /// Scan for WiFi networks
    pub async fn scan(&self, interface: &str) -> NetctlResult<Vec<ScanResult>> {
        // Trigger scan
        let _ = self.run_iw(&["dev", interface, "scan"]).await;

        // Get results
        let output = self.run_iw(&["dev", interface, "scan", "dump"]).await?;

        let mut results = Vec::new();
        let mut current: Option<ScanResult> = None;

        for line in output.lines() {
            let line = line.trim();

            if line.starts_with("BSS ") {
                if let Some(result) = current.take() {
                    results.push(result);
                }
                current = Some(ScanResult {
                    bssid: line.strip_prefix("BSS ").unwrap_or("").split('(').next().unwrap_or("").trim().to_string(),
                    ssid: None,
                    frequency: None,
                    signal: None,
                    capabilities: Vec::new(),
                });
            } else if let Some(ref mut result) = current {
                if line.starts_with("SSID: ") {
                    result.ssid = Some(line.strip_prefix("SSID: ").unwrap_or("").to_string());
                } else if line.starts_with("freq: ") {
                    result.frequency = line.strip_prefix("freq: ").and_then(|s| s.parse().ok());
                } else if line.starts_with("signal: ") {
                    result.signal = Some(line.strip_prefix("signal: ").unwrap_or("").to_string());
                } else if line.contains("capability:") {
                    if let Some(caps) = line.strip_prefix("capability: ") {
                        result.capabilities = caps.split_whitespace().map(|s| s.to_string()).collect();
                    }
                }
            }
        }

        if let Some(result) = current {
            results.push(result);
        }

        Ok(results)
    }

    // === Helper functions ===

    async fn run_iw(&self, args: &[&str]) -> NetctlResult<String> {
        let output = Command::new("iw")
            .args(args)
            .output()
            .await
            .map_err(|e| NetctlError::CommandFailed {
                cmd: format!("iw {}", args.join(" ")),
                code: None,
                stderr: e.to_string(),
            })?;

        if !output.status.success() {
            return Err(NetctlError::CommandFailed {
                cmd: format!("iw {}", args.join(" ")),
                code: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn run_iw_no_output(&self, args: &[&str]) -> NetctlResult<()> {
        let output = Command::new("iw")
            .args(args)
            .output()
            .await
            .map_err(|e| NetctlError::CommandFailed {
                cmd: format!("iw {}", args.join(" ")),
                code: None,
                stderr: e.to_string(),
            })?;

        if !output.status.success() {
            return Err(NetctlError::CommandFailed {
                cmd: format!("iw {}", args.join(" ")),
                code: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(())
    }
}

impl Default for WifiController {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub bssid: String,
    pub ssid: Option<String>,
    pub frequency: Option<u32>,
    pub signal: Option<String>,
    pub capabilities: Vec<String>,
}
