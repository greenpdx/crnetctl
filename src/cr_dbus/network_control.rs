//! CR Network Control D-Bus interface
//!
//! Main D-Bus interface for controlling network operations through the CR router

use super::types::*;
use crate::error::{NetctlError, NetctlResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug};
use zbus::{Connection, fdo, interface};
use zbus::object_server::SignalEmitter;
use zbus::zvariant::Value;

/// CR Network Control D-Bus interface
#[derive(Clone)]
pub struct CRNetworkControl {
    /// Tracked devices by path
    devices: Arc<RwLock<HashMap<String, CRDeviceInfo>>>,
    /// Global network state
    state: Arc<RwLock<CRNetworkState>>,
    /// Connectivity state
    connectivity: Arc<RwLock<CRConnectivity>>,
    /// Whether networking is enabled
    networking_enabled: Arc<RwLock<bool>>,
    /// Whether wireless is enabled
    wireless_enabled: Arc<RwLock<bool>>,
    /// API version
    version: String,
}

impl CRNetworkControl {
    /// Create a new CR Network Control interface
    pub fn new() -> Self {
        Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
            state: Arc::new(RwLock::new(CRNetworkState::Disconnected)),
            connectivity: Arc::new(RwLock::new(CRConnectivity::Unknown)),
            networking_enabled: Arc::new(RwLock::new(true)),
            wireless_enabled: Arc::new(RwLock::new(true)),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Add a device to tracking
    pub async fn add_device(&self, device: CRDeviceInfo) {
        let mut devices = self.devices.write().await;
        let path = device.path.clone();
        info!("CR: Adding device {} at {}", device.interface, path);
        devices.insert(path, device);
    }

    /// Remove a device from tracking
    pub async fn remove_device(&self, path: &str) -> NetctlResult<()> {
        let mut devices = self.devices.write().await;
        if devices.remove(path).is_some() {
            info!("CR: Removed device at {}", path);
            Ok(())
        } else {
            Err(NetctlError::NotFound(format!("Device {} not found", path)))
        }
    }

    /// Update device state
    pub async fn update_device_state(&self, path: &str, state: CRDeviceState) -> NetctlResult<()> {
        let mut devices = self.devices.write().await;
        if let Some(device) = devices.get_mut(path) {
            device.state = state;
            info!("CR: Device {} state updated to {:?}", path, state);
            Ok(())
        } else {
            Err(NetctlError::NotFound(format!("Device {} not found", path)))
        }
    }

    /// Get device information (internal)
    pub async fn get_device_info_internal(&self, path: &str) -> Option<CRDeviceInfo> {
        let devices = self.devices.read().await;
        devices.get(path).cloned()
    }

    /// Update global network state
    pub async fn set_network_state(&self, new_state: CRNetworkState) {
        let mut state = self.state.write().await;
        *state = new_state;
        info!("CR: Network state changed to {:?}", new_state);
    }

    /// Update connectivity state
    pub async fn set_connectivity(&self, new_connectivity: CRConnectivity) {
        let mut connectivity = self.connectivity.write().await;
        *connectivity = new_connectivity;
        info!("CR: Connectivity changed to {:?}", new_connectivity);
    }

    /// Set networking enabled state
    pub async fn set_networking_enabled(&self, enabled: bool) {
        let mut networking_enabled = self.networking_enabled.write().await;
        *networking_enabled = enabled;
        info!("CR: Networking enabled: {}", enabled);
    }

    /// Set wireless enabled state
    pub async fn set_wireless_enabled(&self, enabled: bool) {
        let mut wireless_enabled = self.wireless_enabled.write().await;
        *wireless_enabled = enabled;
        info!("CR: Wireless enabled: {}", enabled);
    }
}

#[interface(name = "org.crrouter.NetworkControl")]
impl CRNetworkControl {
    /// Get API version
    async fn get_version(&self) -> String {
        self.version.clone()
    }

    /// Get all network devices
    async fn get_devices(&self) -> Vec<String> {
        let devices = self.devices.read().await;
        let paths: Vec<String> = devices.keys().cloned().collect();
        debug!("CR: GetDevices returning {} devices", paths.len());
        paths
    }

    /// Get device by interface name
    async fn get_device_by_interface(&self, iface: &str) -> fdo::Result<String> {
        let devices = self.devices.read().await;
        for (path, device) in devices.iter() {
            if device.interface == iface {
                debug!("CR: Found device {} for interface {}", path, iface);
                return Ok(path.clone());
            }
        }
        Err(fdo::Error::Failed(format!("Device with interface {} not found", iface)))
    }

    /// Get device information as a dictionary
    async fn get_device_info(&self, device_path: &str) -> fdo::Result<HashMap<String, Value<'_>>> {
        let devices = self.devices.read().await;
        if let Some(device) = devices.get(device_path) {
            let mut info = HashMap::new();
            info.insert("Interface".to_string(), Value::new(device.interface.clone()));
            info.insert("DeviceType".to_string(), Value::new(device.device_type as u32));
            info.insert("State".to_string(), Value::new(device.state as u32));

            if let Some(ref ipv4) = device.ipv4_address {
                info.insert("IPv4Address".to_string(), Value::new(ipv4.clone()));
            }
            if let Some(ref ipv6) = device.ipv6_address {
                info.insert("IPv6Address".to_string(), Value::new(ipv6.clone()));
            }
            if let Some(ref hw) = device.hw_address {
                info.insert("HwAddress".to_string(), Value::new(hw.clone()));
            }
            info.insert("Mtu".to_string(), Value::new(device.mtu));

            Ok(info)
        } else {
            Err(fdo::Error::Failed(format!("Device {} not found", device_path)))
        }
    }

    /// Activate a device (bring it up)
    async fn activate_device(&self, device_path: &str) -> fdo::Result<()> {
        info!("CR: Activating device {}", device_path);

        let devices = self.devices.read().await;
        if !devices.contains_key(device_path) {
            return Err(fdo::Error::Failed(format!("Device {} not found", device_path)));
        }

        // Device activation will be handled by the integration layer
        Ok(())
    }

    /// Deactivate a device (bring it down)
    async fn deactivate_device(&self, device_path: &str) -> fdo::Result<()> {
        info!("CR: Deactivating device {}", device_path);

        let devices = self.devices.read().await;
        if !devices.contains_key(device_path) {
            return Err(fdo::Error::Failed(format!("Device {} not found", device_path)));
        }

        // Device deactivation will be handled by the integration layer
        Ok(())
    }

    /// Get global network state
    async fn get_state(&self) -> u32 {
        let state = self.state.read().await;
        (*state) as u32
    }

    /// Get connectivity state
    async fn get_connectivity(&self) -> u32 {
        let connectivity = self.connectivity.read().await;
        (*connectivity) as u32
    }

    /// Check connectivity (performs active check)
    async fn check_connectivity(&self) -> fdo::Result<u32> {
        info!("CR: Checking connectivity");
        let connectivity = self.connectivity.read().await;
        Ok((*connectivity) as u32)
    }

    /// Get networking enabled state
    async fn get_networking_enabled(&self) -> bool {
        *self.networking_enabled.read().await
    }

    /// Set networking enabled state
    async fn set_networking_enabled_method(&self, enabled: bool) -> fdo::Result<()> {
        info!("CR: Setting networking enabled to {}", enabled);
        self.set_networking_enabled(enabled).await;
        Ok(())
    }

    /// Get wireless enabled state
    async fn get_wireless_enabled(&self) -> bool {
        *self.wireless_enabled.read().await
    }

    /// Set wireless enabled state
    async fn set_wireless_enabled_method(&self, enabled: bool) -> fdo::Result<()> {
        info!("CR: Setting wireless enabled to {}", enabled);
        self.set_wireless_enabled(enabled).await;
        Ok(())
    }

    /// Reload configuration
    async fn reload(&self) -> fdo::Result<()> {
        info!("CR: Reloading configuration");
        // Configuration reload will be handled by the integration layer
        Ok(())
    }

    // ============ D-Bus Signals ============

    /// StateChanged signal - emitted when global network state changes
    #[zbus(signal)]
    async fn state_changed(signal_emitter: &SignalEmitter<'_>, state: u32) -> zbus::Result<()>;

    /// DeviceAdded signal - emitted when a device is added
    #[zbus(signal)]
    async fn device_added(signal_emitter: &SignalEmitter<'_>, device_path: &str) -> zbus::Result<()>;

    /// DeviceRemoved signal - emitted when a device is removed
    #[zbus(signal)]
    async fn device_removed(signal_emitter: &SignalEmitter<'_>, device_path: &str) -> zbus::Result<()>;

    /// DeviceStateChanged signal - emitted when a device state changes
    #[zbus(signal)]
    async fn device_state_changed(signal_emitter: &SignalEmitter<'_>, device_path: &str, state: u32) -> zbus::Result<()>;

    /// ConnectivityChanged signal - emitted when connectivity changes
    #[zbus(signal)]
    async fn connectivity_changed(signal_emitter: &SignalEmitter<'_>, connectivity: u32) -> zbus::Result<()>;

    /// PropertiesChanged signal - emitted when properties change
    #[zbus(signal)]
    async fn properties_changed(signal_emitter: &SignalEmitter<'_>, properties: HashMap<String, Value<'_>>) -> zbus::Result<()>;
}

impl Default for CRNetworkControl {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper module for emitting signals
pub mod signals {
    use super::*;

    /// Emit StateChanged signal
    pub async fn emit_state_changed(
        conn: &Connection,
        state: CRNetworkState,
    ) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRNetworkControl>(CR_DBUS_PATH)
            .await
        {
            CRNetworkControl::state_changed(iface_ref.signal_emitter(), state as u32)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit StateChanged: {}", e)))?;
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
            .interface::<_, CRNetworkControl>(CR_DBUS_PATH)
            .await
        {
            CRNetworkControl::device_added(iface_ref.signal_emitter(), device_path)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit DeviceAdded: {}", e)))?;
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
            .interface::<_, CRNetworkControl>(CR_DBUS_PATH)
            .await
        {
            CRNetworkControl::device_removed(iface_ref.signal_emitter(), device_path)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit DeviceRemoved: {}", e)))?;
        }
        Ok(())
    }

    /// Emit DeviceStateChanged signal
    pub async fn emit_device_state_changed(
        conn: &Connection,
        device_path: &str,
        state: CRDeviceState,
    ) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRNetworkControl>(CR_DBUS_PATH)
            .await
        {
            CRNetworkControl::device_state_changed(
                iface_ref.signal_emitter(),
                device_path,
                state as u32,
            )
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to emit DeviceStateChanged: {}", e)))?;
        }
        Ok(())
    }

    /// Emit ConnectivityChanged signal
    pub async fn emit_connectivity_changed(
        conn: &Connection,
        connectivity: CRConnectivity,
    ) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRNetworkControl>(CR_DBUS_PATH)
            .await
        {
            CRNetworkControl::connectivity_changed(iface_ref.signal_emitter(), connectivity as u32)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit ConnectivityChanged: {}", e)))?;
        }
        Ok(())
    }

    /// Emit PropertiesChanged signal
    pub async fn emit_properties_changed(
        conn: &Connection,
        properties: HashMap<String, Value<'_>>,
    ) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRNetworkControl>(CR_DBUS_PATH)
            .await
        {
            CRNetworkControl::properties_changed(iface_ref.signal_emitter(), properties)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit PropertiesChanged: {}", e)))?;
        }
        Ok(())
    }
}
