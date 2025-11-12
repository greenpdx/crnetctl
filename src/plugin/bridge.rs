//! Bridge plugin implementation

use super::traits::*;
use crate::error::{NetctlError, NetctlResult};
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

/// Bridge plugin
pub struct BridgePlugin {
    metadata: PluginMetadata,
    state: PluginState,
    enabled: bool,
    bridges: RwLock<HashMap<String, BridgeInterface>>,
}

/// Bridge interface instance
struct BridgeInterface {
    uuid: String,
    config: ConnectionConfig,
    state: PluginState,
    bridge_name: String,
    member_interfaces: Vec<String>,
    stats: ConnectionStats,
    start_time: Option<std::time::Instant>,
}

impl BridgePlugin {
    /// Create a new Bridge plugin instance
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                id: "bridge".to_string(),
                name: "Bridge".to_string(),
                version: "1.0.0".to_string(),
                description: "Linux bridge support".to_string(),
                author: "netctl team".to_string(),
                capabilities: vec![PluginCapability::Virtual],
                dbus_service: Some("org.freedesktop.NetworkManager.bridge".to_string()),
                dbus_path: Some("/org/freedesktop/NetworkManager/bridge".to_string()),
            },
            state: PluginState::Uninitialized,
            enabled: false,
            bridges: RwLock::new(HashMap::new()),
        }
    }

    /// Validate bridge configuration
    fn validate_bridge_config(settings: &HashMap<String, serde_json::Value>) -> NetctlResult<(String, Vec<String>)> {
        let bridge_name = settings.get("bridge_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NetctlError::InvalidParameter("bridge_name is required".to_string()))?
            .to_string();

        let members = if let Some(members_val) = settings.get("members") {
            if let Some(arr) = members_val.as_array() {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        Ok((bridge_name, members))
    }

    /// Create bridge interface
    async fn create_bridge(&self, bridge_name: &str) -> NetctlResult<()> {
        let output = Command::new("ip")
            .args(&["link", "add", "name", bridge_name, "type", "bridge"])
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to create bridge: {}", e)))?;

        if !output.status.success() {
            return Err(NetctlError::ServiceError(
                format!("Failed to create bridge: {}", String::from_utf8_lossy(&output.stderr))
            ));
        }

        Ok(())
    }

    /// Delete bridge interface
    async fn delete_bridge(&self, bridge_name: &str) -> NetctlResult<()> {
        let output = Command::new("ip")
            .args(&["link", "delete", bridge_name, "type", "bridge"])
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to delete bridge: {}", e)))?;

        if !output.status.success() {
            warn!("Failed to delete bridge: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }

    /// Add interface to bridge
    async fn add_to_bridge(&self, bridge_name: &str, interface: &str) -> NetctlResult<()> {
        let output = Command::new("ip")
            .args(&["link", "set", "dev", interface, "master", bridge_name])
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to add interface to bridge: {}", e)))?;

        if !output.status.success() {
            return Err(NetctlError::ServiceError(
                format!("Failed to add {} to bridge: {}", interface, String::from_utf8_lossy(&output.stderr))
            ));
        }

        Ok(())
    }

    /// Remove interface from bridge
    async fn remove_from_bridge(&self, interface: &str) -> NetctlResult<()> {
        let output = Command::new("ip")
            .args(&["link", "set", "dev", interface, "nomaster"])
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to remove interface from bridge: {}", e)))?;

        if !output.status.success() {
            warn!("Failed to remove {} from bridge: {}", interface, String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }

    /// Configure bridge IP
    async fn configure_ip(&self, bridge: &BridgeInterface) -> NetctlResult<()> {
        if let Some(address) = bridge.config.settings.get("address") {
            if let Some(addr_str) = address.as_str() {
                let output = Command::new("ip")
                    .args(&["addr", "add", addr_str, "dev", &bridge.bridge_name])
                    .output()
                    .await
                    .map_err(|e| NetctlError::ServiceError(format!("Failed to set IP: {}", e)))?;

                if !output.status.success() {
                    return Err(NetctlError::ServiceError(
                        format!("Failed to set IP address: {}", String::from_utf8_lossy(&output.stderr))
                    ));
                }
            }
        }

        Ok(())
    }

    /// Configure bridge STP (Spanning Tree Protocol)
    async fn configure_stp(&self, bridge: &BridgeInterface) -> NetctlResult<()> {
        if let Some(stp) = bridge.config.settings.get("stp") {
            let stp_enabled = stp.as_bool().unwrap_or(false);
            let stp_value = if stp_enabled { "1" } else { "0" };

            let stp_path = format!("/sys/class/net/{}/bridge/stp_state", bridge.bridge_name);
            if let Err(e) = tokio::fs::write(&stp_path, stp_value).await {
                warn!("Failed to set STP state: {}", e);
            }
        }

        Ok(())
    }

    /// Bring bridge up
    async fn bring_up(&self, bridge_name: &str) -> NetctlResult<()> {
        let output = Command::new("ip")
            .args(&["link", "set", "dev", bridge_name, "up"])
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to bring up bridge: {}", e)))?;

        if !output.status.success() {
            return Err(NetctlError::ServiceError(
                format!("Failed to bring up bridge: {}", String::from_utf8_lossy(&output.stderr))
            ));
        }

        Ok(())
    }

    /// Bring bridge down
    async fn bring_down(&self, bridge_name: &str) -> NetctlResult<()> {
        let output = Command::new("ip")
            .args(&["link", "set", "dev", bridge_name, "down"])
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to bring down bridge: {}", e)))?;

        if !output.status.success() {
            warn!("Failed to bring down bridge: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }
}

#[async_trait]
impl NetworkPlugin for BridgePlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(&mut self) -> NetctlResult<()> {
        info!("Initializing Bridge plugin");
        self.state = PluginState::Initializing;

        // Check if ip command is available
        match Command::new("ip").arg("--version").output().await {
            Ok(_) => {
                info!("ip command available");
                self.state = PluginState::Ready;
                Ok(())
            }
            Err(e) => {
                Err(NetctlError::NotSupported(format!("ip command not found: {}", e)))
            }
        }
    }

    async fn shutdown(&mut self) -> NetctlResult<()> {
        info!("Shutting down Bridge plugin");

        // Delete all bridges
        let mut bridges = self.bridges.write().await;
        for (uuid, bridge) in bridges.iter() {
            info!("Deleting bridge: {}", uuid);

            // Remove member interfaces
            for member in &bridge.member_interfaces {
                let _ = self.remove_from_bridge(member).await;
            }

            // Delete bridge
            let _ = self.delete_bridge(&bridge.bridge_name).await;
        }
        bridges.clear();

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
        if config.conn_type != "bridge" {
            return Err(NetctlError::InvalidParameter(
                format!("Invalid connection type: {}", config.conn_type)
            ));
        }

        Self::validate_bridge_config(&config.settings)?;
        Ok(())
    }

    async fn create_connection(&mut self, config: ConnectionConfig) -> NetctlResult<String> {
        let uuid = config.uuid.clone();
        info!("Creating Bridge connection: {}", uuid);

        let (bridge_name, member_interfaces) = Self::validate_bridge_config(&config.settings)?;

        let bridge = BridgeInterface {
            uuid: uuid.clone(),
            config,
            state: PluginState::Ready,
            bridge_name,
            member_interfaces,
            stats: ConnectionStats {
                rx_bytes: 0,
                tx_bytes: 0,
                rx_packets: 0,
                tx_packets: 0,
                uptime: 0,
            },
            start_time: None,
        };

        let mut bridges = self.bridges.write().await;
        bridges.insert(uuid.clone(), bridge);

        Ok(uuid)
    }

    async fn delete_connection(&mut self, uuid: &str) -> NetctlResult<()> {
        info!("Deleting Bridge connection: {}", uuid);

        // Deactivate first if active
        if let Ok(state) = self.get_status(uuid).await {
            if state == PluginState::Active {
                self.deactivate(uuid).await?;
            }
        }

        let mut bridges = self.bridges.write().await;
        bridges.remove(uuid);

        Ok(())
    }

    async fn activate(&mut self, uuid: &str) -> NetctlResult<()> {
        info!("Activating Bridge connection: {}", uuid);

        let mut bridges = self.bridges.write().await;
        let bridge = bridges.get_mut(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        bridge.state = PluginState::Activating;

        // Create bridge interface
        self.create_bridge(&bridge.bridge_name).await?;

        // Add member interfaces to bridge
        for member in &bridge.member_interfaces {
            if let Err(e) = self.add_to_bridge(&bridge.bridge_name, member).await {
                warn!("Failed to add {} to bridge: {}", member, e);
            }
        }

        // Configure IP if specified
        self.configure_ip(bridge).await?;

        // Configure STP
        self.configure_stp(bridge).await?;

        // Bring bridge up
        self.bring_up(&bridge.bridge_name).await?;

        bridge.state = PluginState::Active;
        bridge.start_time = Some(std::time::Instant::now());

        info!("Bridge connection {} activated", uuid);
        Ok(())
    }

    async fn deactivate(&mut self, uuid: &str) -> NetctlResult<()> {
        info!("Deactivating Bridge connection: {}", uuid);

        let mut bridges = self.bridges.write().await;
        let bridge = bridges.get_mut(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        bridge.state = PluginState::Deactivating;

        // Bring bridge down
        self.bring_down(&bridge.bridge_name).await?;

        // Remove member interfaces from bridge
        for member in &bridge.member_interfaces {
            let _ = self.remove_from_bridge(member).await;
        }

        // Delete bridge interface
        self.delete_bridge(&bridge.bridge_name).await?;

        bridge.state = PluginState::Ready;
        bridge.start_time = None;

        info!("Bridge connection {} deactivated", uuid);
        Ok(())
    }

    async fn get_status(&self, uuid: &str) -> NetctlResult<PluginState> {
        let bridges = self.bridges.read().await;
        let bridge = bridges.get(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        Ok(bridge.state)
    }

    async fn get_stats(&self, uuid: &str) -> NetctlResult<ConnectionStats> {
        let bridges = self.bridges.read().await;
        let bridge = bridges.get(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        let mut stats = bridge.stats.clone();

        // Calculate uptime
        if let Some(start_time) = bridge.start_time {
            stats.uptime = start_time.elapsed().as_secs();
        }

        // TODO: Parse actual stats from /sys/class/net/<bridge>/statistics/

        Ok(stats)
    }

    async fn list_connections(&self) -> NetctlResult<Vec<ConnectionConfig>> {
        let bridges = self.bridges.read().await;
        Ok(bridges.values().map(|b| b.config.clone()).collect())
    }

    async fn update_connection(&mut self, uuid: &str, config: ConnectionConfig) -> NetctlResult<()> {
        self.validate_config(&config).await?;

        let mut bridges = self.bridges.write().await;
        let bridge = bridges.get_mut(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        bridge.config = config;
        Ok(())
    }

    fn settings_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "bridge_name": {
                    "type": "string",
                    "description": "Bridge interface name"
                },
                "members": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "Member interfaces to add to bridge"
                },
                "address": {
                    "type": "string",
                    "description": "IP address with prefix (e.g., 192.168.1.1/24)"
                },
                "stp": {
                    "type": "boolean",
                    "default": false,
                    "description": "Enable Spanning Tree Protocol"
                },
                "forward_delay": {
                    "type": "integer",
                    "description": "Forward delay in seconds"
                },
                "hello_time": {
                    "type": "integer",
                    "description": "Hello time in seconds"
                },
                "max_age": {
                    "type": "integer",
                    "description": "Maximum message age in seconds"
                },
                "ageing_time": {
                    "type": "integer",
                    "description": "MAC address ageing time in seconds"
                }
            },
            "required": ["bridge_name"]
        })
    }
}

impl Default for BridgePlugin {
    fn default() -> Self {
        Self::new()
    }
}
