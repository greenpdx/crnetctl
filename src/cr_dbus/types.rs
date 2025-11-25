//! CR D-Bus interface types
//!
//! Common types and enums used across the CR D-Bus interface

use serde::{Deserialize, Serialize};
use zbus::zvariant::Type;

/// CR D-Bus service name
pub const CR_DBUS_SERVICE: &str = "org.crrouter.NetworkControl";

/// CR D-Bus main object path
pub const CR_DBUS_PATH: &str = "/org/crrouter/NetworkControl";

/// CR D-Bus device path prefix
pub const CR_DEVICE_PATH_PREFIX: &str = "/org/crrouter/NetworkControl/Devices";

/// CR D-Bus WiFi path
pub const CR_WIFI_PATH: &str = "/org/crrouter/NetworkControl/WiFi";

/// CR D-Bus VPN path prefix
pub const CR_VPN_PATH_PREFIX: &str = "/org/crrouter/NetworkControl/VPN";

/// CR D-Bus Connection path
pub const CR_CONNECTION_PATH: &str = "/org/crrouter/NetworkControl/Connection";

/// CR D-Bus Connection path prefix
pub const CR_CONNECTION_PATH_PREFIX: &str = "/org/crrouter/NetworkControl/Connections";

/// CR D-Bus DHCP path
pub const CR_DHCP_PATH: &str = "/org/crrouter/NetworkControl/DHCP";

/// CR D-Bus DNS path
pub const CR_DNS_PATH: &str = "/org/crrouter/NetworkControl/DNS";

/// CR D-Bus Routing path
pub const CR_ROUTING_PATH: &str = "/org/crrouter/NetworkControl/Routing";

/// Network control state
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CRNetworkState {
    /// Network is unknown
    Unknown = 0,
    /// Network is initializing
    Initializing = 10,
    /// Network is disconnected
    Disconnected = 20,
    /// Network is connecting
    Connecting = 30,
    /// Network is connected locally
    ConnectedLocal = 40,
    /// Network is connected to site
    ConnectedSite = 50,
    /// Network is fully connected (internet access)
    ConnectedGlobal = 60,
}

/// Connectivity state
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CRConnectivity {
    /// Connectivity is unknown
    Unknown = 0,
    /// No connectivity
    None = 1,
    /// Limited connectivity (local network only)
    Limited = 2,
    /// Portal detected (captive portal)
    Portal = 3,
    /// Full connectivity (internet access)
    Full = 4,
}

/// Device type enumeration
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CRDeviceType {
    /// Unknown device type
    Unknown = 0,
    /// Ethernet device
    Ethernet = 1,
    /// WiFi device
    WiFi = 2,
    /// Bluetooth device
    Bluetooth = 3,
    /// Bridge device
    Bridge = 4,
    /// VLAN device
    Vlan = 5,
    /// TUN/TAP device
    TunTap = 6,
    /// VPN device
    Vpn = 7,
    /// Loopback device
    Loopback = 8,
}

/// Device state enumeration
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CRDeviceState {
    /// State is unknown
    Unknown = 0,
    /// Device is unmanaged
    Unmanaged = 10,
    /// Device is unavailable
    Unavailable = 20,
    /// Device is disconnected
    Disconnected = 30,
    /// Device is preparing to connect
    Preparing = 40,
    /// Device is being configured
    Configuring = 50,
    /// Device needs authentication
    NeedAuth = 60,
    /// Device IP configuration in progress
    IpConfig = 70,
    /// Device IP connectivity check in progress
    IpCheck = 80,
    /// Device is waiting for secondaries
    Secondaries = 90,
    /// Device is activated
    Activated = 100,
    /// Device is deactivating
    Deactivating = 110,
    /// Device has failed
    Failed = 120,
}

/// WiFi security type
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CRWiFiSecurity {
    /// No security (open network)
    None = 0,
    /// WEP security
    Wep = 1,
    /// WPA security
    Wpa = 2,
    /// WPA2 security
    Wpa2 = 3,
    /// WPA3 security
    Wpa3 = 4,
    /// Enterprise security
    Enterprise = 5,
}

/// WiFi mode
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CRWiFiMode {
    /// Unknown mode
    Unknown = 0,
    /// Infrastructure mode (client)
    Infrastructure = 1,
    /// Access Point mode
    AccessPoint = 2,
    /// Ad-hoc mode
    AdHoc = 3,
}

/// VPN protocol type
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CRVpnType {
    /// Unknown VPN type
    Unknown = 0,
    /// OpenVPN
    OpenVpn = 1,
    /// WireGuard
    WireGuard = 2,
    /// IPsec
    IPsec = 3,
    /// Arti/Tor
    Arti = 4,
}

/// VPN state
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CRVpnState {
    /// VPN is unknown
    Unknown = 0,
    /// VPN is disconnected
    Disconnected = 1,
    /// VPN is connecting
    Connecting = 2,
    /// VPN is connected
    Connected = 3,
    /// VPN is disconnecting
    Disconnecting = 4,
    /// VPN has failed
    Failed = 5,
}

/// Device information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRDeviceInfo {
    /// Device object path
    pub path: String,
    /// Interface name
    pub interface: String,
    /// Device type
    pub device_type: CRDeviceType,
    /// Device state
    pub state: CRDeviceState,
    /// IPv4 address
    pub ipv4_address: Option<String>,
    /// IPv6 address
    pub ipv6_address: Option<String>,
    /// MAC address
    pub hw_address: Option<String>,
    /// MTU
    pub mtu: u32,
}

impl CRDeviceInfo {
    /// Create a new device info with default values
    pub fn new(interface: String, device_type: CRDeviceType) -> Self {
        let path = format!("{}/{}", CR_DEVICE_PATH_PREFIX, interface);
        Self {
            path,
            interface,
            device_type,
            state: CRDeviceState::Disconnected,
            ipv4_address: None,
            ipv6_address: None,
            hw_address: None,
            mtu: 1500,
        }
    }
}

/// WiFi access point information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRAccessPointInfo {
    /// SSID (network name)
    pub ssid: String,
    /// BSSID (MAC address)
    pub bssid: String,
    /// Signal strength (0-100)
    pub strength: u8,
    /// Security type
    pub security: CRWiFiSecurity,
    /// Frequency in MHz
    pub frequency: u32,
    /// WiFi mode
    pub mode: CRWiFiMode,
}

/// VPN connection information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRVpnInfo {
    /// VPN connection name
    pub name: String,
    /// VPN object path
    pub path: String,
    /// VPN protocol type
    pub vpn_type: CRVpnType,
    /// VPN state
    pub state: CRVpnState,
    /// Local IP address (when connected)
    pub local_ip: Option<String>,
    /// Remote server address
    pub remote_address: Option<String>,
}

impl CRVpnInfo {
    /// Create a new VPN info
    pub fn new(name: String, vpn_type: CRVpnType) -> Self {
        let path = format!("{}/{}", CR_VPN_PATH_PREFIX, name);
        Self {
            name,
            path,
            vpn_type,
            state: CRVpnState::Disconnected,
            local_ip: None,
            remote_address: None,
        }
    }
}

/// Helper function to convert device type to u32
impl From<CRDeviceType> for u32 {
    fn from(dt: CRDeviceType) -> u32 {
        dt as u32
    }
}

/// Helper function to convert device state to u32
impl From<CRDeviceState> for u32 {
    fn from(ds: CRDeviceState) -> u32 {
        ds as u32
    }
}

/// Helper function to convert network state to u32
impl From<CRNetworkState> for u32 {
    fn from(ns: CRNetworkState) -> u32 {
        ns as u32
    }
}

/// Helper function to convert connectivity to u32
impl From<CRConnectivity> for u32 {
    fn from(c: CRConnectivity) -> u32 {
        c as u32
    }
}

/// Connection type enumeration
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CRConnectionType {
    /// Unknown connection type
    Unknown = 0,
    /// Ethernet connection
    Ethernet = 1,
    /// WiFi connection
    WiFi = 2,
    /// VPN connection
    Vpn = 3,
    /// Bridge connection
    Bridge = 4,
    /// Bond connection
    Bond = 5,
    /// VLAN connection
    Vlan = 6,
    /// Loopback connection
    Loopback = 7,
}

/// Connection state enumeration
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CRConnectionState {
    /// Connection state unknown
    Unknown = 0,
    /// Connection is activating
    Activating = 1,
    /// Connection is activated
    Activated = 2,
    /// Connection is deactivating
    Deactivating = 3,
    /// Connection is deactivated
    Deactivated = 4,
}

/// Connection information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRConnectionInfo {
    /// Connection UUID
    pub uuid: String,
    /// Connection ID (name)
    pub id: String,
    /// Connection type
    pub conn_type: CRConnectionType,
    /// Connection state
    pub state: CRConnectionState,
    /// Device path (if active)
    pub device: Option<String>,
    /// Object path
    pub path: String,
    /// Autoconnect enabled
    pub autoconnect: bool,
}

impl CRConnectionInfo {
    /// Create a new connection info
    pub fn new(uuid: String, id: String, conn_type: CRConnectionType) -> Self {
        let path = format!("{}/{}", CR_CONNECTION_PATH_PREFIX, uuid);
        Self {
            uuid,
            id,
            conn_type,
            state: CRConnectionState::Deactivated,
            device: None,
            path,
            autoconnect: true,
        }
    }
}

/// Helper function to convert connection type to u32
impl From<CRConnectionType> for u32 {
    fn from(ct: CRConnectionType) -> u32 {
        ct as u32
    }
}

/// Helper function to convert connection state to u32
impl From<CRConnectionState> for u32 {
    fn from(cs: CRConnectionState) -> u32 {
        cs as u32
    }
}

/// DHCP lease information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRDhcpLease {
    /// MAC address of the client
    pub mac_address: String,
    /// Assigned IP address
    pub ip_address: String,
    /// Client hostname (if provided)
    pub hostname: Option<String>,
    /// Lease expiration time (Unix timestamp)
    pub expiry: u64,
    /// Lease start time (Unix timestamp)
    pub start_time: u64,
}

/// Route type enumeration
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CRRouteType {
    /// Unknown route type
    Unknown = 0,
    /// Unicast route
    Unicast = 1,
    /// Local route
    Local = 2,
    /// Broadcast route
    Broadcast = 3,
    /// Anycast route
    Anycast = 4,
    /// Multicast route
    Multicast = 5,
    /// Blackhole route
    Blackhole = 6,
    /// Unreachable route
    Unreachable = 7,
    /// Prohibit route
    Prohibit = 8,
}

/// Route information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRRouteInfo {
    /// Destination network (e.g., "192.168.1.0/24" or "default")
    pub destination: String,
    /// Gateway/next hop IP address
    pub gateway: Option<String>,
    /// Outgoing interface
    pub interface: Option<String>,
    /// Route metric (priority)
    pub metric: u32,
    /// Route type
    pub route_type: CRRouteType,
    /// Route table ID
    pub table: u32,
    /// Route scope
    pub scope: u32,
}

impl CRRouteInfo {
    /// Create a new route info
    pub fn new(destination: String) -> Self {
        Self {
            destination,
            gateway: None,
            interface: None,
            metric: 0,
            route_type: CRRouteType::Unicast,
            table: 254, // Main routing table
            scope: 0,   // Global scope
        }
    }
}

/// Helper function to convert route type to u32
impl From<CRRouteType> for u32 {
    fn from(rt: CRRouteType) -> u32 {
        rt as u32
    }
}
