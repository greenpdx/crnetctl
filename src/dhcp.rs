//! DHCP server management via dora

use crate::error::{NetctlError, NetctlResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhcpConfig {
    pub interface: String,
    pub range_start: String,
    pub range_end: String,
    pub gateway: String,
    pub dns_servers: Vec<String>,
    pub lease_time: u32,
    pub domain: Option<String>,
}

impl Default for DhcpConfig {
    fn default() -> Self {
        Self {
            interface: "wlan0".to_string(),
            range_start: "10.255.24.10".to_string(),
            range_end: "10.255.24.250".to_string(),
            gateway: "10.255.24.1".to_string(),
            dns_servers: vec!["10.255.24.1".to_string()],
            lease_time: 3600,
            domain: Some("local".to_string()),
        }
    }
}

pub struct DhcpController {
    config_path: PathBuf,
    dora_bin: PathBuf,
}

impl DhcpController {
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            dora_bin: PathBuf::from("/usr/local/bin/dora"),
        }
    }

    pub fn generate_config(&self, config: &DhcpConfig) -> NetctlResult<String> {
        let mut yaml = String::new();
        yaml.push_str(&format!("{}:\n", config.interface));
        yaml.push_str(&format!("  interfaces:\n    - {}\n", config.interface));
        yaml.push_str("  ranges:\n");
        yaml.push_str(&format!("    - start: {}\n", config.range_start));
        yaml.push_str(&format!("      end: {}\n", config.range_end));
        yaml.push_str("  options:\n");
        yaml.push_str("    - opt: 1\n      val: !ip 255.255.255.0\n");
        yaml.push_str(&format!("    - opt: 3\n      val: !ip {}\n", config.gateway));

        if !config.dns_servers.is_empty() {
            yaml.push_str(&format!("    - opt: 6\n      val: !ips [{}]\n",
                config.dns_servers.join(", ")));
        }

        yaml.push_str(&format!("    - opt: 51\n      val: !u32 {}\n", config.lease_time));
        yaml.push_str("  ping_check: false\n  probe_check: false\n");

        Ok(yaml)
    }

    pub async fn write_config(&self, config: &DhcpConfig) -> NetctlResult<()> {
        let yaml = self.generate_config(config)?;
        fs::write(&self.config_path, yaml).await?;
        Ok(())
    }
}
