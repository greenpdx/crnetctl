//! VLAN plugin implementation

use super::traits::*;
use crate::error::{NetctlError, NetctlResult};
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

/// VLAN plugin
pub struct VlanPlugin {
    metadata: PluginMetadata,
    state: PluginState,
    enabled: bool,
    vlans: RwLock<HashMap<String, VlanInterface>>,
}

/// VLAN interface instance
struct VlanInterface {
    uuid: String,
    config: ConnectionConfig,
    state: PluginState,
    interface_name: String,
    parent_interface: String,
    vlan_id: u16,
    stats: ConnectionStats,
    start_time: Option<std::time::Instant>,
}

impl VlanPlugin {
    /// Create a new VLAN plugin instance
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                id: "vlan".to_string(),
                name: "VLAN".to_string(),
                version: "1.0.0".to_string(),
                description: "802.1Q VLAN support".to_string(),
                author: "netctl team".to_string(),
                capabilities: vec![PluginCapability::Virtual],
                dbus_service: Some("org.freedesktop.NetworkManager.vlan".to_string()),
                dbus_path: Some("/org/freedesktop/NetworkManager/vlan".to_string()),
            },
            state: PluginState::Uninitialized,
            enabled: false,
            vlans: RwLock::new(HashMap::new()),
        }
    }

    /// Validate VLAN configuration
    fn validate_vlan_config(settings: &HashMap<String, serde_json::Value>) -> NetctlResult<(String, u16)> {
        let parent = settings.get("parent")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NetctlError::InvalidParameter("parent interface is required".to_string()))?
            .to_string();

        let vlan_id = settings.get("vlan_id")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| NetctlError::InvalidParameter("vlan_id is required".to_string()))?;

        if vlan_id > 4094 {
            return Err(NetctlError::InvalidParameter(
                "vlan_id must be between 0 and 4094".to_string()
            ));
        }

        Ok((parent, vlan_id as u16))
    }

    /// Get VLAN interface name
    fn get_vlan_name(&self, parent: &str, vlan_id: u16) -> String {
        format!("{}.{}", parent, vlan_id)
    }

    /// Create VLAN interface
    async fn create_vlan(&self, vlan: &VlanInterface) -> NetctlResult<()> {
        let output = Command::new("ip")
            .args(&[
                "link", "add", "link", &vlan.parent_interface,
                "name", &vlan.interface_name,
                "type", "vlan", "id", &vlan.vlan_id.to_string()
            ])
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to create VLAN: {}", e)))?;

        if !output.status.success() {
            return Err(NetctlError::ServiceError(
                format!("Failed to create VLAN: {}", String::from_utf8_lossy(&output.stderr))
            ));
        }

        Ok(())
    }

    /// Delete VLAN interface
    async fn delete_vlan(&self, interface_name: &str) -> NetctlResult<()> {
        let output = Command::new("ip")
            .args(&["link", "delete", interface_name])
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to delete VLAN: {}", e)))?;

        if !output.status.success() {
            warn!("Failed to delete VLAN: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }

    /// Configure VLAN interface IP
    async fn configure_ip(&self, vlan: &VlanInterface) -> NetctlResult<()> {
        if let Some(address) = vlan.config.settings.get("address") {
            if let Some(addr_str) = address.as_str() {
                let output = Command::new("ip")
                    .args(&["addr", "add", addr_str, "dev", &vlan.interface_name])
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

    /// Bring VLAN interface up
    async fn bring_up(&self, interface_name: &str) -> NetctlResult<()> {
        let output = Command::new("ip")
            .args(&["link", "set", "dev", interface_name, "up"])
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to bring up interface: {}", e)))?;

        if !output.status.success() {
            return Err(NetctlError::ServiceError(
                format!("Failed to bring up interface: {}", String::from_utf8_lossy(&output.stderr))
            ));
        }

        Ok(())
    }

    /// Bring VLAN interface down
    async fn bring_down(&self, interface_name: &str) -> NetctlResult<()> {
        let output = Command::new("ip")
            .args(&["link", "set", "dev", interface_name, "down"])
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to bring down interface: {}", e)))?;

        if !output.status.success() {
            warn!("Failed to bring down interface: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }
}

#[async_trait]
impl NetworkPlugin for VlanPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(&mut self) -> NetctlResult<()> {
        info!("Initializing VLAN plugin");
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
        info!("Shutting down VLAN plugin");

        // Delete all VLAN interfaces
        let mut vlans = self.vlans.write().await;
        for (uuid, vlan) in vlans.iter() {
            info!("Deleting VLAN interface: {}", uuid);
            let _ = self.delete_vlan(&vlan.interface_name).await;
        }
        vlans.clear();

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
        if config.conn_type != "vlan" {
            return Err(NetctlError::InvalidParameter(
                format!("Invalid connection type: {}", config.conn_type)
            ));
        }

        Self::validate_vlan_config(&config.settings)?;
        Ok(())
    }

    async fn create_connection(&mut self, config: ConnectionConfig) -> NetctlResult<String> {
        let uuid = config.uuid.clone();
        info!("Creating VLAN connection: {}", uuid);

        let (parent_interface, vlan_id) = Self::validate_vlan_config(&config.settings)?;
        let interface_name = self.get_vlan_name(&parent_interface, vlan_id);

        let vlan = VlanInterface {
            uuid: uuid.clone(),
            config,
            state: PluginState::Ready,
            interface_name,
            parent_interface,
            vlan_id,
            stats: ConnectionStats {
                rx_bytes: 0,
                tx_bytes: 0,
                rx_packets: 0,
                tx_packets: 0,
                uptime: 0,
            },
            start_time: None,
        };

        let mut vlans = self.vlans.write().await;
        vlans.insert(uuid.clone(), vlan);

        Ok(uuid)
    }

    async fn delete_connection(&mut self, uuid: &str) -> NetctlResult<()> {
        info!("Deleting VLAN connection: {}", uuid);

        // Deactivate first if active
        if let Ok(state) = self.get_status(uuid).await {
            if state == PluginState::Active {
                self.deactivate(uuid).await?;
            }
        }

        let mut vlans = self.vlans.write().await;
        vlans.remove(uuid);

        Ok(())
    }

    async fn activate(&mut self, uuid: &str) -> NetctlResult<()> {
        info!("Activating VLAN connection: {}", uuid);

        let mut vlans = self.vlans.write().await;
        let vlan = vlans.get_mut(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        vlan.state = PluginState::Activating;

        // Create VLAN interface
        self.create_vlan(vlan).await?;

        // Configure IP if specified
        self.configure_ip(vlan).await?;

        // Bring interface up
        self.bring_up(&vlan.interface_name).await?;

        vlan.state = PluginState::Active;
        vlan.start_time = Some(std::time::Instant::now());

        info!("VLAN connection {} activated", uuid);
        Ok(())
    }

    async fn deactivate(&mut self, uuid: &str) -> NetctlResult<()> {
        info!("Deactivating VLAN connection: {}", uuid);

        let mut vlans = self.vlans.write().await;
        let vlan = vlans.get_mut(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        vlan.state = PluginState::Deactivating;

        // Bring interface down
        self.bring_down(&vlan.interface_name).await?;

        // Delete VLAN interface
        self.delete_vlan(&vlan.interface_name).await?;

        vlan.state = PluginState::Ready;
        vlan.start_time = None;

        info!("VLAN connection {} deactivated", uuid);
        Ok(())
    }

    async fn get_status(&self, uuid: &str) -> NetctlResult<PluginState> {
        let vlans = self.vlans.read().await;
        let vlan = vlans.get(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        Ok(vlan.state)
    }

    async fn get_stats(&self, uuid: &str) -> NetctlResult<ConnectionStats> {
        let vlans = self.vlans.read().await;
        let vlan = vlans.get(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        let mut stats = vlan.stats.clone();

        // Calculate uptime
        if let Some(start_time) = vlan.start_time {
            stats.uptime = start_time.elapsed().as_secs();
        }

        // TODO: Parse actual stats from /sys/class/net/<interface>/statistics/

        Ok(stats)
    }

    async fn list_connections(&self) -> NetctlResult<Vec<ConnectionConfig>> {
        let vlans = self.vlans.read().await;
        Ok(vlans.values().map(|v| v.config.clone()).collect())
    }

    async fn update_connection(&mut self, uuid: &str, config: ConnectionConfig) -> NetctlResult<()> {
        self.validate_config(&config).await?;

        let mut vlans = self.vlans.write().await;
        let vlan = vlans.get_mut(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        vlan.config = config;
        Ok(())
    }

    fn settings_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "parent": {
                    "type": "string",
                    "description": "Parent interface name"
                },
                "vlan_id": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 4094,
                    "description": "VLAN ID (0-4094)"
                },
                "address": {
                    "type": "string",
                    "description": "IP address with prefix (e.g., 192.168.1.1/24)"
                },
                "mtu": {
                    "type": "integer",
                    "description": "MTU size"
                }
            },
            "required": ["parent", "vlan_id"]
        })
    }
}

impl Default for VlanPlugin {
    fn default() -> Self {
        Self::new()
    }
}
