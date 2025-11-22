//! CR WiFi D-Bus interface
//!
//! D-Bus interface for WiFi operations

use super::types::*;
use crate::error::{NetctlError, NetctlResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug};
use zbus::{Connection, fdo, interface};
use zbus::object_server::SignalEmitter;
use zbus::zvariant::Value;

/// CR WiFi D-Bus interface
#[derive(Clone)]
pub struct CRWiFi {
    /// Scanned access points
    access_points: Arc<RwLock<Vec<CRAccessPointInfo>>>,
    /// Current connection SSID
    current_ssid: Arc<RwLock<Option<String>>>,
    /// Whether WiFi is enabled
    enabled: Arc<RwLock<bool>>,
    /// Whether scanning is in progress
    scanning: Arc<RwLock<bool>>,
}

impl CRWiFi {
    /// Create a new CR WiFi interface
    pub fn new() -> Self {
        Self {
            access_points: Arc::new(RwLock::new(Vec::new())),
            current_ssid: Arc::new(RwLock::new(None)),
            enabled: Arc::new(RwLock::new(true)),
            scanning: Arc::new(RwLock::new(false)),
        }
    }

    /// Update the list of scanned access points
    pub async fn update_access_points(&self, aps: Vec<CRAccessPointInfo>) {
        let mut access_points = self.access_points.write().await;
        *access_points = aps;
        info!("CR WiFi: Updated access points list ({} APs)", access_points.len());
    }

    /// Set current connected SSID
    pub async fn set_current_ssid(&self, ssid: Option<String>) {
        let mut current_ssid = self.current_ssid.write().await;
        *current_ssid = ssid.clone();
        if let Some(ref s) = ssid {
            info!("CR WiFi: Connected to SSID: {}", s);
        } else {
            info!("CR WiFi: Disconnected");
        }
    }

    /// Set WiFi enabled state (internal)
    pub async fn set_enabled_internal(&self, enabled: bool) {
        let mut e = self.enabled.write().await;
        *e = enabled;
        info!("CR WiFi: Enabled state set to {}", enabled);
    }

    /// Set scanning state
    pub async fn set_scanning(&self, scanning: bool) {
        let mut s = self.scanning.write().await;
        *s = scanning;
        debug!("CR WiFi: Scanning state set to {}", scanning);
    }
}

#[interface(name = "org.crrouter.NetworkControl.WiFi")]
impl CRWiFi {
    /// Get WiFi enabled state
    async fn get_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    /// Set WiFi enabled state
    async fn set_enabled(&self, enabled: bool) -> fdo::Result<()> {
        info!("CR WiFi: Setting enabled to {}", enabled);
        self.set_enabled_internal(enabled).await;
        Ok(())
    }

    /// Start a WiFi scan
    async fn scan(&self) -> fdo::Result<()> {
        info!("CR WiFi: Starting scan");
        self.set_scanning(true).await;
        // Actual scanning will be handled by integration layer
        Ok(())
    }

    /// Get list of scanned access points
    async fn get_access_points(&self) -> Vec<HashMap<String, Value<'static>>> {
        let access_points = self.access_points.read().await;
        let mut result = Vec::new();

        for ap in access_points.iter() {
            let mut ap_info = HashMap::new();
            ap_info.insert("SSID".to_string(), Value::new(ap.ssid.clone()));
            ap_info.insert("BSSID".to_string(), Value::new(ap.bssid.clone()));
            ap_info.insert("Strength".to_string(), Value::new(ap.strength));
            ap_info.insert("Security".to_string(), Value::new(ap.security as u32));
            ap_info.insert("Frequency".to_string(), Value::new(ap.frequency));
            ap_info.insert("Mode".to_string(), Value::new(ap.mode as u32));
            result.push(ap_info);
        }

        debug!("CR WiFi: Returning {} access points", result.len());
        result
    }

    /// Get current connected SSID
    async fn get_current_ssid(&self) -> String {
        let current_ssid = self.current_ssid.read().await;
        current_ssid.clone().unwrap_or_default()
    }

    /// Connect to a WiFi network
    async fn connect(
        &self,
        ssid: &str,
        password: &str,
        security: u32,
    ) -> fdo::Result<()> {
        info!("CR WiFi: Connecting to SSID: {}", ssid);

        // Validate security type
        let _security_type = match security {
            0 => CRWiFiSecurity::None,
            1 => CRWiFiSecurity::Wep,
            2 => CRWiFiSecurity::Wpa,
            3 => CRWiFiSecurity::Wpa2,
            4 => CRWiFiSecurity::Wpa3,
            5 => CRWiFiSecurity::Enterprise,
            _ => return Err(fdo::Error::InvalidArgs(format!("Invalid security type: {}", security))),
        };

        // Connection will be handled by integration layer
        Ok(())
    }

    /// Disconnect from current WiFi network
    async fn disconnect(&self) -> fdo::Result<()> {
        info!("CR WiFi: Disconnecting");
        // Disconnection will be handled by integration layer
        Ok(())
    }

    /// Start WiFi Access Point mode
    async fn start_access_point(
        &self,
        ssid: &str,
        password: &str,
        channel: u32,
    ) -> fdo::Result<()> {
        info!("CR WiFi: Starting AP mode - SSID: {}, Channel: {}", ssid, channel);
        // AP mode will be handled by integration layer
        Ok(())
    }

    /// Stop WiFi Access Point mode
    async fn stop_access_point(&self) -> fdo::Result<()> {
        info!("CR WiFi: Stopping AP mode");
        // AP mode stop will be handled by integration layer
        Ok(())
    }

    /// Get whether scanning is in progress
    async fn is_scanning(&self) -> bool {
        *self.scanning.read().await
    }

    // ============ D-Bus Signals ============

    /// ScanCompleted signal - emitted when a scan completes
    #[zbus(signal)]
    async fn scan_completed(signal_emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    /// AccessPointAdded signal - emitted when a new AP is detected
    #[zbus(signal)]
    async fn access_point_added(signal_emitter: &SignalEmitter<'_>, ssid: &str, bssid: &str) -> zbus::Result<()>;

    /// AccessPointRemoved signal - emitted when an AP is no longer visible
    #[zbus(signal)]
    async fn access_point_removed(signal_emitter: &SignalEmitter<'_>, ssid: &str, bssid: &str) -> zbus::Result<()>;

    /// Connected signal - emitted when connected to a network
    #[zbus(signal)]
    async fn connected(signal_emitter: &SignalEmitter<'_>, ssid: &str) -> zbus::Result<()>;

    /// Disconnected signal - emitted when disconnected from a network
    #[zbus(signal)]
    async fn disconnected(signal_emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

impl Default for CRWiFi {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper module for emitting WiFi signals
pub mod signals {
    use super::*;

    /// Emit ScanCompleted signal
    pub async fn emit_scan_completed(conn: &Connection) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRWiFi>(CR_WIFI_PATH)
            .await
        {
            CRWiFi::scan_completed(iface_ref.signal_emitter())
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit ScanCompleted: {}", e)))?;
        }
        Ok(())
    }

    /// Emit AccessPointAdded signal
    pub async fn emit_access_point_added(
        conn: &Connection,
        ssid: &str,
        bssid: &str,
    ) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRWiFi>(CR_WIFI_PATH)
            .await
        {
            CRWiFi::access_point_added(iface_ref.signal_emitter(), ssid, bssid)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit AccessPointAdded: {}", e)))?;
        }
        Ok(())
    }

    /// Emit AccessPointRemoved signal
    pub async fn emit_access_point_removed(
        conn: &Connection,
        ssid: &str,
        bssid: &str,
    ) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRWiFi>(CR_WIFI_PATH)
            .await
        {
            CRWiFi::access_point_removed(iface_ref.signal_emitter(), ssid, bssid)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit AccessPointRemoved: {}", e)))?;
        }
        Ok(())
    }

    /// Emit Connected signal
    pub async fn emit_connected(conn: &Connection, ssid: &str) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRWiFi>(CR_WIFI_PATH)
            .await
        {
            CRWiFi::connected(iface_ref.signal_emitter(), ssid)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit Connected: {}", e)))?;
        }
        Ok(())
    }

    /// Emit Disconnected signal
    pub async fn emit_disconnected(conn: &Connection) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRWiFi>(CR_WIFI_PATH)
            .await
        {
            CRWiFi::disconnected(iface_ref.signal_emitter())
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit Disconnected: {}", e)))?;
        }
        Ok(())
    }
}
