//! netctl - Network Control Library
//!
//! Async network management library providing interfaces for:
//! - Network interface control
//! - WiFi management
//! - Access Point (hostapd)
//! - DHCP server (dora)
//! - DHCP testing and diagnostics (dhcpm)
//! - DNS configuration
//! - Routing
//! - VPN management (WireGuard, OpenVPN, IPsec)
//!
//! Includes NetworkManager D-Bus compatibility layer.

pub mod error;
pub mod validation;
pub mod interface;
pub mod wifi;
pub mod hostapd;
pub mod dhcp;
pub mod dhcpm;
pub mod routing;
pub mod plugin;
pub mod connection_config;
pub mod vpn;
pub mod network_monitor;

#[cfg(feature = "dbus-nm")]
pub mod dbus;

#[cfg(feature = "dbus-nm")]
pub mod dbus_integration;

// Re-export commonly used types
pub use error::{NetctlError, NetctlResult};
pub use interface::{InterfaceController, InterfaceInfo, IpAddress, InterfaceStats};
pub use wifi::{WifiController, WifiDeviceInfo, RegDomain, ScanResult};
pub use hostapd::{HostapdController, AccessPointConfig};
pub use dhcp::{DhcpController, DhcpConfig};
pub use dhcpm::{
    DhcpmController, DhcpTestConfig, DhcpTestResult, DhcpResponse,
    DhcpMessageType, DhcpOption,
};
pub use routing::RoutingController;
pub use plugin::{
    NetworkPlugin, PluginCapability, PluginMetadata, PluginState,
    ConnectionConfig, ConnectionStats, PluginManager, PluginLoader,
    PluginConfig, PluginConfigManager,
};
pub use vpn::{
    VpnBackend, VpnBackendFactory, VpnManager, VpnState, VpnStats,
};
pub use network_monitor::{NetworkMonitor, NetworkEvent};
