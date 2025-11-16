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
//! Includes NetworkManager D-Bus compatibility layer and CR D-Bus interface.

pub mod error;
pub mod validation;
pub mod interface;
pub mod wifi;
pub mod hostapd;
pub mod dhcp;

#[cfg(feature = "dhcp-testing")]
pub mod dhcpm;

pub mod routing;
pub mod device;
pub mod plugin;
pub mod connection_config;
pub mod vpn;
pub mod network_monitor;
pub mod libnm_compat;
pub mod cr_dbus;

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

#[cfg(feature = "dhcp-testing")]
pub use dhcpm::{
    DhcpmController, DhcpTestConfig, DhcpTestResult, DhcpResponse,
    DhcpMessageType, DhcpOption,
};

pub use routing::RoutingController;
pub use device::{
    DeviceController, Device, DeviceType, DeviceState, DeviceCapabilities,
    DeviceStats, DeviceConfig,
};
pub use plugin::{
    NetworkPlugin, PluginCapability, PluginMetadata, PluginState,
    ConnectionConfig, ConnectionStats, PluginManager,
    PluginConfig, PluginConfigManager,
};

#[cfg(feature = "plugins")]
pub use plugin::PluginLoader;
pub use vpn::{
    VpnBackend, VpnBackendFactory, VpnManager, VpnState, VpnStats,
};
pub use network_monitor::{NetworkMonitor, NetworkEvent};

// libnm-compatible API (CR prefix)
pub use libnm_compat::{
    CRClient, CRDevice, CRDeviceType, CRDeviceState, CRDeviceCapabilities,
    CRConnection, CRRemoteConnection, CRActiveConnection, CRAccessPoint,
    CRIPConfig, CRIPAddress, CRIPRoute,
    CRSetting, CRSettingConnection, CRSettingWired, CRSettingWireless,
    CRSettingIP4Config, CRSettingIP6Config,
    CRState, CRConnectivityState, CRActiveConnectionState,
};

// CR D-Bus interface
pub use cr_dbus::{
    CRDbusService, CRNetworkControl, CRWiFi, CRVPN,
    CRNetworkState, CRConnectivity, CRDeviceInfo,
    CRAccessPointInfo, CRVpnInfo, CRWiFiSecurity, CRWiFiMode, CRVpnType, CRVpnState,
    CR_DBUS_SERVICE, CR_DBUS_PATH, CR_WIFI_PATH,
};
