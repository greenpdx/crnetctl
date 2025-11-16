//! CRConnection - Network connection configuration (libnm NMConnection equivalent)

use super::device::CRDevice;
use super::settings::*;
use crate::error::{NetctlError, NetctlResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Network connection configuration (equivalent to NMConnection)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRConnection {
    /// Connection settings (NMSettingConnection)
    pub connection: CRSettingConnection,
    /// Wired settings (NMSettingWired)
    pub wired: Option<CRSettingWired>,
    /// Wireless settings (NMSettingWireless)
    pub wireless: Option<CRSettingWireless>,
    /// IPv4 settings (NMSettingIP4Config)
    pub ipv4: Option<CRSettingIP4Config>,
    /// IPv6 settings (NMSettingIP6Config)
    pub ipv6: Option<CRSettingIP6Config>,
    /// Additional settings
    pub settings: HashMap<String, CRSetting>,
}

impl CRConnection {
    /// Creates a new connection (equivalent to nm_simple_connection_new)
    pub fn new() -> Self {
        Self {
            connection: CRSettingConnection {
                id: "New Connection".to_string(),
                uuid: Uuid::new_v4().to_string(),
                connection_type: "802-3-ethernet".to_string(),
                interface_name: None,
                autoconnect: true,
                timestamp: 0,
            },
            wired: None,
            wireless: None,
            ipv4: None,
            ipv6: None,
            settings: HashMap::new(),
        }
    }

    /// Creates a new connection for a device
    pub(crate) fn new_for_device(device: &CRDevice) -> NetctlResult<Self> {
        let mut conn = Self::new();
        conn.connection.id = format!("{} connection", device.get_iface());
        conn.connection.interface_name = Some(device.get_iface().to_string());

        match device.get_device_type() {
            super::device::CRDeviceType::Ethernet => {
                conn.connection.connection_type = "802-3-ethernet".to_string();
                conn.wired = Some(CRSettingWired::default());
            }
            super::device::CRDeviceType::Wifi => {
                conn.connection.connection_type = "802-11-wireless".to_string();
                conn.wireless = Some(CRSettingWireless::default());
            }
            _ => {
                conn.connection.connection_type = "generic".to_string();
            }
        }

        Ok(conn)
    }

    /// Creates a connection from a device's current configuration
    pub(crate) fn new_from_device(device: &CRDevice) -> Self {
        let mut conn = Self::new_for_device(device).unwrap_or_else(|_| Self::new());
        conn.connection.id = format!("{} active", device.get_iface());
        conn
    }

    /// Gets the connection path (equivalent to nm_connection_get_path)
    pub fn get_path(&self) -> String {
        format!("/org/freedesktop/NetworkManager/Settings/{}",
                self.connection.uuid.replace("-", ""))
    }

    /// Gets the connection UUID (equivalent to nm_connection_get_uuid)
    pub fn get_uuid(&self) -> &str {
        &self.connection.uuid
    }

    /// Gets the connection ID (equivalent to nm_connection_get_id)
    pub fn get_id(&self) -> &str {
        &self.connection.id
    }

    /// Gets the connection type (equivalent to nm_connection_get_connection_type)
    pub fn get_connection_type(&self) -> &str {
        &self.connection.connection_type
    }

    /// Gets the interface name (equivalent to nm_connection_get_interface_name)
    pub fn get_interface_name(&self) -> Option<&str> {
        self.connection.interface_name.as_deref()
    }

    /// Gets a setting by name (equivalent to nm_connection_get_setting_by_name)
    pub fn get_setting_by_name(&self, name: &str) -> Option<&CRSetting> {
        self.settings.get(name)
    }

    /// Gets the connection setting (equivalent to nm_connection_get_setting_connection)
    pub fn get_setting_connection(&self) -> &CRSettingConnection {
        &self.connection
    }

    /// Gets the wired setting (equivalent to nm_connection_get_setting_wired)
    pub fn get_setting_wired(&self) -> Option<&CRSettingWired> {
        self.wired.as_ref()
    }

    /// Gets the wireless setting (equivalent to nm_connection_get_setting_wireless)
    pub fn get_setting_wireless(&self) -> Option<&CRSettingWireless> {
        self.wireless.as_ref()
    }

    /// Gets the IPv4 setting (equivalent to nm_connection_get_setting_ip4_config)
    pub fn get_setting_ip4_config(&self) -> Option<&CRSettingIP4Config> {
        self.ipv4.as_ref()
    }

    /// Gets the IPv6 setting (equivalent to nm_connection_get_setting_ip6_config)
    pub fn get_setting_ip6_config(&self) -> Option<&CRSettingIP6Config> {
        self.ipv6.as_ref()
    }

    /// Adds a setting to the connection (equivalent to nm_connection_add_setting)
    pub fn add_setting(&mut self, name: String, setting: CRSetting) {
        self.settings.insert(name, setting);
    }

    /// Removes a setting from the connection (equivalent to nm_connection_remove_setting)
    pub fn remove_setting(&mut self, name: &str) -> Option<CRSetting> {
        self.settings.remove(name)
    }

    /// Verifies the connection (equivalent to nm_connection_verify)
    pub fn verify(&self) -> NetctlResult<()> {
        if self.connection.id.is_empty() {
            return Err(NetctlError::InvalidParameter("Connection ID cannot be empty".to_string()));
        }
        if self.connection.uuid.is_empty() {
            return Err(NetctlError::InvalidParameter("Connection UUID cannot be empty".to_string()));
        }
        Ok(())
    }

    /// Normalizes the connection (equivalent to nm_connection_normalize)
    pub fn normalize(&mut self) -> NetctlResult<()> {
        // Ensure UUID is set
        if self.connection.uuid.is_empty() {
            self.connection.uuid = Uuid::new_v4().to_string();
        }

        // Ensure ID is set
        if self.connection.id.is_empty() {
            self.connection.id = "Connection".to_string();
        }

        Ok(())
    }

    /// Compares two connections (equivalent to nm_connection_compare)
    pub fn compare(&self, other: &CRConnection) -> bool {
        self.connection.uuid == other.connection.uuid
    }

    /// Duplicates the connection (equivalent to nm_connection_duplicate)
    pub fn duplicate(&self) -> Self {
        let mut dup = self.clone();
        dup.connection.uuid = Uuid::new_v4().to_string();
        dup.connection.id = format!("{} copy", self.connection.id);
        dup
    }

    /// Checks if the connection is of type ethernet
    pub fn is_type_ethernet(&self) -> bool {
        self.connection.connection_type == "802-3-ethernet"
    }

    /// Checks if the connection is of type WiFi
    pub fn is_type_wifi(&self) -> bool {
        self.connection.connection_type == "802-11-wireless"
    }

    /// Checks if the connection is of type VPN
    pub fn is_type_vpn(&self) -> bool {
        self.connection.connection_type == "vpn"
    }
}

impl Default for CRConnection {
    fn default() -> Self {
        Self::new()
    }
}

/// Remote connection (equivalent to NMRemoteConnection)
///
/// Represents a connection that is stored in NetworkManager's configuration.
#[derive(Debug, Clone)]
pub struct CRRemoteConnection {
    connection: CRConnection,
    unsaved: bool,
    visible: bool,
}

impl CRRemoteConnection {
    /// Creates a new remote connection
    pub fn new(connection: CRConnection) -> Self {
        Self {
            connection,
            unsaved: false,
            visible: true,
        }
    }

    /// Gets the underlying connection (equivalent to nm_remote_connection_get_connection)
    pub fn get_connection(&self) -> &CRConnection {
        &self.connection
    }

    /// Gets whether the connection has unsaved changes (equivalent to nm_remote_connection_get_unsaved)
    pub fn get_unsaved(&self) -> bool {
        self.unsaved
    }

    /// Gets whether the connection is visible to the user (equivalent to nm_remote_connection_get_visible)
    pub fn get_visible(&self) -> bool {
        self.visible
    }

    /// Commits changes to the connection (equivalent to nm_remote_connection_commit_changes_async)
    pub async fn commit_changes(&mut self) -> NetctlResult<()> {
        self.connection.verify()?;
        self.unsaved = false;
        Ok(())
    }

    /// Saves the connection (equivalent to nm_remote_connection_save_async)
    pub async fn save(&mut self) -> NetctlResult<()> {
        self.commit_changes().await
    }

    /// Deletes the connection (equivalent to nm_remote_connection_delete_async)
    pub async fn delete(self) -> NetctlResult<()> {
        // In a real implementation, this would remove from storage
        Ok(())
    }

    /// Updates the connection without saving (equivalent to nm_remote_connection_update2)
    pub fn update(&mut self, new_connection: CRConnection) {
        self.connection = new_connection;
        self.unsaved = true;
    }
}
