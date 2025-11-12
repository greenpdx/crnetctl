//! Plugin trait definitions

use crate::error::{NetctlError, NetctlResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Plugin capability flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginCapability {
    /// VPN connection support
    Vpn,
    /// TUN/TAP device creation
    TunTap,
    /// Virtual interface (bridge, bond, VLAN)
    Virtual,
    /// Wireless functionality
    Wireless,
    /// Mobile broadband (3G/4G/5G)
    MobileBroadband,
    /// Bluetooth networking
    Bluetooth,
    /// PPP/PPPoE
    Ppp,
    /// IPv6 support
    Ipv6,
    /// DHCP client/server
    Dhcp,
    /// DNS management
    Dns,
    /// Routing management
    Routing,
    /// Firewall integration
    Firewall,
}

/// Plugin state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginState {
    /// Plugin is not initialized
    Uninitialized,
    /// Plugin is initializing
    Initializing,
    /// Plugin is ready to use
    Ready,
    /// Plugin is activating a connection
    Activating,
    /// Plugin has an active connection
    Active,
    /// Plugin is deactivating
    Deactivating,
    /// Plugin has failed
    Failed,
    /// Plugin is disabled
    Disabled,
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin unique identifier
    pub id: String,
    /// Plugin display name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Plugin description
    pub description: String,
    /// Plugin author
    pub author: String,
    /// Plugin capabilities
    pub capabilities: Vec<PluginCapability>,
    /// D-Bus service name (optional)
    pub dbus_service: Option<String>,
    /// D-Bus object path (optional)
    pub dbus_path: Option<String>,
}

/// Connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Connection UUID
    pub uuid: String,
    /// Connection name
    pub name: String,
    /// Connection type (vpn, tun, bridge, etc.)
    pub conn_type: String,
    /// Plugin-specific settings
    pub settings: HashMap<String, serde_json::Value>,
    /// Auto-connect on startup
    pub autoconnect: bool,
}

/// Connection statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    /// Bytes received
    pub rx_bytes: u64,
    /// Bytes transmitted
    pub tx_bytes: u64,
    /// Packets received
    pub rx_packets: u64,
    /// Packets transmitted
    pub tx_packets: u64,
    /// Connection uptime in seconds
    pub uptime: u64,
}

/// Main plugin trait - all network plugins must implement this
#[async_trait]
pub trait NetworkPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Initialize the plugin
    async fn initialize(&mut self) -> NetctlResult<()>;

    /// Shutdown the plugin
    async fn shutdown(&mut self) -> NetctlResult<()>;

    /// Get current plugin state
    fn state(&self) -> PluginState;

    /// Check if plugin is enabled
    fn is_enabled(&self) -> bool;

    /// Enable the plugin
    async fn enable(&mut self) -> NetctlResult<()>;

    /// Disable the plugin
    async fn disable(&mut self) -> NetctlResult<()>;

    /// Validate connection configuration
    async fn validate_config(&self, config: &ConnectionConfig) -> NetctlResult<()>;

    /// Create a new connection
    async fn create_connection(&mut self, config: ConnectionConfig) -> NetctlResult<String>;

    /// Delete a connection
    async fn delete_connection(&mut self, uuid: &str) -> NetctlResult<()>;

    /// Activate a connection
    async fn activate(&mut self, uuid: &str) -> NetctlResult<()>;

    /// Deactivate a connection
    async fn deactivate(&mut self, uuid: &str) -> NetctlResult<()>;

    /// Get connection status
    async fn get_status(&self, uuid: &str) -> NetctlResult<PluginState>;

    /// Get connection statistics
    async fn get_stats(&self, uuid: &str) -> NetctlResult<ConnectionStats>;

    /// List all connections managed by this plugin
    async fn list_connections(&self) -> NetctlResult<Vec<ConnectionConfig>>;

    /// Update connection configuration
    async fn update_connection(&mut self, uuid: &str, config: ConnectionConfig) -> NetctlResult<()>;

    /// Get plugin-specific settings schema (JSON Schema)
    fn settings_schema(&self) -> serde_json::Value;

    /// Handle plugin-specific D-Bus method calls
    #[cfg(feature = "dbus-nm")]
    async fn handle_dbus_method(
        &mut self,
        method: &str,
        _params: HashMap<String, serde_json::Value>,
    ) -> NetctlResult<serde_json::Value> {
        Err(NetctlError::NotSupported(format!("D-Bus method '{}' not supported", method)))
    }

    /// Get plugin-specific D-Bus properties
    #[cfg(feature = "dbus-nm")]
    async fn dbus_properties(&self) -> NetctlResult<HashMap<String, serde_json::Value>> {
        Ok(HashMap::new())
    }
}

/// Plugin factory for creating plugin instances
pub type PluginFactory = Box<dyn Fn() -> Box<dyn NetworkPlugin> + Send + Sync>;
