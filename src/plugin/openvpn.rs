//! OpenVPN plugin implementation

use super::traits::*;
use crate::error::{NetctlError, NetctlResult};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tracing::info;

/// OpenVPN plugin
pub struct OpenVpnPlugin {
    metadata: PluginMetadata,
    state: PluginState,
    enabled: bool,
    connections: RwLock<HashMap<String, OpenVpnConnection>>,
    #[allow(dead_code)]
    config_dir: PathBuf,
}

/// OpenVPN connection instance
struct OpenVpnConnection {
    #[allow(dead_code)]
    uuid: String,
    config: ConnectionConfig,
    state: PluginState,
    process: Option<Child>,
    stats: ConnectionStats,
    start_time: Option<std::time::Instant>,
}

impl OpenVpnPlugin {
    /// Create a new OpenVPN plugin instance
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            metadata: PluginMetadata {
                id: "openvpn".to_string(),
                name: "OpenVPN".to_string(),
                version: "1.0.0".to_string(),
                description: "OpenVPN VPN connection support".to_string(),
                author: "netctl team".to_string(),
                capabilities: vec![PluginCapability::Vpn, PluginCapability::TunTap],
                dbus_service: Some("org.freedesktop.NetworkManager.openvpn".to_string()),
                dbus_path: Some("/org/freedesktop/NetworkManager/openvpn".to_string()),
            },
            state: PluginState::Uninitialized,
            enabled: false,
            connections: RwLock::new(HashMap::new()),
            config_dir,
        }
    }

    /// Validate OpenVPN configuration
    fn validate_openvpn_config(settings: &HashMap<String, serde_json::Value>) -> NetctlResult<()> {
        // Check required fields
        if !settings.contains_key("config_file") && !settings.contains_key("remote") {
            return Err(NetctlError::InvalidParameter(
                "Either 'config_file' or 'remote' must be specified".to_string()
            ));
        }

        // Validate connection type
        if let Some(conn_type) = settings.get("connection-type") {
            let type_str = conn_type.as_str().ok_or_else(|| {
                NetctlError::InvalidParameter("connection-type must be a string".to_string())
            })?;

            if !["tun", "tap"].contains(&type_str) {
                return Err(NetctlError::InvalidParameter(
                    "connection-type must be 'tun' or 'tap'".to_string()
                ));
            }
        }

        Ok(())
    }

    /// Build OpenVPN command arguments
    fn build_command_args(
        &self,
        conn: &OpenVpnConnection,
    ) -> NetctlResult<Vec<String>> {
        let settings = &conn.config.settings;
        let mut args = Vec::new();

        // Use config file if specified
        if let Some(config_file) = settings.get("config_file") {
            let path = config_file.as_str().ok_or_else(|| {
                NetctlError::InvalidParameter("config_file must be a string".to_string())
            })?;
            args.push("--config".to_string());
            args.push(path.to_string());
        } else {
            // Build command line from individual settings
            if let Some(remote) = settings.get("remote") {
                let remote_str = remote.as_str().ok_or_else(|| {
                    NetctlError::InvalidParameter("remote must be a string".to_string())
                })?;
                args.push("--remote".to_string());
                args.push(remote_str.to_string());
            }

            if let Some(port) = settings.get("port") {
                let port_num = port.as_u64().ok_or_else(|| {
                    NetctlError::InvalidParameter("port must be a number".to_string())
                })?;
                args.push("--port".to_string());
                args.push(port_num.to_string());
            }

            if let Some(proto) = settings.get("proto") {
                let proto_str = proto.as_str().ok_or_else(|| {
                    NetctlError::InvalidParameter("proto must be a string".to_string())
                })?;
                args.push("--proto".to_string());
                args.push(proto_str.to_string());
            }

            if let Some(dev_type) = settings.get("connection-type") {
                let type_str = dev_type.as_str().unwrap_or("tun");
                args.push("--dev-type".to_string());
                args.push(type_str.to_string());
            }

            if let Some(ca) = settings.get("ca").and_then(|v| v.as_str()) {
                args.push("--ca".to_string());
                args.push(ca.to_string());
            }

            if let Some(cert) = settings.get("cert").and_then(|v| v.as_str()) {
                args.push("--cert".to_string());
                args.push(cert.to_string());
            }

            if let Some(key) = settings.get("key").and_then(|v| v.as_str()) {
                args.push("--key".to_string());
                args.push(key.to_string());
            }
        }

        // Common options
        args.push("--nobind".to_string());
        args.push("--persist-key".to_string());
        args.push("--persist-tun".to_string());

        Ok(args)
    }
}

#[async_trait]
impl NetworkPlugin for OpenVpnPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(&mut self) -> NetctlResult<()> {
        info!("Initializing OpenVPN plugin");
        self.state = PluginState::Initializing;

        // Check if openvpn binary is available
        match Command::new("openvpn").arg("--version").output().await {
            Ok(output) => {
                if output.status.success() {
                    info!("OpenVPN binary found");
                    self.state = PluginState::Ready;
                    Ok(())
                } else {
                    Err(NetctlError::NotSupported("OpenVPN binary not functional".to_string()))
                }
            }
            Err(e) => {
                Err(NetctlError::NotSupported(format!("OpenVPN not found: {}", e)))
            }
        }
    }

    async fn shutdown(&mut self) -> NetctlResult<()> {
        info!("Shutting down OpenVPN plugin");

        // Stop all active connections
        let mut connections = self.connections.write().await;
        for (uuid, conn) in connections.iter_mut() {
            if let Some(ref mut process) = conn.process {
                info!("Stopping OpenVPN connection: {}", uuid);
                let _ = process.kill().await;
            }
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
        if config.conn_type != "vpn" && config.conn_type != "openvpn" {
            return Err(NetctlError::InvalidParameter(
                format!("Invalid connection type: {}", config.conn_type)
            ));
        }

        Self::validate_openvpn_config(&config.settings)
    }

    async fn create_connection(&mut self, config: ConnectionConfig) -> NetctlResult<String> {
        let uuid = config.uuid.clone();
        info!("Creating OpenVPN connection: {}", uuid);

        let conn = OpenVpnConnection {
            uuid: uuid.clone(),
            config,
            state: PluginState::Ready,
            process: None,
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
        info!("Deleting OpenVPN connection: {}", uuid);

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
        info!("Activating OpenVPN connection: {}", uuid);

        let mut connections = self.connections.write().await;
        let conn = connections.get_mut(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        conn.state = PluginState::Activating;

        // Build command
        let args = self.build_command_args(conn)?;

        // Start OpenVPN process
        let child = Command::new("openvpn")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| NetctlError::ServiceError(format!("Failed to start OpenVPN: {}", e)))?;

        conn.process = Some(child);
        conn.state = PluginState::Active;
        conn.start_time = Some(std::time::Instant::now());

        info!("OpenVPN connection {} activated", uuid);
        Ok(())
    }

    async fn deactivate(&mut self, uuid: &str) -> NetctlResult<()> {
        info!("Deactivating OpenVPN connection: {}", uuid);

        let mut connections = self.connections.write().await;
        let conn = connections.get_mut(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        conn.state = PluginState::Deactivating;

        if let Some(ref mut process) = conn.process {
            process.kill().await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to stop OpenVPN: {}", e)))?;
        }

        conn.process = None;
        conn.state = PluginState::Ready;
        conn.start_time = None;

        info!("OpenVPN connection {} deactivated", uuid);
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
                "config_file": {
                    "type": "string",
                    "description": "Path to OpenVPN configuration file"
                },
                "remote": {
                    "type": "string",
                    "description": "Remote server hostname/IP"
                },
                "port": {
                    "type": "integer",
                    "default": 1194,
                    "description": "Remote server port"
                },
                "proto": {
                    "type": "string",
                    "enum": ["udp", "tcp"],
                    "default": "udp",
                    "description": "Protocol"
                },
                "connection-type": {
                    "type": "string",
                    "enum": ["tun", "tap"],
                    "default": "tun",
                    "description": "Virtual device type"
                },
                "ca": {
                    "type": "string",
                    "description": "Path to CA certificate"
                },
                "cert": {
                    "type": "string",
                    "description": "Path to client certificate"
                },
                "key": {
                    "type": "string",
                    "description": "Path to client private key"
                },
                "username": {
                    "type": "string",
                    "description": "Username for authentication"
                },
                "password": {
                    "type": "string",
                    "description": "Password for authentication"
                }
            }
        })
    }
}
