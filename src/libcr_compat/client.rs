//! CRClient - Main NetworkManager client (libnm NMClient equivalent)

use crate::error::{NetctlError, NetctlResult};
use crate::device::DeviceController;
use crate::interface::InterfaceController;
use crate::wifi::WifiController;
use super::device::CRDevice;
use super::connection::CRConnection;
use super::active_connection::CRActiveConnection;
use super::enums::{CRState, CRConnectivityState};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main client for interacting with network management (equivalent to NMClient)
///
/// CRClient is the primary entry point for all network management operations.
/// It provides access to devices, connections, and network state.
pub struct CRClient {
    device_controller: Arc<DeviceController>,
    interface_controller: Arc<InterfaceController>,
    wifi_controller: Arc<WifiController>,
    state: Arc<RwLock<CRState>>,
    connectivity: Arc<RwLock<CRConnectivityState>>,
}

impl CRClient {
    /// Creates a new CRClient instance (equivalent to nm_client_new)
    ///
    /// # Example
    /// ```no_run
    /// use netctl::libcr_compat::CRClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = CRClient::new().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn new() -> NetctlResult<Self> {
        Ok(Self {
            device_controller: Arc::new(DeviceController::new()),
            interface_controller: Arc::new(InterfaceController::new()),
            wifi_controller: Arc::new(WifiController::new()),
            state: Arc::new(RwLock::new(CRState::ConnectedGlobal)),
            connectivity: Arc::new(RwLock::new(CRConnectivityState::Full)),
        })
    }

    /// Creates a new CRClient instance asynchronously (equivalent to nm_client_new_async)
    pub async fn new_async() -> NetctlResult<Self> {
        Self::new().await
    }

    /// Gets all known network devices (equivalent to nm_client_get_devices)
    ///
    /// Returns a list of all network devices known to the system.
    ///
    /// # Example
    /// ```no_run
    /// # use netctl::libcr_compat::CRClient;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = CRClient::new().await?;
    /// let devices = client.get_devices().await?;
    /// for device in devices {
    ///     println!("Device: {}", device.get_iface());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_devices(&self) -> NetctlResult<Vec<CRDevice>> {
        let devices = self.device_controller.list_devices().await?;
        let mut cr_devices = Vec::new();

        for device in devices {
            cr_devices.push(CRDevice::from_device(
                device,
                self.interface_controller.clone(),
                self.wifi_controller.clone(),
            ));
        }

        Ok(cr_devices)
    }

    /// Gets a device by interface name (equivalent to nm_client_get_device_by_iface)
    ///
    /// # Arguments
    /// * `iface` - The interface name (e.g., "eth0", "wlan0")
    ///
    /// # Example
    /// ```no_run
    /// # use netctl::libcr_compat::CRClient;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = CRClient::new().await?;
    /// if let Some(device) = client.get_device_by_iface("eth0").await? {
    ///     println!("Found device: {}", device.get_iface());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_device_by_iface(&self, iface: &str) -> NetctlResult<Option<CRDevice>> {
        let devices = self.get_devices().await?;
        Ok(devices.into_iter().find(|d| d.get_iface() == iface))
    }

    /// Gets a device by path (equivalent to nm_client_get_device_by_path)
    pub async fn get_device_by_path(&self, path: &str) -> NetctlResult<Option<CRDevice>> {
        // Path format: /org/freedesktop/NetworkManager/Devices/N
        // Extract interface name from path or use path as interface name
        let iface = path.split('/').last().unwrap_or(path);
        self.get_device_by_iface(iface).await
    }

    /// Gets all active connections (equivalent to nm_client_get_active_connections)
    pub async fn get_active_connections(&self) -> NetctlResult<Vec<CRActiveConnection>> {
        let devices = self.get_devices().await?;
        let mut active_connections = Vec::new();

        for device in devices {
            if let Some(active_conn) = device.get_active_connection().await {
                active_connections.push(active_conn);
            }
        }

        Ok(active_connections)
    }

    /// Gets the primary connection (equivalent to nm_client_get_primary_connection)
    pub async fn get_primary_connection(&self) -> NetctlResult<Option<CRActiveConnection>> {
        let active_connections = self.get_active_connections().await?;
        Ok(active_connections.into_iter().next())
    }

    /// Activates a connection (equivalent to nm_client_activate_connection_async)
    ///
    /// # Arguments
    /// * `connection` - The connection to activate
    /// * `device` - The device to activate on (optional)
    ///
    /// # Example
    /// ```no_run
    /// # use netctl::libcr_compat::CRClient;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = CRClient::new().await?;
    /// # let connection = todo!();
    /// # let device = None;
    /// let active_conn = client.activate_connection(&connection, device.as_ref()).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn activate_connection(
        &self,
        connection: &CRConnection,
        device: Option<&CRDevice>,
    ) -> NetctlResult<CRActiveConnection> {
        let iface = if let Some(dev) = device {
            dev.get_iface().to_string()
        } else {
            // Find suitable device based on connection type
            return Err(NetctlError::InvalidParameter("Device required for activation".to_string()));
        };

        // Activate the connection
        self.interface_controller.up(&iface).await?;

        // Create and return active connection
        CRActiveConnection::new(connection.clone(), device.cloned())
    }

    /// Adds and activates a new connection (equivalent to nm_client_add_and_activate_connection_async)
    pub async fn add_and_activate_connection(
        &self,
        connection: Option<&CRConnection>,
        device: &CRDevice,
    ) -> NetctlResult<CRActiveConnection> {
        let conn = if let Some(c) = connection {
            c.clone()
        } else {
            // Create a default connection for the device
            CRConnection::new_for_device(device)?
        };

        self.activate_connection(&conn, Some(device)).await
    }

    /// Deactivates an active connection (equivalent to nm_client_deactivate_connection_async)
    pub async fn deactivate_connection(&self, active_connection: &CRActiveConnection) -> NetctlResult<()> {
        if let Some(device) = active_connection.get_device() {
            self.interface_controller.down(device.get_iface()).await?;
        }
        Ok(())
    }

    /// Gets the current network state (equivalent to nm_client_get_state)
    pub async fn get_state(&self) -> CRState {
        *self.state.read().await
    }

    /// Gets the network connectivity state (equivalent to nm_client_get_connectivity)
    pub async fn get_connectivity(&self) -> CRConnectivityState {
        *self.connectivity.read().await
    }

    /// Checks connectivity (equivalent to nm_client_check_connectivity_async)
    pub async fn check_connectivity(&self) -> NetctlResult<CRConnectivityState> {
        // In a real implementation, this would perform connectivity checks
        // For now, return the current cached state
        Ok(self.get_connectivity().await)
    }

    /// Gets whether networking is enabled (equivalent to nm_client_networking_get_enabled)
    pub fn networking_get_enabled(&self) -> bool {
        true // In a real implementation, check actual state
    }

    /// Sets whether networking is enabled (equivalent to nm_client_networking_set_enabled)
    pub async fn networking_set_enabled(&self, enabled: bool) -> NetctlResult<()> {
        if enabled {
            *self.state.write().await = CRState::ConnectedGlobal;
        } else {
            *self.state.write().await = CRState::Asleep;
        }
        Ok(())
    }

    /// Gets whether wireless is enabled (equivalent to nm_client_wireless_get_enabled)
    pub fn wireless_get_enabled(&self) -> bool {
        true // In a real implementation, check actual WiFi state
    }

    /// Sets whether wireless is enabled (equivalent to nm_client_wireless_set_enabled)
    pub async fn wireless_set_enabled(&self, enabled: bool) -> NetctlResult<()> {
        // In a real implementation, enable/disable WiFi devices
        Ok(())
    }

    /// Gets the version of NetworkManager (equivalent to nm_client_get_version)
    pub fn get_version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    /// Gets whether the daemon is running (equivalent to nm_client_get_nm_running)
    pub fn get_nm_running(&self) -> bool {
        true // libnetctl is always "running" as a library
    }
}

impl Default for CRClient {
    fn default() -> Self {
        Self {
            device_controller: Arc::new(DeviceController::new()),
            interface_controller: Arc::new(InterfaceController::new()),
            wifi_controller: Arc::new(WifiController::new()),
            state: Arc::new(RwLock::new(CRState::ConnectedGlobal)),
            connectivity: Arc::new(RwLock::new(CRConnectivityState::Full)),
        }
    }
}
