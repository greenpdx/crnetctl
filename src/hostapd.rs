//! hostapd management for WiFi Access Point
//!
//! Configuration generation and control for hostapd

use crate::error::{NetctlError, NetctlResult};
use serde::{Deserialize, Serialize};
use tokio::fs;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPointConfig {
    /// Interface name
    pub interface: String,
    /// SSID
    pub ssid: String,
    /// Password (WPA2/WPA3), None for open network
    pub password: Option<String>,
    /// Channel number
    pub channel: u8,
    /// Band: "2.4GHz" or "5GHz"
    pub band: String,
    /// Country code (regulatory domain)
    pub country_code: String,
    /// Hidden SSID
    pub hidden: bool,
    /// Maximum number of clients
    pub max_clients: Option<u32>,
    /// Client isolation (prevent client-to-client communication)
    pub ap_isolate: bool,
    /// Enable WMM/QoS
    pub wmm_enabled: bool,
    /// IEEE 802.11n (HT) support
    pub ieee80211n: bool,
    /// IEEE 802.11ac (VHT) support
    pub ieee80211ac: bool,
    /// IEEE 802.11ax (HE/WiFi 6) support
    pub ieee80211ax: bool,
    /// Channel width: 20, 40, 80, 160 MHz
    pub channel_width: u8,
    /// TX power limit (dBm)
    pub tx_power: Option<u8>,
}

impl Default for AccessPointConfig {
    fn default() -> Self {
        Self {
            interface: "wlan0".to_string(),
            ssid: "CRRouter-AP".to_string(),
            password: Some("crrouter123".to_string()),
            channel: 6,
            band: "2.4GHz".to_string(),
            country_code: "US".to_string(),
            hidden: false,
            max_clients: Some(32),
            ap_isolate: false,
            wmm_enabled: true,
            ieee80211n: true,
            ieee80211ac: false,
            ieee80211ax: false,
            channel_width: 20,
            tx_power: None,
        }
    }
}

/// hostapd controller
pub struct HostapdController {
    config_dir: PathBuf,
    pid_file: PathBuf,
}

impl HostapdController {
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            pid_file: config_dir.join("hostapd.pid"),
            config_dir,
        }
    }

    /// Generate hostapd configuration file
    pub fn generate_config(&self, config: &AccessPointConfig) -> NetctlResult<String> {
        let mut conf = String::new();

        conf.push_str(&format!("interface={}\n", config.interface));
        conf.push_str("driver=nl80211\n");
        conf.push_str(&format!("ssid={}\n", config.ssid));
        conf.push_str(&format!("country_code={}\n", config.country_code));

        let hw_mode = if config.band == "5GHz" { "a" } else { "g" };
        conf.push_str(&format!("hw_mode={}\n", hw_mode));
        conf.push_str(&format!("channel={}\n", config.channel));

        if config.hidden {
            conf.push_str("ignore_broadcast_ssid=1\n");
        }

        if let Some(ref password) = config.password {
            if password.len() < 8 {
                return Err(NetctlError::InvalidParameter(
                    "Password must be at least 8 characters".to_string()
                ));
            }
            conf.push_str("wpa=2\nwpa_passphrase=");
            conf.push_str(password);
            conf.push_str("\nwpa_key_mgmt=WPA-PSK\nwpa_pairwise=CCMP\nrsn_pairwise=CCMP\n");
        }

        if config.wmm_enabled {
            conf.push_str("wmm_enabled=1\n");
        }

        if config.ieee80211n {
            conf.push_str("ieee80211n=1\n");
            if config.channel_width >= 40 {
                conf.push_str("ht_capab=[HT40+][SHORT-GI-40]\n");
            }
        }

        if config.ieee80211ac && hw_mode == "a" {
            conf.push_str("ieee80211ac=1\n");
        }

        if config.ieee80211ax {
            conf.push_str("ieee80211ax=1\n");
        }

        if config.ap_isolate {
            conf.push_str("ap_isolate=1\n");
        }

        if let Some(max) = config.max_clients {
            conf.push_str(&format!("max_num_sta={}\n", max));
        }

        conf.push_str("auth_algs=1\nmacaddr_acl=0\n");

        Ok(conf)
    }

    pub async fn write_config(&self, config: &AccessPointConfig) -> NetctlResult<PathBuf> {
        let conf_content = self.generate_config(config)?;
        let conf_path = self.config_dir.join("hostapd.conf");
        fs::create_dir_all(&self.config_dir).await?;
        fs::write(&conf_path, conf_content).await?;
        Ok(conf_path)
    }

    pub async fn start(&self, config: &AccessPointConfig) -> NetctlResult<()> {
        if self.is_running().await? {
            return Err(NetctlError::AlreadyExists("hostapd already running".to_string()));
        }

        let conf_path = self.write_config(config).await?;

        let output = Command::new("/usr/sbin/hostapd")
            .arg("-B")
            .arg("-P").arg(&self.pid_file)
            .arg(&conf_path)
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to start hostapd: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(NetctlError::ServiceError(
                format!("hostapd failed\nStderr: {}\nStdout: {}", stderr, stdout)
            ));
        }

        sleep(Duration::from_secs(2)).await;

        if !self.is_running().await? {
            return Err(NetctlError::ServiceError(
                "hostapd process did not start successfully".to_string()
            ));
        }

        Ok(())
    }

    pub async fn stop(&self) -> NetctlResult<()> {
        if !self.is_running().await? {
            return Ok(());
        }

        let pid_str = fs::read_to_string(&self.pid_file).await?;
        let pid: i32 = pid_str.trim().parse()
            .map_err(|_| NetctlError::ServiceError("Invalid PID".to_string()))?;

        Command::new("kill").arg("-TERM").arg(pid.to_string()).output().await?;

        for _ in 0..10 {
            sleep(Duration::from_millis(500)).await;
            if !self.is_running().await? {
                let _ = fs::remove_file(&self.pid_file).await;
                return Ok(());
            }
        }

        Err(NetctlError::Timeout("hostapd did not stop".to_string()))
    }

    pub async fn is_running(&self) -> NetctlResult<bool> {
        if !self.pid_file.exists() {
            return Ok(false);
        }
        let pid_str = fs::read_to_string(&self.pid_file).await.ok();
        if let Some(pid) = pid_str {
            if let Ok(p) = pid.trim().parse::<i32>() {
                return Ok(Path::new(&format!("/proc/{}", p)).exists());
            }
        }
        Ok(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostapdStatus {
    pub running: bool,
    pub pid: Option<i32>,
}
