//! DHCP server management via dora

use crate::error::NetctlResult;
use crate::validation;
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
    #[allow(dead_code)]
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
        // Validate all user-provided configuration values
        validation::validate_interface_name(&config.interface)?;
        validation::validate_ip_address(&config.range_start)?;
        validation::validate_ip_address(&config.range_end)?;
        validation::validate_ip_address(&config.gateway)?;

        // Validate all DNS server IPs
        for dns in &config.dns_servers {
            validation::validate_ip_address(dns)?;
        }

        // Use sanitized values for config generation
        let interface = validation::sanitize_config_value(&config.interface)?;
        let range_start = validation::sanitize_config_value(&config.range_start)?;
        let range_end = validation::sanitize_config_value(&config.range_end)?;
        let gateway = validation::sanitize_config_value(&config.gateway)?;

        let mut yaml = String::new();
        yaml.push_str(&format!("{}:\n", interface));
        yaml.push_str(&format!("  interfaces:\n    - {}\n", interface));
        yaml.push_str("  ranges:\n");
        yaml.push_str(&format!("    - start: {}\n", range_start));
        yaml.push_str(&format!("      end: {}\n", range_end));
        yaml.push_str("  options:\n");
        yaml.push_str("    - opt: 1\n      val: !ip 255.255.255.0\n");
        yaml.push_str(&format!("    - opt: 3\n      val: !ip {}\n", gateway));

        if !config.dns_servers.is_empty() {
            // Sanitize DNS servers
            let dns_sanitized: Result<Vec<_>, _> = config.dns_servers.iter()
                .map(|dns| validation::sanitize_config_value(dns))
                .collect();
            let dns_servers = dns_sanitized?;
            yaml.push_str(&format!("    - opt: 6\n      val: !ips [{}]\n",
                dns_servers.join(", ")));
        }

        yaml.push_str(&format!("    - opt: 51\n      val: !u32 {}\n", config.lease_time));
        yaml.push_str("  ping_check: false\n  probe_check: false\n");

        Ok(yaml)
    }

    pub async fn write_config(&self, config: &DhcpConfig) -> NetctlResult<()> {
        let yaml = self.generate_config(config)?;

        // Validate the config path to prevent path traversal
        // Use the parent directory of config_path as the allowed base
        if let Some(base_dir) = self.config_path.parent() {
            let validated_path = validation::validate_config_path(&self.config_path, base_dir)?;
            fs::write(&validated_path, yaml).await?;
        } else {
            // If no parent directory, write directly (shouldn't normally happen)
            fs::write(&self.config_path, yaml).await?;
        }

        Ok(())
    }
}
