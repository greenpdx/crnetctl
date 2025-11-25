//! Enumerations compatible with libnm API

use serde::{Deserialize, Serialize};

/// NetworkManager daemon state (equivalent to NMState)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CRState {
    /// Networking state is unknown
    Unknown = 0,
    /// Networking is inactive and all devices are disabled
    Asleep = 10,
    /// There is no active network connection
    Disconnected = 20,
    /// Network connections are being cleaned up
    Disconnecting = 30,
    /// A network connection is being started
    Connecting = 40,
    /// There is only local IPv4 and/or IPv6 connectivity
    ConnectedLocal = 50,
    /// There is only site-wide IPv4 and/or IPv6 connectivity
    ConnectedSite = 60,
    /// There is global IPv4 and/or IPv6 Internet connectivity
    ConnectedGlobal = 70,
}

/// Network connectivity state (equivalent to NMConnectivityState)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CRConnectivityState {
    /// Network connectivity is unknown
    Unknown = 0,
    /// The host is not connected to any network
    None = 1,
    /// The host is behind a captive portal and cannot reach the full Internet
    Portal = 2,
    /// The host is connected to a network, but does not appear to be able to reach the full Internet
    Limited = 3,
    /// The host is connected to a network and appears to be able to reach the full Internet
    Full = 4,
}

/// Active connection state (equivalent to NMActiveConnectionState)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CRActiveConnectionState {
    /// The state of the connection is unknown
    Unknown = 0,
    /// A network connection is being prepared
    Activating = 1,
    /// There is a connection to the network
    Activated = 2,
    /// The network connection is being torn down and cleaned up
    Deactivating = 3,
    /// The network connection is disconnected and will be removed
    Deactivated = 4,
}

/// Device state reasons (equivalent to NMDeviceStateReason)
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CRDeviceStateReason {
    /// No reason given
    None = 0,
    /// Unknown error
    Unknown = 1,
    /// Device is now managed
    NowManaged = 2,
    /// Device is now unmanaged
    NowUnmanaged = 3,
    /// The device could not be readied for configuration
    ConfigFailed = 4,
    /// IP configuration could not be reserved (no available address, timeout, etc)
    IpConfigUnavailable = 5,
    /// The IP config is no longer valid
    IpConfigExpired = 6,
    /// Secrets were required, but not provided
    NoSecrets = 7,
    /// 802.1x supplicant disconnected
    SupplicantDisconnect = 8,
    /// 802.1x supplicant configuration failed
    SupplicantConfigFailed = 9,
    /// 802.1x supplicant failed
    SupplicantFailed = 10,
    /// 802.1x supplicant took too long to authenticate
    SupplicantTimeout = 11,
    /// PPP service failed to start
    PppStartFailed = 12,
    /// PPP service disconnected
    PppDisconnect = 13,
    /// PPP failed
    PppFailed = 14,
    /// DHCP client failed to start
    DhcpStartFailed = 15,
    /// DHCP client error
    DhcpError = 16,
    /// DHCP client failed
    DhcpFailed = 17,
    /// Shared connection service failed to start
    SharedStartFailed = 18,
    /// Shared connection service failed
    SharedFailed = 19,
    /// AutoIP service failed to start
    AutoIpStartFailed = 20,
    /// AutoIP service error
    AutoIpError = 21,
    /// AutoIP service failed
    AutoIpFailed = 22,
    /// The line is busy
    ModemBusy = 23,
    /// No dial tone
    ModemNoDialTone = 24,
    /// No carrier could be established
    ModemNoCarrier = 25,
    /// The dialing request timed out
    ModemDialTimeout = 26,
    /// The dialing attempt failed
    ModemDialFailed = 27,
    /// Modem initialization failed
    ModemInitFailed = 28,
    /// Failed to select the specified APN
    GsmApnFailed = 29,
    /// Not searching for networks
    GsmRegistrationNotSearching = 30,
    /// Network registration denied
    GsmRegistrationDenied = 31,
    /// Network registration timed out
    GsmRegistrationTimeout = 32,
    /// Failed to register with the requested network
    GsmRegistrationFailed = 33,
    /// PIN check failed
    GsmPinCheckFailed = 34,
    /// Necessary firmware for the device may be missing
    FirmwareMissing = 35,
    /// The device was removed
    Removed = 36,
    /// NetworkManager went to sleep
    Sleeping = 37,
    /// The device's active connection disappeared
    ConnectionRemoved = 38,
    /// Device disconnected by user or client
    UserRequested = 39,
    /// Carrier/link changed
    Carrier = 40,
    /// The device's existing connection was assumed
    ConnectionAssumed = 41,
    /// The supplicant is now available
    SupplicantAvailable = 42,
    /// The modem could not be found
    ModemNotFound = 43,
    /// The Bluetooth connection failed or timed out
    BtFailed = 44,
    /// GSM Modem's SIM Card not inserted
    GsmSimNotInserted = 45,
    /// GSM Modem's SIM Pin required
    GsmSimPinRequired = 46,
    /// GSM Modem's SIM Puk required
    GsmSimPukRequired = 47,
    /// GSM Modem's SIM wrong
    GsmSimWrong = 48,
    /// InfiniBand device does not support connected mode
    InfinibandMode = 49,
    /// A dependency of the connection failed
    DependencyFailed = 50,
    /// Problem with the RFC 2684 Ethernet over ADSL bridge
    Br2684Failed = 51,
    /// ModemManager not running
    ModemManagerUnavailable = 52,
    /// The WiFi network could not be found
    SsidNotFound = 53,
    /// A secondary connection of the base connection failed
    SecondaryConnectionFailed = 54,
    /// DCB or FCoE setup failed
    DcbFcoeFailed = 55,
    /// teamd control failed
    TeamdControlFailed = 56,
    /// Modem failed or no longer available
    ModemFailed = 57,
    /// Modem now ready and available
    ModemAvailable = 58,
    /// SIM PIN was incorrect
    SimPinIncorrect = 59,
    /// New connection activation was enqueued
    NewActivation = 60,
    /// The device's parent changed
    ParentChanged = 61,
    /// The device parent's management changed
    ParentManagedChanged = 62,
}

/// Connection type enumeration
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CRConnectionType {
    /// Ethernet connection
    Ethernet,
    /// WiFi connection
    Wifi,
    /// Bluetooth connection
    Bluetooth,
    /// VLAN connection
    Vlan,
    /// Bridge connection
    Bridge,
    /// Bond connection
    Bond,
    /// VPN connection
    Vpn,
    /// Generic connection
    Generic,
}
