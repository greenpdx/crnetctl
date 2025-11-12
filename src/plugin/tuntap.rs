//! TUN/TAP device plugin implementation

use super::traits::*;
use crate::error::{NetctlError, NetctlResult};
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

/// TUN/TAP plugin
pub struct TunTapPlugin {
    metadata: PluginMetadata,
    state: PluginState,
    enabled: bool,
    devices: RwLock<HashMap<String, TunTapDevice>>,
}

/// TUN/TAP device instance
struct TunTapDevice {
    uuid: String,
    config: ConnectionConfig,
    state: PluginState,
    device_name: String,
    device_type: DeviceType,
    stats: ConnectionStats,
    start_time: Option<std::time::Instant>,
}

/// Device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeviceType {
    Tun,
    Tap,
}

impl TunTapPlugin {
    /// Create a new TUN/TAP plugin instance
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                id: "tuntap".to_string(),
                name: "TUN/TAP".to_string(),
                version: "1.0.0".to_string(),
                description: "TUN/TAP virtual network device support".to_string(),
                author: "netctl team".to_string(),
                capabilities: vec![PluginCapability::TunTap, PluginCapability::Virtual],
                dbus_service: Some("org.freedesktop.NetworkManager.tuntap".to_string()),
                dbus_path: Some("/org/freedesktop/NetworkManager/tuntap".to_string()),
            },
            state: PluginState::Uninitialized,
            enabled: false,
            devices: RwLock::new(HashMap::new()),
        }
    }

    /// Validate TUN/TAP configuration
    fn validate_tuntap_config(settings: &HashMap<String, serde_json::Value>) -> NetctlResult<DeviceType> {
        let device_type = settings.get("device_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NetctlError::InvalidParameter("device_type is required".to_string()))?;

        match device_type {
            "tun" => Ok(DeviceType::Tun),
            "tap" => Ok(DeviceType::Tap),
            _ => Err(NetctlError::InvalidParameter(
                format!("Invalid device_type: {} (must be 'tun' or 'tap')", device_type)
            )),
        }
    }

    /// Get device name for connection
    fn get_device_name(&self, uuid: &str, device_type: DeviceType) -> String {
        let prefix = match device_type {
            DeviceType::Tun => "tun",
            DeviceType::Tap => "tap",
        };
        format!("{}-{}", prefix, &uuid[..8])
    }

    /// Create TUN/TAP device
    async fn create_device(&self, device_name: &str, device_type: DeviceType) -> NetctlResult<()> {
        let type_str = match device_type {
            DeviceType::Tun => "tun",
            DeviceType::Tap => "tap",
        };

        let output = Command::new("ip")
            .args(&["tuntap", "add", "dev", device_name, "mode", type_str])
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to create device: {}", e)))?;

        if !output.status.success() {
            return Err(NetctlError::ServiceError(
                format!("Failed to create {}: {}", type_str, String::from_utf8_lossy(&output.stderr))
            ));
        }

        Ok(())
    }

    /// Delete TUN/TAP device
    async fn delete_device(&self, device_name: &str, device_type: DeviceType) -> NetctlResult<()> {
        let type_str = match device_type {
            DeviceType::Tun => "tun",
            DeviceType::Tap => "tap",
        };

        let output = Command::new("ip")
            .args(&["tuntap", "del", "dev", device_name, "mode", type_str])
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to delete device: {}", e)))?;

        if !output.status.success() {
            warn!("Failed to delete {}: {}", type_str, String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }

    /// Configure device IP address
    async fn configure_ip(&self, device: &TunTapDevice) -> NetctlResult<()> {
        if let Some(address) = device.config.settings.get("address") {
            if let Some(addr_str) = address.as_str() {
                let output = Command::new("ip")
                    .args(&["addr", "add", addr_str, "dev", &device.device_name])
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

    /// Bring device up
    async fn bring_up(&self, device_name: &str) -> NetctlResult<()> {
        let output = Command::new("ip")
            .args(&["link", "set", "dev", device_name, "up"])
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to bring up device: {}", e)))?;

        if !output.status.success() {
            return Err(NetctlError::ServiceError(
                format!("Failed to bring up device: {}", String::from_utf8_lossy(&output.stderr))
            ));
        }

        Ok(())
    }

    /// Bring device down
    async fn bring_down(&self, device_name: &str) -> NetctlResult<()> {
        let output = Command::new("ip")
            .args(&["link", "set", "dev", device_name, "down"])
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to bring down device: {}", e)))?;

        if !output.status.success() {
            warn!("Failed to bring down device: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }
}

#[async_trait]
impl NetworkPlugin for TunTapPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(&mut self) -> NetctlResult<()> {
        info!("Initializing TUN/TAP plugin");
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
        info!("Shutting down TUN/TAP plugin");

        // Delete all devices
        let mut devices = self.devices.write().await;
        for (uuid, device) in devices.iter() {
            info!("Deleting TUN/TAP device: {}", uuid);
            let _ = self.delete_device(&device.device_name, device.device_type).await;
        }
        devices.clear();

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
        if config.conn_type != "tun" && config.conn_type != "tap" && config.conn_type != "tuntap" {
            return Err(NetctlError::InvalidParameter(
                format!("Invalid connection type: {}", config.conn_type)
            ));
        }

        Self::validate_tuntap_config(&config.settings)?;
        Ok(())
    }

    async fn create_connection(&mut self, config: ConnectionConfig) -> NetctlResult<String> {
        let uuid = config.uuid.clone();
        info!("Creating TUN/TAP connection: {}", uuid);

        let device_type = Self::validate_tuntap_config(&config.settings)?;
        let device_name = self.get_device_name(&uuid, device_type);

        let device = TunTapDevice {
            uuid: uuid.clone(),
            config,
            state: PluginState::Ready,
            device_name,
            device_type,
            stats: ConnectionStats {
                rx_bytes: 0,
                tx_bytes: 0,
                rx_packets: 0,
                tx_packets: 0,
                uptime: 0,
            },
            start_time: None,
        };

        let mut devices = self.devices.write().await;
        devices.insert(uuid.clone(), device);

        Ok(uuid)
    }

    async fn delete_connection(&mut self, uuid: &str) -> NetctlResult<()> {
        info!("Deleting TUN/TAP connection: {}", uuid);

        // Deactivate first if active
        if let Ok(state) = self.get_status(uuid).await {
            if state == PluginState::Active {
                self.deactivate(uuid).await?;
            }
        }

        let mut devices = self.devices.write().await;
        devices.remove(uuid);

        Ok(())
    }

    async fn activate(&mut self, uuid: &str) -> NetctlResult<()> {
        info!("Activating TUN/TAP connection: {}", uuid);

        let mut devices = self.devices.write().await;
        let device = devices.get_mut(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        device.state = PluginState::Activating;

        // Create the device
        self.create_device(&device.device_name, device.device_type).await?;

        // Configure IP if specified
        self.configure_ip(device).await?;

        // Bring device up
        self.bring_up(&device.device_name).await?;

        device.state = PluginState::Active;
        device.start_time = Some(std::time::Instant::now());

        info!("TUN/TAP connection {} activated", uuid);
        Ok(())
    }

    async fn deactivate(&mut self, uuid: &str) -> NetctlResult<()> {
        info!("Deactivating TUN/TAP connection: {}", uuid);

        let mut devices = self.devices.write().await;
        let device = devices.get_mut(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        device.state = PluginState::Deactivating;

        // Bring device down
        self.bring_down(&device.device_name).await?;

        // Delete the device
        self.delete_device(&device.device_name, device.device_type).await?;

        device.state = PluginState::Ready;
        device.start_time = None;

        info!("TUN/TAP connection {} deactivated", uuid);
        Ok(())
    }

    async fn get_status(&self, uuid: &str) -> NetctlResult<PluginState> {
        let devices = self.devices.read().await;
        let device = devices.get(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        Ok(device.state)
    }

    async fn get_stats(&self, uuid: &str) -> NetctlResult<ConnectionStats> {
        let devices = self.devices.read().await;
        let device = devices.get(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        let mut stats = device.stats.clone();

        // Calculate uptime
        if let Some(start_time) = device.start_time {
            stats.uptime = start_time.elapsed().as_secs();
        }

        // TODO: Parse actual stats from /sys/class/net/<device>/statistics/

        Ok(stats)
    }

    async fn list_connections(&self) -> NetctlResult<Vec<ConnectionConfig>> {
        let devices = self.devices.read().await;
        Ok(devices.values().map(|d| d.config.clone()).collect())
    }

    async fn update_connection(&mut self, uuid: &str, config: ConnectionConfig) -> NetctlResult<()> {
        self.validate_config(&config).await?;

        let mut devices = self.devices.write().await;
        let device = devices.get_mut(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("Connection {} not found", uuid)))?;

        device.config = config;
        Ok(())
    }

    fn settings_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "device_type": {
                    "type": "string",
                    "enum": ["tun", "tap"],
                    "description": "Device type (tun or tap)"
                },
                "address": {
                    "type": "string",
                    "description": "IP address with prefix (e.g., 10.0.0.1/24)"
                },
                "owner": {
                    "type": "integer",
                    "description": "Owner UID"
                },
                "group": {
                    "type": "integer",
                    "description": "Group GID"
                },
                "multi_queue": {
                    "type": "boolean",
                    "default": false,
                    "description": "Enable multi-queue support"
                }
            },
            "required": ["device_type"]
        })
    }
}

impl Default for TunTapPlugin {
    fn default() -> Self {
        Self::new()
    }
}
