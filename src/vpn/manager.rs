use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::plugin::ConnectionConfig;
use crate::error::{NetctlError, NetctlResult};
use super::backend::{VpnBackend, VpnBackendFactory, VpnState, VpnStats};

/// Represents an active VPN connection
struct VpnConnection {
    #[allow(dead_code)]
    uuid: String,
    config: ConnectionConfig,
    backend: Box<dyn VpnBackend>,
    interface_name: Option<String>,
}

/// VPN Manager - provides a unified interface for managing all VPN connections
/// across different VPN technologies (WireGuard, OpenVPN, IPsec, etc.)
pub struct VpnManager {
    /// Active VPN connections
    connections: Arc<RwLock<HashMap<String, VpnConnection>>>,
    /// Registered backend factories
    backends: HashMap<String, VpnBackendFactory>,
    /// Configuration directory
    #[allow(dead_code)]
    config_dir: PathBuf,
}

impl VpnManager {
    /// Create a new VPN manager
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            backends: HashMap::new(),
            config_dir,
        }
    }

    /// Register a VPN backend driver
    pub fn register_backend(&mut self, name: &str, factory: VpnBackendFactory) {
        info!("Registering VPN backend: {}", name);
        self.backends.insert(name.to_string(), factory);
    }

    /// Get a list of registered backend names
    pub fn available_backends(&self) -> Vec<String> {
        self.backends.keys().cloned().collect()
    }

    /// Check if a backend is registered
    pub fn has_backend(&self, name: &str) -> bool {
        self.backends.contains_key(name)
    }

    /// Create a VPN connection
    pub async fn create_connection(&self, config: ConnectionConfig) -> NetctlResult<String> {
        let vpn_type = config.settings.get("vpn_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NetctlError::InvalidParameter("Missing vpn_type in config".to_string()))?;

        let factory = self.backends.get(vpn_type)
            .ok_or_else(|| NetctlError::NotSupported(format!("VPN type '{}' not supported", vpn_type)))?;

        let backend = factory();

        // Check if backend is available
        if !backend.is_available().await {
            return Err(NetctlError::NotSupported(format!(
                "VPN backend '{}' is not available (software not installed?)",
                vpn_type
            )));
        }

        // Validate configuration
        backend.validate_config(&config).await?;

        let uuid = config.uuid.clone();
        let connection = VpnConnection {
            uuid: uuid.clone(),
            config,
            backend,
            interface_name: None,
        };

        let mut connections = self.connections.write().await;
        connections.insert(uuid.clone(), connection);
        info!("Created VPN connection: {}", uuid);

        Ok(uuid)
    }

    /// Connect to a VPN
    pub async fn connect(&self, uuid: &str) -> NetctlResult<String> {
        let mut connections = self.connections.write().await;
        let connection = connections.get_mut(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("VPN connection {} not found", uuid)))?;

        info!("Connecting VPN: {}", uuid);
        let interface_name = connection.backend.connect(&connection.config).await?;
        connection.interface_name = Some(interface_name.clone());

        info!("VPN connected: {} (interface: {})", uuid, interface_name);
        Ok(interface_name)
    }

    /// Disconnect from a VPN
    pub async fn disconnect(&self, uuid: &str) -> NetctlResult<()> {
        let mut connections = self.connections.write().await;
        let connection = connections.get_mut(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("VPN connection {} not found", uuid)))?;

        info!("Disconnecting VPN: {}", uuid);
        connection.backend.disconnect().await?;
        connection.interface_name = None;

        info!("VPN disconnected: {}", uuid);
        Ok(())
    }

    /// Delete a VPN connection
    pub async fn delete_connection(&self, uuid: &str) -> NetctlResult<()> {
        let mut connections = self.connections.write().await;

        if let Some(mut connection) = connections.remove(uuid) {
            // Disconnect if connected
            if connection.backend.state().await != VpnState::Disconnected {
                warn!("Disconnecting VPN {} before deletion", uuid);
                let _ = connection.backend.disconnect().await;
            }
            info!("Deleted VPN connection: {}", uuid);
        }

        Ok(())
    }

    /// Get VPN connection state
    pub async fn get_state(&self, uuid: &str) -> NetctlResult<VpnState> {
        let connections = self.connections.read().await;
        let connection = connections.get(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("VPN connection {} not found", uuid)))?;

        Ok(connection.backend.state().await)
    }

    /// Get VPN connection statistics
    pub async fn get_stats(&self, uuid: &str) -> NetctlResult<VpnStats> {
        let connections = self.connections.read().await;
        let connection = connections.get(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("VPN connection {} not found", uuid)))?;

        connection.backend.stats().await
    }

    /// Get detailed status as JSON
    pub async fn get_status(&self, uuid: &str) -> NetctlResult<Value> {
        let connections = self.connections.read().await;
        let connection = connections.get(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("VPN connection {} not found", uuid)))?;

        connection.backend.status_json().await
    }

    /// List all VPN connections
    pub async fn list_connections(&self) -> Vec<String> {
        let connections = self.connections.read().await;
        connections.keys().cloned().collect()
    }

    /// Get connection configuration
    pub async fn get_config(&self, uuid: &str) -> NetctlResult<ConnectionConfig> {
        let connections = self.connections.read().await;
        let connection = connections.get(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("VPN connection {} not found", uuid)))?;

        Ok(connection.config.clone())
    }

    /// Update connection configuration (only allowed when disconnected)
    pub async fn update_config(&self, uuid: &str, new_config: ConnectionConfig) -> NetctlResult<()> {
        let mut connections = self.connections.write().await;
        let connection = connections.get_mut(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("VPN connection {} not found", uuid)))?;

        // Check if disconnected
        if connection.backend.state().await != VpnState::Disconnected {
            return Err(NetctlError::InvalidState(
                "Cannot update configuration while connected".to_string()
            ));
        }

        // Validate new configuration
        connection.backend.validate_config(&new_config).await?;

        connection.config = new_config;
        info!("Updated VPN connection configuration: {}", uuid);

        Ok(())
    }

    /// Import a VPN configuration file
    pub async fn import_config(&self, vpn_type: &str, path: &std::path::Path, name: String) -> NetctlResult<String> {
        let factory = self.backends.get(vpn_type)
            .ok_or_else(|| NetctlError::NotSupported(format!("VPN type '{}' not supported", vpn_type)))?;

        let backend = factory();

        // Import the configuration
        let settings = backend.import_config(path).await?;

        // Create a ConnectionConfig
        let uuid = uuid::Uuid::new_v4().to_string();
        let mut config = ConnectionConfig {
            uuid: uuid.clone(),
            name,
            conn_type: "vpn".to_string(),
            settings: HashMap::new(),
            autoconnect: false,
        };

        config.settings.insert("vpn_type".to_string(), Value::String(vpn_type.to_string()));
        for (key, value) in settings {
            config.settings.insert(key, value);
        }

        // Create the connection
        self.create_connection(config).await?;

        info!("Imported VPN configuration from {:?}: {}", path, uuid);
        Ok(uuid)
    }

    /// Export a VPN configuration file
    pub async fn export_config(&self, uuid: &str, path: &std::path::Path) -> NetctlResult<()> {
        let connections = self.connections.read().await;
        let connection = connections.get(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("VPN connection {} not found", uuid)))?;

        connection.backend.export_config(&connection.config, path).await?;

        info!("Exported VPN configuration to {:?}: {}", path, uuid);
        Ok(())
    }

    /// Get the interface name for a connected VPN
    pub async fn get_interface_name(&self, uuid: &str) -> NetctlResult<Option<String>> {
        let connections = self.connections.read().await;
        let connection = connections.get(uuid)
            .ok_or_else(|| NetctlError::NotFound(format!("VPN connection {} not found", uuid)))?;

        Ok(connection.backend.interface_name())
    }

    /// Disconnect all VPNs
    pub async fn disconnect_all(&self) -> NetctlResult<()> {
        let uuids: Vec<String> = {
            let connections = self.connections.read().await;
            connections.keys().cloned().collect()
        };

        for uuid in uuids {
            if let Err(e) = self.disconnect(&uuid).await {
                warn!("Failed to disconnect VPN {}: {}", uuid, e);
            }
        }

        Ok(())
    }
}

impl Drop for VpnManager {
    fn drop(&mut self) {
        // Note: async drop is not yet stable, so we can't properly disconnect here
        // The caller should call disconnect_all() before dropping
        debug!("VpnManager dropped");
    }
}
