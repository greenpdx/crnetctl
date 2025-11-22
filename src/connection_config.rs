//! Connection configuration file reading and management

use crate::error::{NetctlError, NetctlResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tracing::info;

/// Netctl connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetctlConnectionConfig {
    pub connection: ConnectionSection,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wifi: Option<WifiSection>,
    #[serde(rename = "wifi-security", skip_serializing_if = "Option::is_none")]
    pub wifi_security: Option<WifiSecuritySection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vpn: Option<VpnSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ethernet: Option<EthernetSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv4: Option<IpConfigSection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipv6: Option<IpConfigSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSection {
    pub name: String,
    pub uuid: String,
    #[serde(rename = "type")]
    pub conn_type: String,
    #[serde(default)]
    pub autoconnect: bool,
    #[serde(rename = "interface-name", skip_serializing_if = "Option::is_none")]
    pub interface_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiSection {
    pub ssid: String,
    #[serde(default = "default_wifi_mode")]
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bssid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<u32>,
}

fn default_wifi_mode() -> String {
    "infrastructure".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiSecuritySection {
    #[serde(rename = "key-mgmt")]
    pub key_mgmt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub psk: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnSection {
    #[serde(rename = "connection-type")]
    pub connection_type: String,  // "wireguard", "openvpn", "ipsec"

    // WireGuard configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wireguard: Option<WireGuardVpnSection>,

    // OpenVPN configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openvpn: Option<OpenVpnSection>,

    // Legacy fields (for backward compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proto: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(rename = "config_file", skip_serializing_if = "Option::is_none")]
    pub config_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireGuardVpnSection {
    pub private_key: String,
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listen_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtu: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer: Option<WireGuardPeer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peers: Option<Vec<WireGuardPeer>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireGuardPeer {
    pub public_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_ips: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistent_keepalive: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preshared_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenVpnSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proto: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_auth: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cipher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_user_pass: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthernetSection {
    #[serde(rename = "mac-address", skip_serializing_if = "Option::is_none")]
    pub mac_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtu: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpConfigSection {
    pub method: String,  // auto, manual, ignore, link-local
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routes: Option<Vec<String>>,
}

impl NetctlConnectionConfig {
    /// Load configuration from TOML file
    pub async fn from_file<P: AsRef<Path>>(path: P) -> NetctlResult<Self> {
        let path = path.as_ref();
        info!("Loading config from: {}", path.display());

        let contents = fs::read_to_string(path).await?;
        let config: NetctlConnectionConfig = toml::from_str(&contents)
            .map_err(|e| NetctlError::ConfigError(format!("Invalid TOML: {}", e)))?;

        Ok(config)
    }

    /// Save configuration to TOML file
    pub async fn to_file<P: AsRef<Path>>(&self, path: P) -> NetctlResult<()> {
        let path = path.as_ref();
        info!("Saving config to: {}", path.display());

        let toml_str = toml::to_string_pretty(self)
            .map_err(|e| NetctlError::ConfigError(format!("Failed to serialize: {}", e)))?;

        fs::write(path, toml_str).await?;
        Ok(())
    }

    /// Convert to plugin ConnectionConfig format
    pub fn to_plugin_config(&self) -> crate::plugin::ConnectionConfig {
        let mut settings = HashMap::new();

        // Add wifi settings
        if let Some(ref wifi) = self.wifi {
            settings.insert("ssid".to_string(), serde_json::json!(wifi.ssid));
            settings.insert("mode".to_string(), serde_json::json!(wifi.mode));
            if let Some(ref bssid) = wifi.bssid {
                settings.insert("bssid".to_string(), serde_json::json!(bssid));
            }
            if let Some(channel) = wifi.channel {
                settings.insert("channel".to_string(), serde_json::json!(channel));
            }
        }

        // Add wifi security settings
        if let Some(ref security) = self.wifi_security {
            settings.insert("key-mgmt".to_string(), serde_json::json!(security.key_mgmt));
            if let Some(ref psk) = security.psk {
                settings.insert("psk".to_string(), serde_json::json!(psk));
            }
            if let Some(ref password) = security.password {
                settings.insert("password".to_string(), serde_json::json!(password));
            }
        }

        // Add VPN settings
        if let Some(ref vpn) = self.vpn {
            if let Some(ref remote) = vpn.remote {
                settings.insert("remote".to_string(), serde_json::json!(remote));
            }
            if let Some(port) = vpn.port {
                settings.insert("port".to_string(), serde_json::json!(port));
            }
            if let Some(ref proto) = vpn.proto {
                settings.insert("proto".to_string(), serde_json::json!(proto));
            }
            if let Some(ref ca) = vpn.ca {
                settings.insert("ca".to_string(), serde_json::json!(ca));
            }
            if let Some(ref cert) = vpn.cert {
                settings.insert("cert".to_string(), serde_json::json!(cert));
            }
            if let Some(ref key) = vpn.key {
                settings.insert("key".to_string(), serde_json::json!(key));
            }
            if let Some(ref config_file) = vpn.config_file {
                settings.insert("config_file".to_string(), serde_json::json!(config_file));
            }
            settings.insert("connection-type".to_string(), serde_json::json!(vpn.connection_type));
        }

        // Add IP settings
        if let Some(ref ipv4) = self.ipv4 {
            if let Some(ref address) = ipv4.address {
                settings.insert("ipv4_address".to_string(), serde_json::json!(address));
            }
            if let Some(ref gateway) = ipv4.gateway {
                settings.insert("ipv4_gateway".to_string(), serde_json::json!(gateway));
            }
            if let Some(ref dns) = ipv4.dns {
                settings.insert("ipv4_dns".to_string(), serde_json::json!(dns));
            }
        }

        crate::plugin::ConnectionConfig {
            uuid: self.connection.uuid.clone(),
            name: self.connection.name.clone(),
            conn_type: self.connection.conn_type.clone(),
            settings,
            autoconnect: self.connection.autoconnect,
        }
    }
}

/// Connection configuration directory manager
pub struct ConnectionConfigManager {
    config_dir: std::path::PathBuf,
}

impl ConnectionConfigManager {
    /// Create a new config manager
    pub fn new<P: AsRef<Path>>(config_dir: P) -> Self {
        Self {
            config_dir: config_dir.as_ref().to_path_buf(),
        }
    }

    /// Create config directory if it doesn't exist
    pub async fn initialize(&self) -> NetctlResult<()> {
        fs::create_dir_all(&self.config_dir).await?;
        Ok(())
    }

    /// List all configuration files
    pub async fn list_configs(&self) -> NetctlResult<Vec<String>> {
        let mut configs = Vec::new();
        let mut entries = fs::read_dir(&self.config_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "nctl" {
                    if let Some(name) = path.file_stem() {
                        configs.push(name.to_string_lossy().to_string());
                    }
                }
            }
        }

        Ok(configs)
    }

    /// Load a configuration by name
    pub async fn load_config(&self, name: &str) -> NetctlResult<NetctlConnectionConfig> {
        let path = self.config_dir.join(format!("{}.nctl", name));
        NetctlConnectionConfig::from_file(path).await
    }

    /// Save a configuration
    pub async fn save_config(&self, name: &str, config: &NetctlConnectionConfig) -> NetctlResult<()> {
        let path = self.config_dir.join(format!("{}.nctl", name));
        config.to_file(path).await
    }

    /// Delete a configuration
    pub async fn delete_config(&self, name: &str) -> NetctlResult<()> {
        let path = self.config_dir.join(format!("{}.nctl", name));
        fs::remove_file(path).await?;
        Ok(())
    }
}

impl Default for ConnectionConfigManager {
    fn default() -> Self {
        Self::new("/etc/netctl/connections")
    }
}
