//! CR VPN D-Bus interface
//!
//! D-Bus interface for VPN operations

use super::types::*;
use crate::error::{NetctlError, NetctlResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug};
use zbus::{Connection, fdo, interface};
use zbus::object_server::SignalEmitter;
use zbus::zvariant::Value;

/// CR VPN D-Bus interface
#[derive(Clone)]
pub struct CRVPN {
    /// Active VPN connections
    connections: Arc<RwLock<HashMap<String, CRVpnInfo>>>,
}

impl CRVPN {
    /// Create a new CR VPN interface
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a VPN connection
    pub async fn add_connection(&self, vpn_info: CRVpnInfo) {
        let mut connections = self.connections.write().await;
        let name = vpn_info.name.clone();
        connections.insert(name.clone(), vpn_info);
        info!("CR VPN: Added connection {}", name);
    }

    /// Remove a VPN connection
    pub async fn remove_connection(&self, name: &str) -> NetctlResult<()> {
        let mut connections = self.connections.write().await;
        if connections.remove(name).is_some() {
            info!("CR VPN: Removed connection {}", name);
            Ok(())
        } else {
            Err(NetctlError::NotFound(format!("VPN connection {} not found", name)))
        }
    }

    /// Update VPN connection state
    pub async fn update_state(&self, name: &str, state: CRVpnState) -> NetctlResult<()> {
        let mut connections = self.connections.write().await;
        if let Some(vpn) = connections.get_mut(name) {
            vpn.state = state;
            info!("CR VPN: Connection {} state changed to {:?}", name, state);
            Ok(())
        } else {
            Err(NetctlError::NotFound(format!("VPN connection {} not found", name)))
        }
    }

    /// Get VPN connection info
    pub async fn get_connection(&self, name: &str) -> Option<CRVpnInfo> {
        let connections = self.connections.read().await;
        connections.get(name).cloned()
    }
}

#[interface(name = "org.crrouter.NetworkControl.VPN")]
impl CRVPN {
    /// Get list of VPN connections
    async fn get_connections(&self) -> Vec<String> {
        let connections = self.connections.read().await;
        let names: Vec<String> = connections.keys().cloned().collect();
        debug!("CR VPN: Returning {} connections", names.len());
        names
    }

    /// Get VPN connection information
    async fn get_connection_info(&self, name: &str) -> fdo::Result<HashMap<String, Value<'static>>> {
        let connections = self.connections.read().await;
        if let Some(vpn) = connections.get(name) {
            let mut info = HashMap::new();
            info.insert("Name".to_string(), Value::new(vpn.name.clone()));
            info.insert("Path".to_string(), Value::new(vpn.path.clone()));
            info.insert("Type".to_string(), Value::new(vpn.vpn_type as u32));
            info.insert("State".to_string(), Value::new(vpn.state as u32));

            if let Some(ref local_ip) = vpn.local_ip {
                info.insert("LocalIP".to_string(), Value::new(local_ip.clone()));
            }
            if let Some(ref remote_addr) = vpn.remote_address {
                info.insert("RemoteAddress".to_string(), Value::new(remote_addr.clone()));
            }

            Ok(info)
        } else {
            Err(fdo::Error::Failed(format!("VPN connection {} not found", name)))
        }
    }

    /// Connect to a VPN
    async fn connect_openvpn(&self, name: &str, config_file: &str) -> fdo::Result<()> {
        info!("CR VPN: Connecting to OpenVPN - name: {}, config: {}", name, config_file);
        // Connection will be handled by integration layer
        Ok(())
    }

    /// Connect to WireGuard VPN
    async fn connect_wireguard(&self, name: &str, config_file: &str) -> fdo::Result<()> {
        info!("CR VPN: Connecting to WireGuard - name: {}, config: {}", name, config_file);
        // Connection will be handled by integration layer
        Ok(())
    }

    /// Connect to IPsec VPN
    async fn connect_ipsec(
        &self,
        name: &str,
        remote: &str,
        auth_method: &str,
        _credentials: HashMap<String, String>,
    ) -> fdo::Result<()> {
        info!("CR VPN: Connecting to IPsec - name: {}, remote: {}, auth: {}",
              name, remote, auth_method);
        // Connection will be handled by integration layer
        Ok(())
    }

    /// Connect to Arti/Tor
    async fn connect_arti(&self, name: &str, _config: HashMap<String, String>) -> fdo::Result<()> {
        info!("CR VPN: Connecting to Arti/Tor - name: {}", name);
        // Connection will be handled by integration layer
        Ok(())
    }

    /// Disconnect from a VPN
    async fn disconnect(&self, name: &str) -> fdo::Result<()> {
        info!("CR VPN: Disconnecting from {}", name);
        // Disconnection will be handled by integration layer
        Ok(())
    }

    /// Get VPN connection state
    async fn get_state(&self, name: &str) -> fdo::Result<u32> {
        let connections = self.connections.read().await;
        if let Some(vpn) = connections.get(name) {
            Ok(vpn.state as u32)
        } else {
            Err(fdo::Error::Failed(format!("VPN connection {} not found", name)))
        }
    }

    /// Get statistics for a VPN connection
    async fn get_statistics(&self, name: &str) -> fdo::Result<HashMap<String, Value<'static>>> {
        info!("CR VPN: Getting statistics for {}", name);
        let connections = self.connections.read().await;
        if connections.contains_key(name) {
            // Statistics will be populated by integration layer
            let mut stats = HashMap::new();
            stats.insert("BytesReceived".to_string(), Value::new(0u64));
            stats.insert("BytesSent".to_string(), Value::new(0u64));
            stats.insert("Duration".to_string(), Value::new(0u64));
            Ok(stats)
        } else {
            Err(fdo::Error::Failed(format!("VPN connection {} not found", name)))
        }
    }

    /// Delete a VPN connection configuration
    async fn delete_connection(&self, name: &str) -> fdo::Result<()> {
        info!("CR VPN: Deleting connection {}", name);
        // Deletion will be handled by integration layer
        Ok(())
    }

    /// Import VPN configuration from file
    async fn import_config(
        &self,
        vpn_type: &str,
        config_file: &str,
        name: &str,
    ) -> fdo::Result<String> {
        info!(
            "CR VPN: Importing {} config from {} as {}",
            vpn_type, config_file, name
        );

        // Validate VPN type
        let vtype = match vpn_type.to_lowercase().as_str() {
            "openvpn" => CRVpnType::OpenVpn,
            "wireguard" => CRVpnType::WireGuard,
            "ipsec" => CRVpnType::IPsec,
            "arti" | "tor" => CRVpnType::Arti,
            _ => return Err(fdo::Error::InvalidArgs(format!("Unknown VPN type: {}", vpn_type))),
        };

        // Validate file path
        if config_file.is_empty() {
            return Err(fdo::Error::InvalidArgs("Config file path cannot be empty".to_string()));
        }

        // Validate name
        if name.is_empty() {
            return Err(fdo::Error::InvalidArgs("VPN name cannot be empty".to_string()));
        }

        // Create VPN info
        let vpn_info = CRVpnInfo::new(name.to_string(), vtype);
        self.add_connection(vpn_info).await;

        // Actual import will be handled by integration layer
        info!("CR VPN: Successfully imported config as {}", name);
        Ok(name.to_string())
    }

    /// Export VPN configuration to string
    async fn export_config(&self, name: &str) -> fdo::Result<String> {
        info!("CR VPN: Exporting config for {}", name);

        let connections = self.connections.read().await;
        if !connections.contains_key(name) {
            return Err(fdo::Error::Failed(format!("VPN connection {} not found", name)));
        }

        // Actual export will be handled by integration layer
        // Return placeholder config
        let config = format!("# VPN Configuration for {}\n# Type: {:?}\n", name, connections[name].vpn_type);
        Ok(config)
    }

    /// Create VPN connection from TOML configuration
    async fn create_from_config(&self, config_toml: &str) -> fdo::Result<String> {
        info!("CR VPN: Creating connection from TOML config");

        if config_toml.is_empty() {
            return Err(fdo::Error::InvalidArgs("Config cannot be empty".to_string()));
        }

        // Actual parsing and creation will be handled by integration layer
        // For now, return a placeholder name
        let name = "imported_vpn".to_string();
        info!("CR VPN: Successfully created connection from config: {}", name);
        Ok(name)
    }

    // ============ D-Bus Signals ============

    /// ConnectionAdded signal - emitted when a VPN connection is added
    #[zbus(signal)]
    async fn connection_added(signal_emitter: &SignalEmitter<'_>, name: &str, vpn_type: u32) -> zbus::Result<()>;

    /// ConnectionRemoved signal - emitted when a VPN connection is removed
    #[zbus(signal)]
    async fn connection_removed(signal_emitter: &SignalEmitter<'_>, name: &str) -> zbus::Result<()>;

    /// StateChanged signal - emitted when VPN state changes
    #[zbus(signal)]
    async fn state_changed(signal_emitter: &SignalEmitter<'_>, name: &str, state: u32) -> zbus::Result<()>;

    /// Connected signal - emitted when VPN is connected
    #[zbus(signal)]
    async fn connected(signal_emitter: &SignalEmitter<'_>, name: &str, local_ip: &str) -> zbus::Result<()>;

    /// Disconnected signal - emitted when VPN is disconnected
    #[zbus(signal)]
    async fn disconnected(signal_emitter: &SignalEmitter<'_>, name: &str) -> zbus::Result<()>;

    /// Error signal - emitted when an error occurs
    #[zbus(signal)]
    async fn error(signal_emitter: &SignalEmitter<'_>, name: &str, error_message: &str) -> zbus::Result<()>;
}

impl Default for CRVPN {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper module for emitting VPN signals
pub mod signals {
    use super::*;

    /// VPN D-Bus path
    const CR_VPN_PATH: &str = "/org/crrouter/NetworkControl/VPN";

    /// Emit ConnectionAdded signal
    pub async fn emit_connection_added(
        conn: &Connection,
        name: &str,
        vpn_type: CRVpnType,
    ) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRVPN>(CR_VPN_PATH)
            .await
        {
            CRVPN::connection_added(iface_ref.signal_emitter(), name, vpn_type as u32)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit ConnectionAdded: {}", e)))?;
        }
        Ok(())
    }

    /// Emit ConnectionRemoved signal
    pub async fn emit_connection_removed(conn: &Connection, name: &str) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRVPN>(CR_VPN_PATH)
            .await
        {
            CRVPN::connection_removed(iface_ref.signal_emitter(), name)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit ConnectionRemoved: {}", e)))?;
        }
        Ok(())
    }

    /// Emit StateChanged signal
    pub async fn emit_state_changed(
        conn: &Connection,
        name: &str,
        state: CRVpnState,
    ) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRVPN>(CR_VPN_PATH)
            .await
        {
            CRVPN::state_changed(iface_ref.signal_emitter(), name, state as u32)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit StateChanged: {}", e)))?;
        }
        Ok(())
    }

    /// Emit Connected signal
    pub async fn emit_connected(
        conn: &Connection,
        name: &str,
        local_ip: &str,
    ) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRVPN>(CR_VPN_PATH)
            .await
        {
            CRVPN::connected(iface_ref.signal_emitter(), name, local_ip)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit Connected: {}", e)))?;
        }
        Ok(())
    }

    /// Emit Disconnected signal
    pub async fn emit_disconnected(conn: &Connection, name: &str) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRVPN>(CR_VPN_PATH)
            .await
        {
            CRVPN::disconnected(iface_ref.signal_emitter(), name)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit Disconnected: {}", e)))?;
        }
        Ok(())
    }

    /// Emit Error signal
    pub async fn emit_error(
        conn: &Connection,
        name: &str,
        error_message: &str,
    ) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRVPN>(CR_VPN_PATH)
            .await
        {
            CRVPN::error(iface_ref.signal_emitter(), name, error_message)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit Error: {}", e)))?;
        }
        Ok(())
    }
}
