use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;
use tokio::process::Command;
use tracing::{info, warn};

use crate::plugin::ConnectionConfig;
use crate::error::{NetctlError, NetctlResult};
use super::backend::{VpnBackend, VpnState, VpnStats};
use super::common;

/// WireGuard VPN backend implementation
pub struct WireGuardBackend {
    interface_name: Option<String>,
    config_path: Option<std::path::PathBuf>,
    connected_since: Option<SystemTime>,
}

impl WireGuardBackend {
    /// Create a new WireGuard backend instance
    pub fn new() -> Self {
        Self {
            interface_name: None,
            config_path: None,
            connected_since: None,
        }
    }

    /// Build WireGuard configuration file content
    fn build_config_content(&self, config: &ConnectionConfig) -> NetctlResult<String> {
        let settings = &config.settings;
        let mut cfg = String::new();

        // [Interface] section
        cfg.push_str("[Interface]\n");

        if let Some(private_key) = settings.get("private_key").and_then(|v| v.as_str()) {
            cfg.push_str(&format!("PrivateKey = {}\n", private_key));
        } else {
            return Err(NetctlError::InvalidParameter("Missing 'private_key'".to_string()));
        }

        if let Some(address) = settings.get("address").and_then(|v| v.as_str()) {
            cfg.push_str(&format!("Address = {}\n", address));
        }

        if let Some(listen_port) = settings.get("listen_port").and_then(|v| v.as_u64()) {
            cfg.push_str(&format!("ListenPort = {}\n", listen_port));
        }

        if let Some(dns) = settings.get("dns").and_then(|v| v.as_str()) {
            cfg.push_str(&format!("DNS = {}\n", dns));
        }

        if let Some(mtu) = settings.get("mtu").and_then(|v| v.as_u64()) {
            cfg.push_str(&format!("MTU = {}\n", mtu));
        }

        if let Some(table) = settings.get("table").and_then(|v| v.as_str()) {
            cfg.push_str(&format!("Table = {}\n", table));
        }

        // [Peer] section(s)
        if let Some(peer) = settings.get("peer") {
            cfg.push_str("\n[Peer]\n");

            if let Some(peer_obj) = peer.as_object() {
                if let Some(public_key) = peer_obj.get("public_key").and_then(|v| v.as_str()) {
                    cfg.push_str(&format!("PublicKey = {}\n", public_key));
                } else {
                    return Err(NetctlError::InvalidParameter("Missing peer 'public_key'".to_string()));
                }

                if let Some(allowed_ips) = peer_obj.get("allowed_ips").and_then(|v| v.as_str()) {
                    cfg.push_str(&format!("AllowedIPs = {}\n", allowed_ips));
                }

                if let Some(endpoint) = peer_obj.get("endpoint").and_then(|v| v.as_str()) {
                    cfg.push_str(&format!("Endpoint = {}\n", endpoint));
                }

                if let Some(keepalive) = peer_obj.get("persistent_keepalive").and_then(|v| v.as_u64()) {
                    cfg.push_str(&format!("PersistentKeepalive = {}\n", keepalive));
                }

                if let Some(preshared_key) = peer_obj.get("preshared_key").and_then(|v| v.as_str()) {
                    cfg.push_str(&format!("PresharedKey = {}\n", preshared_key));
                }
            }
        }

        // Support multiple peers
        if let Some(peers) = settings.get("peers").and_then(|v| v.as_array()) {
            for peer in peers {
                cfg.push_str("\n[Peer]\n");

                if let Some(peer_obj) = peer.as_object() {
                    if let Some(public_key) = peer_obj.get("public_key").and_then(|v| v.as_str()) {
                        cfg.push_str(&format!("PublicKey = {}\n", public_key));
                    }

                    if let Some(allowed_ips) = peer_obj.get("allowed_ips").and_then(|v| v.as_str()) {
                        cfg.push_str(&format!("AllowedIPs = {}\n", allowed_ips));
                    }

                    if let Some(endpoint) = peer_obj.get("endpoint").and_then(|v| v.as_str()) {
                        cfg.push_str(&format!("Endpoint = {}\n", endpoint));
                    }

                    if let Some(keepalive) = peer_obj.get("persistent_keepalive").and_then(|v| v.as_u64()) {
                        cfg.push_str(&format!("PersistentKeepalive = {}\n", keepalive));
                    }

                    if let Some(preshared_key) = peer_obj.get("preshared_key").and_then(|v| v.as_str()) {
                        cfg.push_str(&format!("PresharedKey = {}\n", preshared_key));
                    }
                }
            }
        }

        Ok(cfg)
    }

    /// Generate interface name from connection UUID
    fn generate_interface_name(uuid: &str) -> String {
        format!("wg-{}", &uuid[..std::cmp::min(8, uuid.len())])
    }

    /// Parse WireGuard configuration file
    async fn parse_wg_config(path: &Path) -> NetctlResult<HashMap<String, Value>> {
        let content = common::read_config_file(path).await?;
        let mut settings = HashMap::new();
        let mut current_section = String::new();
        let mut peer_data: Option<HashMap<String, Value>> = None;
        let mut peers: Vec<Value> = Vec::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            // Section headers
            if line.starts_with('[') && line.ends_with(']') {
                // Save previous peer if exists
                if current_section == "Peer" {
                    if let Some(peer) = peer_data.take() {
                        peers.push(json!(peer));
                    }
                }

                current_section = line[1..line.len()-1].to_string();
                if current_section == "Peer" {
                    peer_data = Some(HashMap::new());
                }
                continue;
            }

            // Parse key = value
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_lowercase().replace(" ", "_");
                let value = value.trim();

                match current_section.as_str() {
                    "Interface" => {
                        settings.insert(key, json!(value));
                    }
                    "Peer" => {
                        if let Some(ref mut peer) = peer_data {
                            peer.insert(key, json!(value));
                        }
                    }
                    _ => {}
                }
            }
        }

        // Save last peer if exists
        if let Some(peer) = peer_data {
            peers.push(json!(peer));
        }

        // Add peers to settings
        if !peers.is_empty() {
            if peers.len() == 1 {
                if let Some(peer) = peers.into_iter().next() {
                    settings.insert("peer".to_string(), peer);
                }
            } else {
                settings.insert("peers".to_string(), json!(peers));
            }
        }

        Ok(settings)
    }
}

#[async_trait]
impl VpnBackend for WireGuardBackend {
    fn name(&self) -> &str {
        "wireguard"
    }

    async fn version(&self) -> NetctlResult<String> {
        common::get_binary_version("wg").await
    }

    async fn is_available(&self) -> bool {
        common::check_binary_available("wg").await &&
        common::check_binary_available("wg-quick").await
    }

    async fn validate_config(&self, config: &ConnectionConfig) -> NetctlResult<()> {
        let settings = &config.settings;

        // Check required fields
        if !settings.contains_key("private_key") {
            return Err(NetctlError::InvalidParameter(
                "WireGuard 'private_key' is required".to_string()
            ));
        }

        // Validate address if present
        if let Some(address) = settings.get("address").and_then(|v| v.as_str()) {
            if !common::is_valid_cidr(address) {
                return Err(NetctlError::InvalidParameter(
                    format!("Invalid address CIDR: {}", address)
                ));
            }
        }

        // Validate peer configuration
        if let Some(peer) = settings.get("peer").and_then(|v| v.as_object()) {
            if !peer.contains_key("public_key") {
                return Err(NetctlError::InvalidParameter(
                    "Peer 'public_key' is required".to_string()
                ));
            }

            if let Some(endpoint) = peer.get("endpoint").and_then(|v| v.as_str()) {
                if !endpoint.contains(':') {
                    return Err(NetctlError::InvalidParameter(
                        "Peer endpoint must be in format 'host:port'".to_string()
                    ));
                }
            }

            if let Some(allowed_ips) = peer.get("allowed_ips").and_then(|v| v.as_str()) {
                for ip in allowed_ips.split(',') {
                    let ip = ip.trim();
                    if !common::is_valid_cidr(ip) {
                        return Err(NetctlError::InvalidParameter(
                            format!("Invalid allowed IP CIDR: {}", ip)
                        ));
                    }
                }
            }
        }

        // Validate peers array if present
        if let Some(peers) = settings.get("peers").and_then(|v| v.as_array()) {
            for peer in peers {
                if let Some(peer_obj) = peer.as_object() {
                    if !peer_obj.contains_key("public_key") {
                        return Err(NetctlError::InvalidParameter(
                            "Each peer must have 'public_key'".to_string()
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    async fn connect(&mut self, config: &ConnectionConfig) -> NetctlResult<String> {
        info!("Connecting WireGuard VPN: {}", config.name);

        // Generate interface name
        let interface_name = Self::generate_interface_name(&config.uuid);

        // Build configuration content
        let config_content = self.build_config_content(config)?;

        // Write configuration to temporary file
        let config_path = std::env::temp_dir().join(format!("{}.conf", interface_name));
        common::write_secure_config(&config_path, &config_content, 0o600).await?;

        // Bring up interface with wg-quick
        let config_path_str = config_path.to_str()
            .ok_or_else(|| NetctlError::InvalidParameter(
                "Config path contains invalid UTF-8".to_string()
            ))?;
        let output = Command::new("wg-quick")
            .args(&["up", config_path_str])
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to start WireGuard: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            common::delete_config_file(&config_path).await.ok();
            return Err(NetctlError::ServiceError(
                format!("wg-quick failed: {}", error_msg)
            ));
        }

        self.interface_name = Some(interface_name.clone());
        self.config_path = Some(config_path);
        self.connected_since = Some(SystemTime::now());

        info!("WireGuard VPN connected: {} (interface: {})", config.name, interface_name);
        Ok(interface_name)
    }

    async fn disconnect(&mut self) -> NetctlResult<()> {
        if let Some(interface_name) = &self.interface_name {
            info!("Disconnecting WireGuard VPN: {}", interface_name);

            let output = Command::new("wg-quick")
                .args(&["down", interface_name])
                .output()
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to stop WireGuard: {}", e)))?;

            if !output.status.success() {
                warn!("wg-quick down failed: {}", String::from_utf8_lossy(&output.stderr));
            }

            // Clean up config file
            if let Some(config_path) = &self.config_path {
                common::delete_config_file(config_path).await.ok();
            }

            self.interface_name = None;
            self.config_path = None;
            self.connected_since = None;

            info!("WireGuard VPN disconnected");
        }

        Ok(())
    }

    async fn state(&self) -> VpnState {
        if let Some(interface_name) = &self.interface_name {
            if common::interface_exists(interface_name).await {
                VpnState::Connected
            } else {
                VpnState::Disconnected
            }
        } else {
            VpnState::Disconnected
        }
    }

    async fn stats(&self) -> NetctlResult<VpnStats> {
        let mut stats = VpnStats::default();
        stats.connected_since = self.connected_since;

        if let Some(interface_name) = &self.interface_name {
            // Get basic interface statistics
            if let Ok((rx_bytes, tx_bytes)) = common::get_interface_stats(interface_name).await {
                stats.bytes_received = rx_bytes;
                stats.bytes_sent = tx_bytes;
            }

            // Parse wg show output for more detailed stats
            if let Ok(output) = Command::new("wg")
                .args(&["show", interface_name, "dump"])
                .output()
                .await
            {
                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    for line in output_str.lines().skip(1) {
                        let parts: Vec<&str> = line.split('\t').collect();
                        if parts.len() >= 6 {
                            // Parse peer stats
                            if let Ok(rx) = parts[5].parse::<u64>() {
                                stats.bytes_received = rx;
                            }
                            if let Ok(tx) = parts[4].parse::<u64>() {
                                stats.bytes_sent = tx;
                            }

                            // Last handshake time
                            if let Ok(timestamp) = parts[3].parse::<u64>() {
                                if timestamp > 0 {
                                    stats.last_handshake = Some(
                                        SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(timestamp)
                                    );
                                }
                            }

                            // Peer endpoint
                            if parts.len() >= 3 {
                                stats.peer_endpoint = Some(parts[2].to_string());
                            }
                        }
                    }
                }
            }
        }

        Ok(stats)
    }

    fn interface_name(&self) -> Option<String> {
        self.interface_name.clone()
    }

    async fn status_json(&self) -> NetctlResult<Value> {
        let state = self.state().await;
        let stats = self.stats().await.unwrap_or_default();

        Ok(json!({
            "backend": "wireguard",
            "state": format!("{:?}", state),
            "interface": self.interface_name,
            "connected_since": stats.connected_since.map(|t| format!("{:?}", t)),
            "bytes_sent": stats.bytes_sent,
            "bytes_received": stats.bytes_received,
            "last_handshake": stats.last_handshake.map(|t| format!("{:?}", t)),
            "peer_endpoint": stats.peer_endpoint,
        }))
    }

    async fn import_config(&self, path: &Path) -> NetctlResult<HashMap<String, Value>> {
        info!("Importing WireGuard configuration from: {:?}", path);
        Self::parse_wg_config(path).await
    }

    async fn export_config(&self, config: &ConnectionConfig, path: &Path) -> NetctlResult<()> {
        info!("Exporting WireGuard configuration to: {:?}", path);
        let config_content = self.build_config_content(config)?;
        common::write_secure_config(path, &config_content, 0o600).await
    }
}

/// Factory function to create a WireGuard backend
pub fn create_backend() -> Box<dyn VpnBackend> {
    Box::new(WireGuardBackend::new())
}
