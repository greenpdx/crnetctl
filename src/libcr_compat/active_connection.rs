//! CRActiveConnection - Active network connection (libnm NMActiveConnection equivalent)

use super::connection::CRConnection;
use super::device::CRDevice;
use super::enums::CRActiveConnectionState;
use super::ip_config::CRIPConfig;
use crate::error::NetctlResult;
use serde::{Deserialize, Serialize};

/// Active network connection (equivalent to NMActiveConnection)
///
/// Represents an active (or activating) network connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRActiveConnection {
    /// The connection being used
    connection: CRConnection,
    /// The device(s) involved (if any)
    #[serde(skip)]
    device: Option<CRDevice>,
    /// Connection state
    state: CRActiveConnectionState,
    /// Whether this is the default IPv4 connection
    default4: bool,
    /// Whether this is the default IPv6 connection
    default6: bool,
    /// VPN connection flag
    vpn: bool,
    /// Connection ID
    id: String,
    /// Connection UUID
    uuid: String,
    /// Connection type
    connection_type: String,
    /// Specific object path (D-Bus path)
    specific_object: Option<String>,
}

impl CRActiveConnection {
    /// Creates a new active connection
    pub(crate) fn new(connection: CRConnection, device: Option<CRDevice>) -> NetctlResult<Self> {
        Ok(Self {
            id: connection.get_id().to_string(),
            uuid: connection.get_uuid().to_string(),
            connection_type: connection.get_connection_type().to_string(),
            connection,
            device,
            state: CRActiveConnectionState::Activated,
            default4: false,
            default6: false,
            vpn: false,
            specific_object: None,
        })
    }

    /// Gets the connection (equivalent to nm_active_connection_get_connection)
    pub fn get_connection(&self) -> &CRConnection {
        &self.connection
    }

    /// Gets the connection ID (equivalent to nm_active_connection_get_id)
    pub fn get_id(&self) -> &str {
        &self.id
    }

    /// Gets the connection UUID (equivalent to nm_active_connection_get_uuid)
    pub fn get_uuid(&self) -> &str {
        &self.uuid
    }

    /// Gets the connection type (equivalent to nm_active_connection_get_connection_type)
    pub fn get_connection_type(&self) -> &str {
        &self.connection_type
    }

    /// Gets the specific object (equivalent to nm_active_connection_get_specific_object)
    pub fn get_specific_object(&self) -> Option<&str> {
        self.specific_object.as_deref()
    }

    /// Gets the primary device (equivalent to nm_active_connection_get_device)
    pub fn get_device(&self) -> Option<&CRDevice> {
        self.device.as_ref()
    }

    /// Gets all devices (equivalent to nm_active_connection_get_devices)
    pub fn get_devices(&self) -> Vec<&CRDevice> {
        if let Some(ref dev) = self.device {
            vec![dev]
        } else {
            vec![]
        }
    }

    /// Gets the connection state (equivalent to nm_active_connection_get_state)
    pub fn get_state(&self) -> CRActiveConnectionState {
        self.state
    }

    /// Sets the connection state
    pub(crate) fn set_state(&mut self, state: CRActiveConnectionState) {
        self.state = state;
    }

    /// Gets whether this is the default IPv4 connection (equivalent to nm_active_connection_get_default)
    pub fn get_default(&self) -> bool {
        self.default4
    }

    /// Gets whether this is the default IPv6 connection (equivalent to nm_active_connection_get_default6)
    pub fn get_default6(&self) -> bool {
        self.default6
    }

    /// Sets whether this is the default IPv4 connection
    pub(crate) fn set_default(&mut self, is_default: bool) {
        self.default4 = is_default;
    }

    /// Sets whether this is the default IPv6 connection
    pub(crate) fn set_default6(&mut self, is_default: bool) {
        self.default6 = is_default;
    }

    /// Gets whether this is a VPN connection (equivalent to nm_active_connection_get_vpn)
    pub fn get_vpn(&self) -> bool {
        self.vpn
    }

    /// Gets the IPv4 configuration (equivalent to nm_active_connection_get_ip4_config)
    pub async fn get_ip4_config(&self) -> Option<CRIPConfig> {
        if let Some(ref device) = self.device {
            device.get_ip4_config().await
        } else {
            None
        }
    }

    /// Gets the IPv6 configuration (equivalent to nm_active_connection_get_ip6_config)
    pub async fn get_ip6_config(&self) -> Option<CRIPConfig> {
        if let Some(ref device) = self.device {
            device.get_ip6_config().await
        } else {
            None
        }
    }

    /// Gets the DHCP4 configuration (equivalent to nm_active_connection_get_dhcp4_config)
    pub fn get_dhcp4_config(&self) -> Option<CRDhcpConfig> {
        // Would return DHCP-specific configuration
        None
    }

    /// Gets the DHCP6 configuration (equivalent to nm_active_connection_get_dhcp6_config)
    pub fn get_dhcp6_config(&self) -> Option<CRDhcpConfig> {
        // Would return DHCPv6-specific configuration
        None
    }

    /// Gets the master connection (for slave connections) (equivalent to nm_active_connection_get_master)
    pub fn get_master(&self) -> Option<&CRDevice> {
        None // Would return master device for bridge/bond slaves
    }

    /// Gets the connection path (equivalent to nm_active_connection_get_path)
    pub fn get_path(&self) -> String {
        format!("/org/freedesktop/NetworkManager/ActiveConnection/{}",
                self.uuid.replace("-", ""))
    }
}

/// DHCP configuration (equivalent to NMDhcpConfig)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRDhcpConfig {
    /// DHCP options
    pub options: std::collections::HashMap<String, String>,
}

impl CRDhcpConfig {
    /// Creates a new DHCP configuration
    pub fn new() -> Self {
        Self {
            options: std::collections::HashMap::new(),
        }
    }

    /// Gets the DHCP options (equivalent to nm_dhcp_config_get_options)
    pub fn get_options(&self) -> &std::collections::HashMap<String, String> {
        &self.options
    }

    /// Gets a specific DHCP option (equivalent to nm_dhcp_config_get_one_option)
    pub fn get_one_option(&self, option: &str) -> Option<&str> {
        self.options.get(option).map(|s| s.as_str())
    }
}

impl Default for CRDhcpConfig {
    fn default() -> Self {
        Self::new()
    }
}
