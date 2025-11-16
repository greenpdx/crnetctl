use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::collections::HashMap;
use crate::plugin::ConnectionConfig;
use crate::error::NetctlResult;

/// Statistics for a VPN connection
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct VpnStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub connected_since: Option<std::time::SystemTime>,
    pub last_handshake: Option<std::time::SystemTime>,
    pub peer_endpoint: Option<String>,
}

/// Connection state for VPN connections
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VpnState {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
    Failed(String),
}

/// Common interface that all VPN backend drivers must implement
/// This provides a unified API for managing VPN connections across
/// different technologies (WireGuard, OpenVPN, IPsec, etc.)
#[async_trait]
pub trait VpnBackend: Send + Sync {
    /// Get the name of this VPN backend (e.g., "wireguard", "openvpn", "ipsec")
    fn name(&self) -> &str;

    /// Get the version of the underlying VPN software
    async fn version(&self) -> NetctlResult<String>;

    /// Check if the VPN software is installed and available
    async fn is_available(&self) -> bool;

    /// Validate the configuration for this VPN type
    /// Returns Ok(()) if valid, Err if invalid with explanation
    async fn validate_config(&self, config: &ConnectionConfig) -> NetctlResult<()>;

    /// Connect to the VPN using the provided configuration
    /// Returns the interface name created (e.g., "wg0", "tun0")
    async fn connect(&mut self, config: &ConnectionConfig) -> NetctlResult<String>;

    /// Disconnect from the VPN
    async fn disconnect(&mut self) -> NetctlResult<()>;

    /// Get the current connection state
    async fn state(&self) -> VpnState;

    /// Get connection statistics
    async fn stats(&self) -> NetctlResult<VpnStats>;

    /// Get the interface name for this connection (if connected)
    fn interface_name(&self) -> Option<String>;

    /// Get backend-specific status information as JSON
    async fn status_json(&self) -> NetctlResult<Value>;

    /// Import a configuration file in the native format for this VPN type
    /// (e.g., .conf for WireGuard, .ovpn for OpenVPN)
    async fn import_config(&self, path: &std::path::Path) -> NetctlResult<HashMap<String, Value>>;

    /// Export the current configuration to a native format file
    async fn export_config(&self, config: &ConnectionConfig, path: &std::path::Path) -> NetctlResult<()>;
}

/// Factory function type for creating VPN backends
pub type VpnBackendFactory = fn() -> Box<dyn VpnBackend>;
