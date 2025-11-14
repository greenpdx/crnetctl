//! NetworkManager D-Bus compatibility layer
//!
//! This module provides a D-Bus interface compatible with NetworkManager,
//! allowing netctl to be used as a drop-in replacement for NetworkManager
//! in applications that depend on the NetworkManager D-Bus API.

use crate::error::{NetctlError, NetctlResult};
use crate::plugin::traits::PluginState;
use zbus::{Connection, dbus_interface, SignalContext, fdo};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

/// NetworkManager D-Bus service name
pub const NM_DBUS_SERVICE: &str = "org.freedesktop.NetworkManager";

/// NetworkManager D-Bus object path
pub const NM_DBUS_PATH: &str = "/org/freedesktop/NetworkManager";

/// Device states matching NetworkManager
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    Unknown = 0,
    Unmanaged = 10,
    Unavailable = 20,
    Disconnected = 30,
    Prepare = 40,
    Config = 50,
    NeedAuth = 60,
    IpConfig = 70,
    IpCheck = 80,
    Secondaries = 90,
    Activated = 100,
    Deactivating = 110,
    Failed = 120,
}

impl From<PluginState> for DeviceState {
    fn from(state: PluginState) -> Self {
        match state {
            PluginState::Uninitialized => DeviceState::Unknown,
            PluginState::Initializing => DeviceState::Prepare,
            PluginState::Ready => DeviceState::Disconnected,
            PluginState::Activating => DeviceState::Config,
            PluginState::Active => DeviceState::Activated,
            PluginState::Deactivating => DeviceState::Deactivating,
            PluginState::Failed => DeviceState::Failed,
            PluginState::Disabled => DeviceState::Unavailable,
        }
    }
}

/// Device information
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub path: String,
    pub interface: String,
    pub device_type: u32,
    pub state: DeviceState,
    pub ip4_address: Option<String>,
    pub ip6_address: Option<String>,
}

/// NetworkManager D-Bus interface
#[derive(Clone)]
pub struct NetworkManagerDBus {
    /// Tracked devices
    devices: Arc<RwLock<HashMap<String, DeviceInfo>>>,
    /// Network state
    state: Arc<RwLock<u32>>,
    /// Connectivity state
    connectivity: Arc<RwLock<u32>>,
}

impl NetworkManagerDBus {
    /// Create a new NetworkManager D-Bus interface
    pub fn new() -> Self {
        Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
            state: Arc::new(RwLock::new(70)), // NM_STATE_CONNECTED_GLOBAL
            connectivity: Arc::new(RwLock::new(4)), // NM_CONNECTIVITY_FULL
        }
    }

    /// Add a device
    pub async fn add_device(&self, device: DeviceInfo) {
        let mut devices = self.devices.write().await;
        let path = device.path.clone();
        devices.insert(path.clone(), device);
        info!("Device added: {}", path);
    }

    /// Remove a device
    pub async fn remove_device(&self, path: &str) {
        let mut devices = self.devices.write().await;
        if devices.remove(path).is_some() {
            info!("Device removed: {}", path);
        }
    }

    /// Update device state
    pub async fn update_device_state(&self, path: &str, state: DeviceState) -> NetctlResult<()> {
        let mut devices = self.devices.write().await;
        if let Some(device) = devices.get_mut(path) {
            device.state = state;
            info!("Device {} state changed to {:?}", path, state);
            Ok(())
        } else {
            Err(NetctlError::NotFound(format!("Device {} not found", path)))
        }
    }

    /// Get device by path
    pub async fn get_device(&self, path: &str) -> Option<DeviceInfo> {
        let devices = self.devices.read().await;
        devices.get(path).cloned()
    }

    /// Update global state
    pub async fn update_state(&self, new_state: u32) {
        let mut state = self.state.write().await;
        *state = new_state;
        info!("NetworkManager state changed to {}", new_state);
    }

    /// Update connectivity
    pub async fn update_connectivity(&self, new_connectivity: u32) {
        let mut connectivity = self.connectivity.write().await;
        *connectivity = new_connectivity;
        info!("Connectivity changed to {}", new_connectivity);
    }
}

/// Helper to emit D-Bus signals
pub mod signals {
    use super::*;

    /// Emit StateChanged signal
    pub async fn emit_state_changed(
        conn: &Connection,
        state: u32,
    ) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, NetworkManagerDBus>(NM_DBUS_PATH)
            .await
        {
            NetworkManagerDBus::state_changed(iface_ref.signal_context(), state)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit StateChanged signal: {}", e)))?;
        }
        Ok(())
    }

    /// Emit DeviceAdded signal
    pub async fn emit_device_added(
        conn: &Connection,
        device_path: &str,
    ) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, NetworkManagerDBus>(NM_DBUS_PATH)
            .await
        {
            NetworkManagerDBus::device_added(iface_ref.signal_context(), device_path)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit DeviceAdded signal: {}", e)))?;
        }
        Ok(())
    }

    /// Emit DeviceRemoved signal
    pub async fn emit_device_removed(
        conn: &Connection,
        device_path: &str,
    ) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, NetworkManagerDBus>(NM_DBUS_PATH)
            .await
        {
            NetworkManagerDBus::device_removed(iface_ref.signal_context(), device_path)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit DeviceRemoved signal: {}", e)))?;
        }
        Ok(())
    }

    /// Emit PropertiesChanged signal
    pub async fn emit_properties_changed(
        conn: &Connection,
        properties: HashMap<String, zbus::zvariant::Value<'_>>,
    ) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, NetworkManagerDBus>(NM_DBUS_PATH)
            .await
        {
            NetworkManagerDBus::properties_changed(iface_ref.signal_context(), properties)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit PropertiesChanged signal: {}", e)))?;
        }
        Ok(())
    }
}

#[dbus_interface(name = "org.freedesktop.NetworkManager")]
impl NetworkManagerDBus {
    /// Get all network devices
    async fn get_devices(&self) -> Vec<String> {
        let devices = self.devices.read().await;
        devices.keys().cloned().collect()
    }

    /// Get all devices (alternate method name)
    async fn get_all_devices(&self) -> Vec<String> {
        self.get_devices().await
    }

    /// Get a device by interface name
    async fn get_device_by_ip_iface(&self, iface: &str) -> fdo::Result<String> {
        let devices = self.devices.read().await;
        for (path, device) in devices.iter() {
            if device.interface == iface {
                return Ok(path.clone());
            }
        }
        Err(fdo::Error::Failed(format!("Device with interface {} not found", iface)))
    }

    /// Activate a connection
    async fn activate_connection(
        &self,
        connection: &str,
        device: &str,
        specific_object: &str,
    ) -> fdo::Result<String> {
        info!("Activating connection {} on device {}", connection, device);
        // Return active connection path
        Ok(format!("/org/freedesktop/NetworkManager/ActiveConnection/{}",
                   connection.split('/').last().unwrap_or("0")))
    }

    /// Deactivate a connection
    async fn deactivate_connection(&self, active_connection: &str) -> fdo::Result<()> {
        info!("Deactivating connection {}", active_connection);
        Ok(())
    }

    /// Get API version
    async fn version(&self) -> String {
        "1.46.0".to_string()
    }

    /// Get networking enabled state
    async fn networking_enabled(&self) -> bool {
        true
    }

    /// Get wireless enabled state
    async fn wireless_enabled(&self) -> bool {
        true
    }

    /// Get global state
    async fn state(&self) -> u32 {
        *self.state.read().await
    }

    /// Get connectivity state
    async fn connectivity(&self) -> u32 {
        *self.connectivity.read().await
    }

    /// Check connectivity
    async fn check_connectivity(&self) -> fdo::Result<u32> {
        Ok(*self.connectivity.read().await)
    }

    /// StateChanged signal - emitted when global networking state changes
    #[dbus_interface(signal)]
    async fn state_changed(ctxt: &SignalContext<'_>, state: u32) -> zbus::Result<()>;

    /// DeviceAdded signal - emitted when a device is added
    #[dbus_interface(signal)]
    async fn device_added(ctxt: &SignalContext<'_>, device_path: &str) -> zbus::Result<()>;

    /// DeviceRemoved signal - emitted when a device is removed
    #[dbus_interface(signal)]
    async fn device_removed(ctxt: &SignalContext<'_>, device_path: &str) -> zbus::Result<()>;

    /// PropertiesChanged signal - emitted when properties change
    #[dbus_interface(signal)]
    async fn properties_changed(
        ctxt: &SignalContext<'_>,
        properties: HashMap<String, zbus::zvariant::Value<'_>>,
    ) -> zbus::Result<()>;
}

/// Start the NetworkManager D-Bus service
pub async fn start_dbus_service() -> NetctlResult<(Arc<NetworkManagerDBus>, Arc<Connection>)> {
    info!("Starting NetworkManager D-Bus compatibility service");

    let connection = Connection::system().await
        .map_err(|e| NetctlError::ServiceError(format!("Failed to connect to D-Bus: {}", e)))?;

    let nm_dbus = NetworkManagerDBus::new();

    // Register the interface - zbus needs ownership of the object
    connection
        .object_server()
        .at(NM_DBUS_PATH, nm_dbus.clone())
        .await
        .map_err(|e| NetctlError::ServiceError(format!("Failed to register object: {}", e)))?;

    // Request well-known name
    match connection.request_name(NM_DBUS_SERVICE).await {
        Ok(_) => {
            info!("Successfully registered D-Bus service: {}", NM_DBUS_SERVICE);
        }
        Err(e) => {
            warn!("Failed to request D-Bus name: {}. Service may already be running.", e);
            // Don't fail - we can still operate without owning the name
        }
    }

    info!("NetworkManager D-Bus service started at {}", NM_DBUS_PATH);
    let conn_arc = Arc::new(connection);
    let nm_dbus_ref = Arc::new(nm_dbus);
    Ok((nm_dbus_ref, conn_arc))
}

impl Default for NetworkManagerDBus {
    fn default() -> Self {
        Self::new()
    }
}
