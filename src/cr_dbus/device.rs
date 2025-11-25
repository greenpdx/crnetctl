//! CR Device D-Bus interface
//!
//! D-Bus interface for individual network devices

use super::types::*;
#[allow(unused_imports)]
use crate::error::NetctlResult;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug};
use zbus::{fdo, interface};
use zbus::object_server::SignalEmitter;
use zbus::zvariant::Value;

/// CR Device D-Bus interface
#[derive(Clone)]
pub struct CRDevice {
    /// Device information
    info: Arc<RwLock<CRDeviceInfo>>,
    /// Autoconnect enabled
    autoconnect: Arc<RwLock<bool>>,
    /// Device managed by NetworkControl
    managed: Arc<RwLock<bool>>,
}

impl CRDevice {
    /// Create a new CR Device interface
    pub fn new(info: CRDeviceInfo) -> Self {
        Self {
            info: Arc::new(RwLock::new(info)),
            autoconnect: Arc::new(RwLock::new(true)),  // Default to autoconnect enabled
            managed: Arc::new(RwLock::new(true)),      // Default to managed
        }
    }

    /// Update device state
    pub async fn set_state(&self, state: CRDeviceState) {
        let mut info = self.info.write().await;
        info.state = state;
        info!("CR Device {}: State changed to {:?}", info.interface, state);
    }

    /// Update IPv4 address
    pub async fn set_ipv4_address(&self, address: Option<String>) {
        let mut info = self.info.write().await;
        info.ipv4_address = address.clone();
        if let Some(ref addr) = address {
            info!("CR Device {}: IPv4 address set to {}", info.interface, addr);
        }
    }

    /// Update IPv6 address
    pub async fn set_ipv6_address(&self, address: Option<String>) {
        let mut info = self.info.write().await;
        info.ipv6_address = address.clone();
        if let Some(ref addr) = address {
            info!("CR Device {}: IPv6 address set to {}", info.interface, addr);
        }
    }

    /// Get device info
    pub async fn get_info(&self) -> CRDeviceInfo {
        self.info.read().await.clone()
    }
}

#[interface(name = "org.crrouter.NetworkControl.Device")]
impl CRDevice {
    /// Get device interface name
    async fn get_interface(&self) -> String {
        let info = self.info.read().await;
        info.interface.clone()
    }

    /// Get device type
    async fn get_device_type(&self) -> u32 {
        let info = self.info.read().await;
        info.device_type as u32
    }

    /// Get device state
    async fn get_state(&self) -> u32 {
        let info = self.info.read().await;
        info.state as u32
    }

    /// Get IPv4 address
    async fn get_ipv4_address(&self) -> String {
        let info = self.info.read().await;
        info.ipv4_address.clone().unwrap_or_default()
    }

    /// Get IPv6 address
    async fn get_ipv6_address(&self) -> String {
        let info = self.info.read().await;
        info.ipv6_address.clone().unwrap_or_default()
    }

    /// Get hardware address (MAC)
    async fn get_hw_address(&self) -> String {
        let info = self.info.read().await;
        info.hw_address.clone().unwrap_or_default()
    }

    /// Get MTU
    async fn get_mtu(&self) -> u32 {
        let info = self.info.read().await;
        info.mtu
    }

    /// Get all device properties as a dictionary
    async fn get_all_properties(&self) -> HashMap<String, Value<'static>> {
        let info = self.info.read().await;
        let mut props = HashMap::new();

        props.insert("Interface".to_string(), Value::new(info.interface.clone()));
        props.insert("DeviceType".to_string(), Value::new(info.device_type as u32));
        props.insert("State".to_string(), Value::new(info.state as u32));
        props.insert("Mtu".to_string(), Value::new(info.mtu));

        if let Some(ref ipv4) = info.ipv4_address {
            props.insert("IPv4Address".to_string(), Value::new(ipv4.clone()));
        }
        if let Some(ref ipv6) = info.ipv6_address {
            props.insert("IPv6Address".to_string(), Value::new(ipv6.clone()));
        }
        if let Some(ref hw) = info.hw_address {
            props.insert("HwAddress".to_string(), Value::new(hw.clone()));
        }

        debug!("CR Device {}: Returning all properties", info.interface);
        props
    }

    /// Activate the device (bring up)
    async fn activate(&self) -> fdo::Result<()> {
        let info = self.info.read().await;
        info!("CR Device {}: Activating", info.interface);
        // Activation will be handled by integration layer
        Ok(())
    }

    /// Deactivate the device (bring down)
    async fn deactivate(&self) -> fdo::Result<()> {
        let info = self.info.read().await;
        info!("CR Device {}: Deactivating", info.interface);
        // Deactivation will be handled by integration layer
        Ok(())
    }

    /// Set MTU
    async fn set_mtu(&self, mtu: u32) -> fdo::Result<()> {
        let mut info = self.info.write().await;
        info!("CR Device {}: Setting MTU to {}", info.interface, mtu);
        info.mtu = mtu;
        // MTU change will be handled by integration layer
        Ok(())
    }

    /// Set autoconnect enabled/disabled
    async fn set_autoconnect(&self, enabled: bool) -> fdo::Result<()> {
        let interface = {
            let info = self.info.read().await;
            info.interface.clone()
        };

        info!("CR Device {}: Setting autoconnect to {}", interface, enabled);
        let mut autoconnect = self.autoconnect.write().await;
        *autoconnect = enabled;
        // Autoconnect setting will be handled by integration layer
        Ok(())
    }

    /// Get autoconnect status
    async fn get_autoconnect(&self) -> bool {
        *self.autoconnect.read().await
    }

    /// Set whether device is managed by NetworkControl
    async fn set_managed(&self, managed: bool) -> fdo::Result<()> {
        let interface = {
            let info = self.info.read().await;
            info.interface.clone()
        };

        info!("CR Device {}: Setting managed to {}", interface, managed);
        let mut mgd = self.managed.write().await;
        *mgd = managed;
        // Managed setting will be handled by integration layer
        Ok(())
    }

    /// Get managed status
    async fn get_managed(&self) -> bool {
        *self.managed.read().await
    }

    // ============ D-Bus Signals ============

    /// StateChanged signal - emitted when device state changes
    #[zbus(signal)]
    async fn state_changed(signal_emitter: &SignalEmitter<'_>, new_state: u32, old_state: u32, reason: u32) -> zbus::Result<()>;

    /// IPConfigChanged signal - emitted when IP configuration changes
    #[zbus(signal)]
    async fn ip_config_changed(signal_emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

impl Default for CRDevice {
    fn default() -> Self {
        Self::new(CRDeviceInfo::new("unknown".to_string(), CRDeviceType::Unknown))
    }
}
