//! Plugin manager - handles loading, lifecycle, and D-Bus exposure

use super::traits::{NetworkPlugin, PluginMetadata, PluginState, ConnectionConfig};
use crate::error::{NetctlError, NetctlResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

/// Plugin manager handles all loaded plugins
pub struct PluginManager {
    /// Loaded plugins by ID
    plugins: Arc<RwLock<HashMap<String, Box<dyn NetworkPlugin>>>>,
    /// Plugin metadata cache
    metadata_cache: Arc<RwLock<HashMap<String, PluginMetadata>>>,
    /// Plugin configurations directory
    config_dir: std::path::PathBuf,
    /// D-Bus connection (when feature enabled)
    #[cfg(feature = "dbus-nm")]
    dbus_conn: Option<Arc<zbus::Connection>>,
    /// NetworkManager D-Bus interface (when feature enabled)
    #[cfg(feature = "dbus-nm")]
    nm_dbus: Option<Arc<crate::dbus::NetworkManagerDBus>>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(config_dir: std::path::PathBuf) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            metadata_cache: Arc::new(RwLock::new(HashMap::new())),
            config_dir,
            #[cfg(feature = "dbus-nm")]
            dbus_conn: None,
            #[cfg(feature = "dbus-nm")]
            nm_dbus: None,
        }
    }

    /// Set the NetworkManager D-Bus interface
    #[cfg(feature = "dbus-nm")]
    pub fn set_dbus_interface(&mut self, nm_dbus: Arc<crate::dbus::NetworkManagerDBus>) {
        self.nm_dbus = Some(nm_dbus);
    }

    /// Initialize the plugin manager
    pub async fn initialize(&mut self) -> NetctlResult<()> {
        info!("Initializing plugin manager");

        // Create config directory if it doesn't exist
        tokio::fs::create_dir_all(&self.config_dir).await?;

        #[cfg(feature = "dbus-nm")]
        {
            // Initialize D-Bus connection
            match zbus::Connection::system().await {
                Ok(conn) => {
                    info!("Connected to system D-Bus");
                    self.dbus_conn = Some(Arc::new(conn));
                }
                Err(e) => {
                    warn!("Failed to connect to D-Bus: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Register a new plugin
    pub async fn register_plugin(&self, mut plugin: Box<dyn NetworkPlugin>) -> NetctlResult<()> {
        let metadata = plugin.metadata().clone();
        let plugin_id = metadata.id.clone();
        let plugin_name = metadata.name.clone();

        info!("Registering plugin: {} ({})", plugin_name, plugin_id);

        // Initialize the plugin
        plugin.initialize().await?;

        // Store plugin
        let mut plugins = self.plugins.write().await;
        let mut cache = self.metadata_cache.write().await;

        #[cfg(feature = "dbus-nm")]
        let dbus_service = metadata.dbus_service.clone();

        plugins.insert(plugin_id.clone(), plugin);
        cache.insert(plugin_id.clone(), metadata);

        #[cfg(feature = "dbus-nm")]
        if let Some(ref service) = dbus_service {
            if let Some(ref conn) = self.dbus_conn {
                let meta = cache.get(&plugin_id).unwrap();
                self.expose_plugin_on_dbus(conn.clone(), &plugin_id, service, meta).await?;
            }
        }

        Ok(())
    }

    /// Unregister a plugin
    pub async fn unregister_plugin(&self, plugin_id: &str) -> NetctlResult<()> {
        info!("Unregistering plugin: {}", plugin_id);

        let mut plugins = self.plugins.write().await;
        let mut cache = self.metadata_cache.write().await;

        if let Some(mut plugin) = plugins.remove(plugin_id) {
            plugin.shutdown().await?;
        }
        cache.remove(plugin_id);

        Ok(())
    }

    /// Get a plugin by ID
    pub async fn get_plugin(&self, plugin_id: &str) -> NetctlResult<()> {
        let plugins = self.plugins.read().await;
        if plugins.contains_key(plugin_id) {
            Ok(())
        } else {
            Err(NetctlError::NotFound(format!("Plugin '{}' not found", plugin_id)))
        }
    }

    /// List all registered plugins
    pub async fn list_plugins(&self) -> Vec<PluginMetadata> {
        let cache = self.metadata_cache.read().await;
        cache.values().cloned().collect()
    }

    /// Get plugin metadata
    pub async fn get_metadata(&self, plugin_id: &str) -> NetctlResult<PluginMetadata> {
        let cache = self.metadata_cache.read().await;
        cache.get(plugin_id)
            .cloned()
            .ok_or_else(|| NetctlError::NotFound(format!("Plugin '{}' not found", plugin_id)))
    }

    /// Create a connection using a specific plugin
    pub async fn create_connection(
        &self,
        plugin_id: &str,
        config: ConnectionConfig,
    ) -> NetctlResult<String> {
        let mut plugins = self.plugins.write().await;
        let plugin = plugins.get_mut(plugin_id)
            .ok_or_else(|| NetctlError::NotFound(format!("Plugin '{}' not found", plugin_id)))?;

        plugin.validate_config(&config).await?;
        let uuid = plugin.create_connection(config).await?;

        info!("Created connection {} using plugin {}", uuid, plugin_id);
        Ok(uuid)
    }

    /// Activate a connection
    pub async fn activate_connection(&self, plugin_id: &str, uuid: &str) -> NetctlResult<()> {
        let mut plugins = self.plugins.write().await;
        let plugin = plugins.get_mut(plugin_id)
            .ok_or_else(|| NetctlError::NotFound(format!("Plugin '{}' not found", plugin_id)))?;

        plugin.activate(uuid).await?;
        info!("Activated connection {} via plugin {}", uuid, plugin_id);

        // Emit D-Bus state changed signal
        #[cfg(feature = "dbus-nm")]
        if let Some(ref nm_dbus) = self.nm_dbus {
            let state: crate::dbus::DeviceState = plugin.state().into();
            let device_path = format!("/org/freedesktop/NetworkManager/Devices/{}", plugin_id);

            // Update device state
            if let Err(e) = nm_dbus.update_device_state(&device_path, state).await {
                warn!("Failed to update D-Bus device state: {}", e);
            }

            // Emit StateChanged signal
            if let Some(ref conn) = self.dbus_conn {
                if let Err(e) = crate::dbus::signals::emit_state_changed(conn, 70).await {
                    warn!("Failed to emit StateChanged signal: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Deactivate a connection
    pub async fn deactivate_connection(&self, plugin_id: &str, uuid: &str) -> NetctlResult<()> {
        let mut plugins = self.plugins.write().await;
        let plugin = plugins.get_mut(plugin_id)
            .ok_or_else(|| NetctlError::NotFound(format!("Plugin '{}' not found", plugin_id)))?;

        plugin.deactivate(uuid).await?;
        info!("Deactivated connection {} via plugin {}", uuid, plugin_id);

        // Emit D-Bus state changed signal
        #[cfg(feature = "dbus-nm")]
        if let Some(ref nm_dbus) = self.nm_dbus {
            let state: crate::dbus::DeviceState = plugin.state().into();
            let device_path = format!("/org/freedesktop/NetworkManager/Devices/{}", plugin_id);

            // Update device state
            if let Err(e) = nm_dbus.update_device_state(&device_path, state).await {
                warn!("Failed to update D-Bus device state: {}", e);
            }

            // Emit StateChanged signal
            if let Some(ref conn) = self.dbus_conn {
                if let Err(e) = crate::dbus::signals::emit_state_changed(conn, 30).await {
                    warn!("Failed to emit StateChanged signal: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Get connection status
    pub async fn get_connection_status(&self, plugin_id: &str, uuid: &str) -> NetctlResult<PluginState> {
        let plugins = self.plugins.read().await;
        let plugin = plugins.get(plugin_id)
            .ok_or_else(|| NetctlError::NotFound(format!("Plugin '{}' not found", plugin_id)))?;

        plugin.get_status(uuid).await
    }

    /// Enable a plugin
    pub async fn enable_plugin(&self, plugin_id: &str) -> NetctlResult<()> {
        let mut plugins = self.plugins.write().await;
        let plugin = plugins.get_mut(plugin_id)
            .ok_or_else(|| NetctlError::NotFound(format!("Plugin '{}' not found", plugin_id)))?;

        plugin.enable().await?;
        info!("Enabled plugin: {}", plugin_id);
        Ok(())
    }

    /// Disable a plugin
    pub async fn disable_plugin(&self, plugin_id: &str) -> NetctlResult<()> {
        let mut plugins = self.plugins.write().await;
        let plugin = plugins.get_mut(plugin_id)
            .ok_or_else(|| NetctlError::NotFound(format!("Plugin '{}' not found", plugin_id)))?;

        plugin.disable().await?;
        info!("Disabled plugin: {}", plugin_id);
        Ok(())
    }

    /// D-Bus: Expose plugin on D-Bus
    #[cfg(feature = "dbus-nm")]
    async fn expose_plugin_on_dbus(
        &self,
        conn: Arc<zbus::Connection>,
        plugin_id: &str,
        service_name: &str,
        metadata: &PluginMetadata,
    ) -> NetctlResult<()> {
        info!("Exposing plugin {} on D-Bus as {}", plugin_id, service_name);

        // D-Bus interface would be implemented here
        // This requires creating a D-Bus interface struct that implements zbus::dbus_interface
        // For now, this is a placeholder

        Ok(())
    }

    /// D-Bus: Handle method call for a plugin
    #[cfg(feature = "dbus-nm")]
    pub async fn handle_plugin_dbus_method(
        &self,
        plugin_id: &str,
        method: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> NetctlResult<serde_json::Value> {
        let mut plugins = self.plugins.write().await;
        let plugin = plugins.get_mut(plugin_id)
            .ok_or_else(|| NetctlError::NotFound(format!("Plugin '{}' not found", plugin_id)))?;

        plugin.handle_dbus_method(method, params).await
    }

    /// Shutdown all plugins
    pub async fn shutdown(&self) -> NetctlResult<()> {
        info!("Shutting down plugin manager");

        let mut plugins = self.plugins.write().await;
        for (id, mut plugin) in plugins.drain() {
            if let Err(e) = plugin.shutdown().await {
                error!("Failed to shutdown plugin {}: {}", id, e);
            }
        }

        Ok(())
    }
}

impl Drop for PluginManager {
    fn drop(&mut self) {
        // Async drop is not available, so we can't await shutdown here
        // This should be called explicitly before dropping
    }
}
