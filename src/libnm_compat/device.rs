//! CRDevice - Network device representation (libnm NMDevice equivalent)

use crate::device::{Device, DeviceType as InternalDeviceType, DeviceState as InternalDeviceState};
use crate::interface::InterfaceController;
use crate::wifi::WifiController;
use crate::error::NetctlResult;
use super::active_connection::CRActiveConnection;
use super::connection::CRConnection;
use super::ip_config::CRIPConfig;
use super::access_point::CRAccessPoint;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

/// Device type enumeration (equivalent to NMDeviceType)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CRDeviceType {
    /// Unknown device type
    Unknown = 0,
    /// Generic device
    Generic = 14,
    /// Ethernet device
    Ethernet = 1,
    /// WiFi device
    Wifi = 2,
    /// Bluetooth device
    Bluetooth = 5,
    /// OLPC mesh device
    OlpcMesh = 6,
    /// WiMAX device
    Wimax = 7,
    /// Modem device
    Modem = 8,
    /// InfiniBand device
    Infiniband = 9,
    /// Bond device
    Bond = 10,
    /// VLAN device
    Vlan = 11,
    /// ADSL device
    Adsl = 12,
    /// Bridge device
    Bridge = 13,
    /// Team device
    Team = 15,
    /// TUN/TAP device
    Tun = 16,
    /// IP tunnel device
    IpTunnel = 17,
    /// MACVLAN device
    Macvlan = 18,
    /// VXLAN device
    Vxlan = 19,
    /// VETH device
    Veth = 20,
}

impl From<InternalDeviceType> for CRDeviceType {
    fn from(dt: InternalDeviceType) -> Self {
        match dt {
            InternalDeviceType::Ethernet => CRDeviceType::Ethernet,
            InternalDeviceType::Wifi => CRDeviceType::Wifi,
            InternalDeviceType::Bridge => CRDeviceType::Bridge,
            InternalDeviceType::Bond => CRDeviceType::Bond,
            InternalDeviceType::Vlan => CRDeviceType::Vlan,
            InternalDeviceType::TunTap => CRDeviceType::Tun,
            InternalDeviceType::Veth => CRDeviceType::Veth,
            InternalDeviceType::Vpn => CRDeviceType::IpTunnel,
            InternalDeviceType::Loopback => CRDeviceType::Generic,
            InternalDeviceType::Container => CRDeviceType::Generic,
            InternalDeviceType::Ppp => CRDeviceType::Generic,
            InternalDeviceType::Unknown => CRDeviceType::Unknown,
        }
    }
}

/// Device state enumeration (equivalent to NMDeviceState)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CRDeviceState {
    /// Device state is unknown
    Unknown = 0,
    /// Device is unmanaged
    Unmanaged = 10,
    /// Device is unavailable
    Unavailable = 20,
    /// Device is disconnected
    Disconnected = 30,
    /// Device is preparing to connect
    Prepare = 40,
    /// Device is being configured
    Config = 50,
    /// Device is waiting for secrets
    NeedAuth = 60,
    /// Device is requesting IP configuration
    IpConfig = 70,
    /// Device is checking IP connectivity
    IpCheck = 80,
    /// Device is waiting for secondary connections
    Secondaries = 90,
    /// Device is active
    Activated = 100,
    /// Device is being deactivated
    Deactivating = 110,
    /// Device has failed
    Failed = 120,
}

impl From<InternalDeviceState> for CRDeviceState {
    fn from(ds: InternalDeviceState) -> Self {
        match ds {
            InternalDeviceState::Up => CRDeviceState::Activated,
            InternalDeviceState::Down => CRDeviceState::Disconnected,
            InternalDeviceState::Unmanaged => CRDeviceState::Unmanaged,
            InternalDeviceState::Unavailable => CRDeviceState::Unavailable,
            InternalDeviceState::Error => CRDeviceState::Failed,
            InternalDeviceState::Unknown => CRDeviceState::Unknown,
        }
    }
}

/// Device capabilities (equivalent to NMDeviceCapabilities)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CRDeviceCapabilities {
    /// Device supports network management
    pub nm_supported: bool,
    /// Device supports carrier detection
    pub carrier_detect: bool,
    /// Device is a software device
    pub is_software: bool,
    /// Device supports SRIOV
    pub sriov: bool,
}

/// Network device representation (equivalent to NMDevice)
#[derive(Clone)]
pub struct CRDevice {
    device: Device,
    interface_controller: Arc<InterfaceController>,
    wifi_controller: Arc<WifiController>,
}

impl std::fmt::Debug for CRDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CRDevice")
            .field("device", &self.device)
            .finish()
    }
}

impl CRDevice {
    /// Creates a CRDevice from an internal Device
    pub(crate) fn from_device(
        device: Device,
        interface_controller: Arc<InterfaceController>,
        wifi_controller: Arc<WifiController>,
    ) -> Self {
        Self {
            device,
            interface_controller,
            wifi_controller,
        }
    }

    /// Gets the interface name (equivalent to nm_device_get_iface)
    pub fn get_iface(&self) -> &str {
        &self.device.name
    }

    /// Gets the device type (equivalent to nm_device_get_device_type)
    pub fn get_device_type(&self) -> CRDeviceType {
        self.device.device_type.into()
    }

    /// Gets the device state (equivalent to nm_device_get_state)
    pub fn get_state(&self) -> CRDeviceState {
        self.device.state.into()
    }

    /// Gets the device driver (equivalent to nm_device_get_driver)
    pub fn get_driver(&self) -> Option<String> {
        self.device.driver.clone()
    }

    /// Gets the device driver version (equivalent to nm_device_get_driver_version)
    pub fn get_driver_version(&self) -> Option<String> {
        None // Not currently tracked
    }

    /// Gets the device firmware version (equivalent to nm_device_get_firmware_version)
    pub fn get_firmware_version(&self) -> Option<String> {
        None // Not currently tracked
    }

    /// Gets the device capabilities (equivalent to nm_device_get_capabilities)
    pub fn get_capabilities(&self) -> CRDeviceCapabilities {
        let _caps = &self.device.capabilities;
        CRDeviceCapabilities {
            nm_supported: true,
            carrier_detect: true, // Assume carrier detection is available
            is_software: matches!(self.device.device_type, InternalDeviceType::Bridge | InternalDeviceType::Bond | InternalDeviceType::Vlan | InternalDeviceType::TunTap | InternalDeviceType::Veth),
            sriov: false, // SRIOV not currently tracked
        }
    }

    /// Gets the hardware address (MAC) (equivalent to nm_device_get_hw_address)
    pub fn get_hw_address(&self) -> Option<String> {
        self.device.mac_address.clone()
    }

    /// Gets the permanent hardware address (equivalent to nm_device_get_permanent_hw_address)
    pub fn get_permanent_hw_address(&self) -> Option<String> {
        self.device.mac_address.clone()
    }

    /// Gets the MTU (equivalent to nm_device_get_mtu)
    pub fn get_mtu(&self) -> u32 {
        self.device.mtu.unwrap_or(1500)
    }

    /// Gets whether the device is managed (equivalent to nm_device_get_managed)
    pub fn get_managed(&self) -> bool {
        true // libnetctl manages all discovered devices
    }

    /// Sets whether the device is managed (equivalent to nm_device_set_managed)
    pub async fn set_managed(&self, _managed: bool) -> NetctlResult<()> {
        Ok(()) // No-op for libnetctl
    }

    /// Gets whether the device should autoconnect (equivalent to nm_device_get_autoconnect)
    pub fn get_autoconnect(&self) -> bool {
        // Device doesn't have a config field, default to false
        false
    }

    /// Sets whether the device should autoconnect (equivalent to nm_device_set_autoconnect)
    pub async fn set_autoconnect(&self, _autoconnect: bool) -> NetctlResult<()> {
        Ok(()) // Would update device config
    }

    /// Gets the IPv4 configuration (equivalent to nm_device_get_ip4_config)
    pub async fn get_ip4_config(&self) -> Option<CRIPConfig> {
        if self.get_state() == CRDeviceState::Activated {
            Some(CRIPConfig::from_interface(self.get_iface(), false))
        } else {
            None
        }
    }

    /// Gets the IPv6 configuration (equivalent to nm_device_get_ip6_config)
    pub async fn get_ip6_config(&self) -> Option<CRIPConfig> {
        if self.get_state() == CRDeviceState::Activated {
            Some(CRIPConfig::from_interface(self.get_iface(), true))
        } else {
            None
        }
    }

    /// Gets the active connection (equivalent to nm_device_get_active_connection)
    pub async fn get_active_connection(&self) -> Option<CRActiveConnection> {
        if self.get_state() == CRDeviceState::Activated {
            let conn = CRConnection::new_from_device(self);
            Some(CRActiveConnection::new(conn, Some(self.clone())).ok()?)
        } else {
            None
        }
    }

    /// Gets available connections (equivalent to nm_device_get_available_connections)
    pub async fn get_available_connections(&self) -> Vec<CRConnection> {
        // Return stored/configured connections for this device
        vec![]
    }

    /// Disconnects the device (equivalent to nm_device_disconnect_async)
    pub async fn disconnect(&self) -> NetctlResult<()> {
        self.interface_controller.down(self.get_iface()).await
    }

    /// Deletes the device (equivalent to nm_device_delete_async)
    pub async fn delete(&self) -> NetctlResult<()> {
        // For virtual devices, this would remove them
        Err(crate::error::NetctlError::NotSupported(
            "Device deletion not supported for physical devices".to_string()
        ))
    }

    /// Checks if a connection is compatible with this device (equivalent to nm_device_connection_compatible)
    pub fn connection_compatible(&self, connection: &CRConnection) -> bool {
        // Check if connection type matches device type
        match self.get_device_type() {
            CRDeviceType::Ethernet => connection.is_type_ethernet(),
            CRDeviceType::Wifi => connection.is_type_wifi(),
            _ => false,
        }
    }

    /// Gets available access points (WiFi only) (equivalent to nm_device_wifi_get_access_points)
    pub async fn wifi_get_access_points(&self) -> NetctlResult<Vec<CRAccessPoint>> {
        if self.get_device_type() != CRDeviceType::Wifi {
            return Ok(vec![]);
        }

        // Perform WiFi scan (which returns results directly)
        let results = self.wifi_controller.scan(self.get_iface()).await?;
        Ok(results.into_iter().map(CRAccessPoint::from_scan_result).collect())
    }

    /// Requests a WiFi scan (equivalent to nm_device_wifi_request_scan_async)
    pub async fn wifi_request_scan(&self) -> NetctlResult<()> {
        if self.get_device_type() != CRDeviceType::Wifi {
            return Err(crate::error::NetctlError::InvalidParameter(
                "Device is not a WiFi device".to_string()
            ));
        }

        self.wifi_controller.scan(self.get_iface()).await.map(|_| ())
    }

    /// Gets the current WiFi access point (equivalent to nm_device_wifi_get_active_access_point)
    pub async fn wifi_get_active_access_point(&self) -> Option<CRAccessPoint> {
        if self.get_device_type() != CRDeviceType::Wifi || self.get_state() != CRDeviceState::Activated {
            return None;
        }

        // Get currently connected AP
        // This would require querying the WiFi interface for current connection
        None
    }

    /// Gets device statistics (equivalent to nm_device_get_statistics)
    pub fn get_statistics(&self) -> Option<DeviceStatistics> {
        self.device.stats.as_ref().map(|s| DeviceStatistics {
            tx_bytes: s.tx_bytes,
            rx_bytes: s.rx_bytes,
            tx_packets: s.tx_packets,
            rx_packets: s.rx_packets,
        })
    }
}

/// Device statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatistics {
    pub tx_bytes: u64,
    pub rx_bytes: u64,
    pub tx_packets: u64,
    pub rx_packets: u64,
}
