//! netctl - Network Control Library
//!
//! Async network management library providing interfaces for:
//! - Network interface control
//! - WiFi management
//! - Access Point (hostapd)
//! - DHCP server (dora)
//! - DNS configuration
//! - Routing
//!
//! Includes NetworkManager D-Bus compatibility layer.

pub mod error;
pub mod interface;
pub mod wifi;
pub mod hostapd;
pub mod dhcp;
pub mod routing;
pub mod plugin;
pub mod connection_config;

#[cfg(feature = "dbus-nm")]
pub mod dbus;

// Re-export commonly used types
pub use error::{NetctlError, NetctlResult};
pub use interface::{InterfaceController, InterfaceInfo, IpAddress, InterfaceStats};
pub use wifi::{WifiController, WifiDeviceInfo, RegDomain, ScanResult};
pub use hostapd::{HostapdController, AccessPointConfig};
pub use dhcp::{DhcpController, DhcpConfig};
pub use routing::RoutingController;
pub use plugin::{
    NetworkPlugin, PluginCapability, PluginMetadata, PluginState,
    ConnectionConfig, ConnectionStats, PluginManager, PluginLoader,
    PluginConfig, PluginConfigManager,
};
