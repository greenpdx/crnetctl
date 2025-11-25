//! CR D-Bus Interface Library
//!
//! This library provides a D-Bus interface for controlling network operations
//! through the CR router. It offers a NetworkManager-like API but with CR branding
//! and naming conventions.
//!
//! # D-Bus Service
//!
//! The CR D-Bus service is available at:
//! - **Service Name**: `org.crrouter.NetworkControl`
//! - **Main Object Path**: `/org/crrouter/NetworkControl`
//!
//! # Interfaces
//!
//! ## Network Control Interface
//! - **Interface Name**: `org.crrouter.NetworkControl`
//! - **Object Path**: `/org/crrouter/NetworkControl`
//! - **Purpose**: Main network control and device management
//!
//! ## WiFi Interface
//! - **Interface Name**: `org.crrouter.NetworkControl.WiFi`
//! - **Object Path**: `/org/crrouter/NetworkControl/WiFi`
//! - **Purpose**: WiFi scanning, connection, and access point management
//!
//! ## VPN Interface
//! - **Interface Name**: `org.crrouter.NetworkControl.VPN`
//! - **Object Path**: `/org/crrouter/NetworkControl/VPN`
//! - **Purpose**: VPN connection management (OpenVPN, WireGuard, IPsec, Arti/Tor)
//!
//! ## Device Interface
//! - **Interface Name**: `org.crrouter.NetworkControl.Device`
//! - **Object Paths**: `/org/crrouter/NetworkControl/Devices/{interface_name}`
//! - **Purpose**: Individual device control and monitoring
//!
//! # Quick Start
//!
//! ## Starting the Service
//!
//! ```rust,no_run
//! use netctl::cr_dbus::CRDbusService;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Start the CR D-Bus service
//!     let service = CRDbusService::start().await?;
//!
//!     // Discover network devices
//!     service.discover_devices().await?;
//!
//!     // Keep service running
//!     while service.is_running().await {
//!         tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Using the D-Bus Interface from Command Line
//!
//! ```bash
//! # List all network devices
//! dbus-send --system --print-reply \
//!   --dest=org.crrouter.NetworkControl \
//!   /org/crrouter/NetworkControl \
//!   org.crrouter.NetworkControl.GetDevices
//!
//! # Get network state
//! dbus-send --system --print-reply \
//!   --dest=org.crrouter.NetworkControl \
//!   /org/crrouter/NetworkControl \
//!   org.crrouter.NetworkControl.GetState
//!
//! # Scan for WiFi networks
//! dbus-send --system --print-reply \
//!   --dest=org.crrouter.NetworkControl \
//!   /org/crrouter/NetworkControl/WiFi \
//!   org.crrouter.NetworkControl.WiFi.Scan
//!
//! # Get WiFi access points
//! dbus-send --system --print-reply \
//!   --dest=org.crrouter.NetworkControl \
//!   /org/crrouter/NetworkControl/WiFi \
//!   org.crrouter.NetworkControl.WiFi.GetAccessPoints
//! ```
//!
//! ## Using from Python with dbus-python
//!
//! ```python
//! import dbus
//!
//! # Connect to system bus
//! bus = dbus.SystemBus()
//!
//! # Get network control interface
//! proxy = bus.get_object('org.crrouter.NetworkControl',
//!                        '/org/crrouter/NetworkControl')
//! network_control = dbus.Interface(proxy, 'org.crrouter.NetworkControl')
//!
//! # List devices
//! devices = network_control.GetDevices()
//! print(f"Found {len(devices)} devices")
//!
//! # Get network state
//! state = network_control.GetState()
//! print(f"Network state: {state}")
//!
//! # WiFi operations
//! wifi_proxy = bus.get_object('org.crrouter.NetworkControl',
//!                             '/org/crrouter/NetworkControl/WiFi')
//! wifi = dbus.Interface(wifi_proxy, 'org.crrouter.NetworkControl.WiFi')
//!
//! # Scan for networks
//! wifi.Scan()
//!
//! # Get access points
//! access_points = wifi.GetAccessPoints()
//! for ap in access_points:
//!     print(f"SSID: {ap['SSID']}, Strength: {ap['Strength']}%")
//! ```
//!
//! # Signal Monitoring
//!
//! The CR D-Bus service emits various signals for monitoring network state changes:
//!
//! - `StateChanged` - Global network state changes
//! - `DeviceAdded` - New device detected
//! - `DeviceRemoved` - Device removed
//! - `DeviceStateChanged` - Device state changed
//! - `ConnectivityChanged` - Internet connectivity changed
//! - WiFi signals: `ScanCompleted`, `Connected`, `Disconnected`
//! - VPN signals: `ConnectionAdded`, `StateChanged`, `Connected`, `Disconnected`
//!
//! ## Monitoring Signals with dbus-monitor
//!
//! ```bash
//! # Monitor all CR D-Bus signals
//! dbus-monitor --system "type='signal',sender='org.crrouter.NetworkControl'"
//! ```
//!
//! # Types and Enumerations
//!
//! All types use the `CR` prefix to distinguish them from NetworkManager types:
//!
//! - `CRNetworkState` - Global network state
//! - `CRConnectivity` - Connectivity level (None, Limited, Portal, Full)
//! - `CRDeviceType` - Device type (Ethernet, WiFi, VPN, etc.)
//! - `CRDeviceState` - Device state (Disconnected, Connecting, Activated, etc.)
//! - `CRWiFiSecurity` - WiFi security type (None, WEP, WPA, WPA2, WPA3, Enterprise)
//! - `CRWiFiMode` - WiFi mode (Infrastructure, AccessPoint, AdHoc)
//! - `CRVpnType` - VPN protocol (OpenVPN, WireGuard, IPsec, Arti)
//! - `CRVpnState` - VPN connection state
//!
//! # Integration with Netctl
//!
//! The CR D-Bus library integrates directly with the netctl application to provide
//! D-Bus control over network operations. All D-Bus method calls are translated
//! into netctl operations, ensuring consistency across CLI and D-Bus interfaces.

// Module declarations
pub mod types;
pub mod network_control;
pub mod device;
pub mod wifi;
pub mod vpn;
pub mod connection;
pub mod dhcp;
pub mod dns;
pub mod routing;
pub mod privilege;
pub mod integration;

// Re-exports for convenience
pub use types::*;
pub use network_control::CRNetworkControl;
pub use device::CRDevice;
pub use wifi::CRWiFi;
pub use vpn::CRVPN;
pub use connection::CRConnection;
pub use dhcp::CRDhcp;
pub use dns::CRDns;
pub use routing::CRRouting;
pub use privilege::CRPrivilege;
pub use integration::CRDbusService;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_constants() {
        assert_eq!(CR_DBUS_SERVICE, "org.crrouter.NetworkControl");
        assert_eq!(CR_DBUS_PATH, "/org/crrouter/NetworkControl");
        assert_eq!(CR_WIFI_PATH, "/org/crrouter/NetworkControl/WiFi");
    }

    #[test]
    fn test_device_type_conversion() {
        let dt = CRDeviceType::WiFi;
        let val: u32 = dt.into();
        assert_eq!(val, 2);
    }

    #[test]
    fn test_device_state_conversion() {
        let ds = CRDeviceState::Activated;
        let val: u32 = ds.into();
        assert_eq!(val, 100);
    }

    #[test]
    fn test_network_state_conversion() {
        let ns = CRNetworkState::ConnectedGlobal;
        let val: u32 = ns.into();
        assert_eq!(val, 60);
    }

    #[test]
    fn test_connectivity_conversion() {
        let c = CRConnectivity::Full;
        let val: u32 = c.into();
        assert_eq!(val, 4);
    }

    #[test]
    fn test_device_info_creation() {
        let info = CRDeviceInfo::new("eth0".to_string(), CRDeviceType::Ethernet);
        assert_eq!(info.interface, "eth0");
        assert_eq!(info.device_type, CRDeviceType::Ethernet);
        assert_eq!(info.state, CRDeviceState::Disconnected);
        assert!(info.path.contains("eth0"));
    }

    #[test]
    fn test_vpn_info_creation() {
        let info = CRVpnInfo::new("my_vpn".to_string(), CRVpnType::WireGuard);
        assert_eq!(info.name, "my_vpn");
        assert_eq!(info.vpn_type, CRVpnType::WireGuard);
        assert_eq!(info.state, CRVpnState::Disconnected);
        assert!(info.path.contains("my_vpn"));
    }
}
