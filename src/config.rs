//! Configuration management for netctl

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::error::{NetctlError, NetctlResult};

/// Main netctl configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetctlConfig {
    /// Configuration file paths
    pub paths: ConfigPaths,
    /// Default settings
    pub defaults: DefaultSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigPaths {
    /// Base configuration directory
    #[serde(default = "default_config_dir")]
    pub config_dir: PathBuf,
    /// Runtime state directory
    #[serde(default = "default_state_dir")]
    pub state_dir: PathBuf,
    /// Log directory
    #[serde(default = "default_log_dir")]
    pub log_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultSettings {
    /// Default interface MTU
    #[serde(default = "default_mtu")]
    pub mtu: u32,
    /// Default DHCP lease time (seconds)
    #[serde(default = "default_lease_time")]
    pub dhcp_lease_time: u32,
    /// Default DNS cache size
    #[serde(default = "default_dns_cache_size")]
    pub dns_cache_size: u32,
}

fn default_config_dir() -> PathBuf {
    PathBuf::from("/etc/crrouter/netctl")
}

fn default_state_dir() -> PathBuf {
    PathBuf::from("/run/crrouter/netctl")
}

fn default_log_dir() -> PathBuf {
    PathBuf::from("/var/log/crrouter/netctl")
}

fn default_mtu() -> u32 {
    1500
}

fn default_lease_time() -> u32 {
    3600
}

fn default_dns_cache_size() -> u32 {
    100
}

impl Default for NetctlConfig {
    fn default() -> Self {
        Self {
            paths: ConfigPaths {
                config_dir: default_config_dir(),
                state_dir: default_state_dir(),
                log_dir: default_log_dir(),
            },
            defaults: DefaultSettings {
                mtu: default_mtu(),
                dhcp_lease_time: default_lease_time(),
                dns_cache_size: default_dns_cache_size(),
            },
        }
    }
}

impl NetctlConfig {
    /// Load configuration from file
    pub fn load<P: AsRef<Path>>(path: P) -> NetctlResult<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| NetctlError::ConfigError(format!("Failed to read config: {}", e)))?;

        toml::from_str(&content)
            .map_err(|e| NetctlError::ConfigError(format!("Failed to parse config: {}", e)))
    }

    /// Save configuration to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> NetctlResult<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| NetctlError::ConfigError(format!("Failed to serialize config: {}", e)))?;

        std::fs::write(path.as_ref(), content)
            .map_err(|e| NetctlError::ConfigError(format!("Failed to write config: {}", e)))?;

        Ok(())
    }

    /// Ensure all directories exist
    pub fn ensure_directories(&self) -> NetctlResult<()> {
        for dir in [&self.paths.config_dir, &self.paths.state_dir, &self.paths.log_dir] {
            std::fs::create_dir_all(dir)
                .map_err(|e| NetctlError::ConfigError(format!("Failed to create directory {:?}: {}", dir, e)))?;
        }
        Ok(())
    }
}
