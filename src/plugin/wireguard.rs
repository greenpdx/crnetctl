//! WireGuard VPN plugin implementation

use super::traits::*;
use crate::error::{NetctlError, NetctlResult};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

/// WireGuard plugin
pub struct WireGuardPlugin {
    metadata: PluginMetadata,
    state: PluginState,
    enabled: bool,
    connections: RwLock<HashMap<String, WireGuardConnection>>,
    config_dir: PathBuf,
}

/// WireGuard connection instance
struct WireGuardConnection {
    uuid: String,
    config: ConnectionConfig,
    state: PluginState,
    interface_name: String,
    stats: ConnectionStats,
    start_time: Option<std::time::Instant>,
}

impl WireGuardPlugin {
    /// Create a new WireGuard plugin instance
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            metadata: PluginMetadata {
                id: "wireguard".to_string(),
                name: "WireGuard".to_string(),
                version: "1.0.0".to_string(),
                description: "WireGuard VPN connection support".to_string(),
                author: "netctl team".to_string(),
                capabilities: vec![PluginCapability::Vpn, PluginCapability::TunTap, PluginCapability::Ipv6],
                dbus_service: Some("org.freedesktop.NetworkManager.wireguard".to_string()),
                dbus_path: Some("/org/freedesktop/NetworkManager/wireguard".to_string()),
            },
            state: PluginState::Uninitialized,
            enabled: false,
            connections: RwLock::new(HashMap::new()),
            config_dir,
        }
    }

    /// Validate WireGuard configuration
    fn validate_wireguard_config(settings: &HashMap<String, serde_json::Value>) -> NetctlResult<()> {
        // Check required fields
        if !settings.contains_key("private_key") {
            return Err(NetctlError::InvalidParameter(
                "WireGuard 'private_key' is required".to_string()
            ));
        }

        if !settings.contains_key("listen_port") && !settings.contains_key("peer_endpoint") {
            return Err(NetctlError::InvalidParameter(
                "Either 'listen_port' or 'peer_endpoint' must be specified".to_string()
            ));
        }

        Ok(())
    }

    /// Build WireGuard configuration file content
    fn build_config_content(&self, conn: &WireGuardConnection) -> NetctlResult<String> {
        let settings = &conn.config.settings;
        let mut config = String::new();

        // [Interface] section
        config.push_str("[Interface]\n");

        if let Some(private_key) = settings.get("private_key") {
            config.push_str(&format!("PrivateKey = {}\n", private_key.as_str().unwrap_or("")));
        }

        if let Some(address) = settings.get("address") {
            config.push_str(&format!("Address = {}\n", address.as_str().unwrap_or("")));
        }

        if let Some(listen_port) = settings.get("listen_port") {
            config.push_str(&format!("ListenPort = {}\n", listen_port.as_u64().unwrap_or(51820)));
        }

        if let Some(dns) = settings.get("dns") {
            config.push_str(&format!("DNS = {}\n", dns.as_str().unwrap_or("")));
        }

        // [Peer] section
        if let Some(peer) = settings.get("peer") {
            config.push_str("\n[Peer]\n");

            if let Some(peer_obj) = peer.as_object() {
                if let Some(public_key) = peer_obj.get("public_key") {
                    config.push_str(&format!("PublicKey = {}\n", public_key.as_str().unwrap_or("")));
                }

                if let Some(allowed_ips) = peer_obj.get("allowed_ips") {
                    config.push_str(&format!("AllowedIPs = {}\n", allowed_ips.as_str().unwrap_or("")));
                }

                if let Some(endpoint) = peer_obj.get("endpoint") {
                    config.push_str(&format!("Endpoint = {}\n", endpoint.as_str().unwrap_or("")));
                }

                if let Some(keepalive) = peer_obj.get("persistent_keepalive") {
                    config.push_str(&format!("PersistentKeepalive = {}\n", keepalive.as_u64().unwrap_or(25)));
                }
            }
        }

        Ok(config)
    }

    /// Get interface name for connection
    fn get_interface_name(&self, uuid: &str) -> String {
        format!("wg-{}", &uuid[..8])
    }
}

#[async_trait]
impl NetworkPlugin for WireGuardPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(&mut self) -> NetctlResult<()> {
        info!("Initializing WireGuard plugin");
        self.state = PluginState::Initializing;

        // Check if wg binary is available
        match Command::new("wg").arg("--version").output().await {
            Ok(output) => {
                if output.status.success() {
                    info!("WireGuard tools found");
                    self.state = PluginState::Ready;
                    Ok(())
                } else {
                    Err(NetctlError::NotSupported("WireGuard tools not functional".to_string()))
                }
            }
            Err(e) => {
                Err(NetctlError::NotSupported(format!("WireGuard not found: {}", e)))
            }
        }
    }

    async fn shutdown(&mut self) -> NetctlResult<()> {
        info!("Shutting down WireGuard plugin");

        // Stop all active connections
        let mut connections = self.connections.write().await;
        for (uuid, conn) in connections.iter() {
            info!("Stopping WireGuard connection: {}", uuid);
            let _ = Command::new("wg-quick")
                .args(&["down", &conn.interface_name])
                .output()
                .await;
        }
        connections.clear();

        self.state = PluginState::Uninitialized;
        Ok(())
    }

    fn state(&self) -> PluginState {
        self.state
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    async fn enable(&mut self) -> NetctlResult<()> {
        self.enabled = true;
        Ok(())
    }

    async fn disable(&mut self) -> NetctlResult<()> {
        self.enabled = false;
        Ok(())
    }

    async fn validate_config(&self, config: &ConnectionConfig) -> NetctlResult<()> {
        if config.conn_type != "vpn" && config.conn_type != "wireguard" {
            return Err(NetctlError::InvalidParameter(
                format!("Invalid connection type: {}", config.conn_type)
            ));
        }

        Self::validate_wireguard_config(&config.settings)
    }

    async fn create_connection(&mut self, config: ConnectionConfig) -> NetctlResult<String> {
        let uuid = config.uuid.clone();
        info!("Creating WireGuard connection: {}", uuid);

        let interface_name = self.get_interface_name(&uuid);

        let conn = WireGuardConnection {
            uuid: uuid.clone(),
            config,
            state: PluginState::Ready,
            interface_name,
            stats: ConnectionStats {
                rx_bytes: 0,
                tx_bytes: 0,
                rx_packets: 0,
                tx_packets: 0,
                uptime: 0,
            },
            start_time: None,
        };

        let mut connections = self.connections.write().await;
        connections.insert(uuid.clone(), conn);

        Ok(uuid)
    }

    async fn delete_connection(&mut self, uuid: &str) -> NetctlResult<()> {
        info!("Deleting WireGuard connection: {}", uuid);

        // Deactivate first if active
        if let Ok(state) = self.get_status(uuid).await {
            if state == PluginState::Active {
                self.deactivate(uuid).await?;
            }
        }

        let mut connections = self.connections.write().await;
        connections.remove(uuid);

        Ok(())
    }

    async fn activate(&mut self, uuid: &str) -> NetctlResult<()> {
        info!("Activating WireGuard connection: {}", uuid);

        let mut connections = self.connections.write().await;
        let conn = connections.get_mut(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        conn.state = PluginState::Activating;

        // Write configuration file
        let config_content = self.build_config_content(conn)?;
        let config_path = self.config_dir.join(format!("{}.conf", conn.interface_name));
        tokio::fs::write(&config_path, config_content).await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to write config: {}", e)))?;

        // Bring up interface with wg-quick
        let output = Command::new("wg-quick")
            .args(&["up", config_path.to_str().unwrap()])
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to start WireGuard: {}", e)))?;

        if !output.status.success() {
            conn.state = PluginState::Failed;
            return Err(NetctlError::ServiceError(
                format!("wg-quick failed: {}", String::from_utf8_lossy(&output.stderr))
            ));
        }

        conn.state = PluginState::Active;
        conn.start_time = Some(std::time::Instant::now());

        info!("WireGuard connection {} activated", uuid);
        Ok(())
    }

    async fn deactivate(&mut self, uuid: &str) -> NetctlResult<()> {
        info!("Deactivating WireGuard connection: {}", uuid);

        let mut connections = self.connections.write().await;
        let conn = connections.get_mut(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        conn.state = PluginState::Deactivating;

        let output = Command::new("wg-quick")
            .args(&["down", &conn.interface_name])
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to stop WireGuard: {}", e)))?;

        if !output.status.success() {
            warn!("wg-quick down failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        conn.state = PluginState::Ready;
        conn.start_time = None;

        info!("WireGuard connection {} deactivated", uuid);
        Ok(())
    }

    async fn get_status(&self, uuid: &str) -> NetctlResult<PluginState> {
        let connections = self.connections.read().await;
        let conn = connections.get(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        Ok(conn.state)
    }

    async fn get_stats(&self, uuid: &str) -> NetctlResult<ConnectionStats> {
        let connections = self.connections.read().await;
        let conn = connections.get(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        let mut stats = conn.stats.clone();

        // Calculate uptime
        if let Some(start_time) = conn.start_time {
            stats.uptime = start_time.elapsed().as_secs();
        }

        // TODO: Parse actual stats from `wg show` command

        Ok(stats)
    }

    async fn list_connections(&self) -> NetctlResult<Vec<ConnectionConfig>> {
        let connections = self.connections.read().await;
        Ok(connections.values().map(|c| c.config.clone()).collect())
    }

    async fn update_connection(&mut self, uuid: &str, config: ConnectionConfig) -> NetctlResult<()> {
        self.validate_config(&config).await?;

        let mut connections = self.connections.write().await;
        let conn = connections.get_mut(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        conn.config = config;
        Ok(())
    }

    fn settings_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "private_key": {
                    "type": "string",
                    "description": "WireGuard private key"
                },
                "address": {
                    "type": "string",
                    "description": "Interface IP address (e.g., 10.0.0.2/24)"
                },
                "listen_port": {
                    "type": "integer",
                    "default": 51820,
                    "description": "UDP port to listen on"
                },
                "dns": {
                    "type": "string",
                    "description": "DNS servers (comma-separated)"
                },
                "peer": {
                    "type": "object",
                    "properties": {
                        "public_key": {
                            "type": "string",
                            "description": "Peer's public key"
                        },
                        "allowed_ips": {
                            "type": "string",
                            "description": "Allowed IPs (e.g., 0.0.0.0/0)"
                        },
                        "endpoint": {
                            "type": "string",
                            "description": "Peer endpoint (host:port)"
                        },
                        "persistent_keepalive": {
                            "type": "integer",
                            "default": 25,
                            "description": "Keepalive interval in seconds"
                        }
                    }
                }
            }
        })
    }
}
