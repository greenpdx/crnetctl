//! Plugin configuration management

use crate::error::{NetctlError, NetctlResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{info, warn};

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Plugin ID
    pub id: String,
    /// Whether the plugin is enabled
    pub enabled: bool,
    /// Plugin-specific settings
    pub settings: HashMap<String, serde_json::Value>,
    /// Auto-load on startup
    pub autoload: bool,
    /// Plugin library path (for dynamic plugins)
    pub library_path: Option<PathBuf>,
}

/// Plugin configuration manager
pub struct PluginConfigManager {
    /// Configuration directory
    config_dir: PathBuf,
    /// Loaded configurations
    configs: HashMap<String, PluginConfig>,
}

impl PluginConfigManager {
    /// Create a new plugin configuration manager
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            config_dir,
            configs: HashMap::new(),
        }
    }

    /// Initialize the configuration manager
    pub async fn initialize(&mut self) -> NetctlResult<()> {
        info!("Initializing plugin configuration manager");

        // Create config directory if it doesn't exist
        fs::create_dir_all(&self.config_dir).await?;

        // Load all plugin configurations
        self.load_all_configs().await?;

        Ok(())
    }

    /// Load all plugin configurations from directory
    async fn load_all_configs(&mut self) -> NetctlResult<()> {
        let mut entries = fs::read_dir(&self.config_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Look for .toml files
            if let Some(ext) = path.extension() {
                if ext == "toml" {
                    match self.load_config_file(&path).await {
                        Ok(config) => {
                            info!("Loaded plugin config: {}", config.id);
                            self.configs.insert(config.id.clone(), config);
                        }
                        Err(e) => {
                            warn!("Failed to load config {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Load a plugin configuration from file
    async fn load_config_file(&self, path: &Path) -> NetctlResult<PluginConfig> {
        let contents = fs::read_to_string(path).await?;
        let config: PluginConfig = toml::from_str(&contents)
            .map_err(|e| NetctlError::InvalidParameter(format!("Invalid TOML: {}", e)))?;
        Ok(config)
    }

    /// Save a plugin configuration to file
    pub async fn save_config(&mut self, config: &PluginConfig) -> NetctlResult<()> {
        let filename = format!("{}.toml", config.id);
        let path = self.config_dir.join(filename);

        let toml_str = toml::to_string_pretty(config)
            .map_err(|e| NetctlError::ServiceError(format!("Failed to serialize config: {}", e)))?;

        fs::write(&path, toml_str).await?;

        self.configs.insert(config.id.clone(), config.clone());
        info!("Saved plugin config: {}", config.id);

        Ok(())
    }

    /// Get a plugin configuration
    pub fn get_config(&self, plugin_id: &str) -> Option<&PluginConfig> {
        self.configs.get(plugin_id)
    }

    /// Get all plugin configurations
    pub fn get_all_configs(&self) -> Vec<&PluginConfig> {
        self.configs.values().collect()
    }

    /// Get all autoload-enabled plugins
    pub fn get_autoload_plugins(&self) -> Vec<&PluginConfig> {
        self.configs
            .values()
            .filter(|c| c.autoload && c.enabled)
            .collect()
    }

    /// Update plugin configuration
    pub async fn update_config(
        &mut self,
        plugin_id: &str,
        update_fn: impl FnOnce(&mut PluginConfig),
    ) -> NetctlResult<()> {
        // Get and update the config
        {
            let config = self.configs.get_mut(plugin_id)
                .ok_or_else(|| NetctlError::NotFound(format!("Plugin config '{}' not found", plugin_id)))?;
            update_fn(config);
        }

        // Save the updated config
        let config = self.configs.get(plugin_id).unwrap().clone();
        self.save_config(&config).await?;

        Ok(())
    }

    /// Enable a plugin
    pub async fn enable_plugin(&mut self, plugin_id: &str) -> NetctlResult<()> {
        self.update_config(plugin_id, |config| {
            config.enabled = true;
        }).await
    }

    /// Disable a plugin
    pub async fn disable_plugin(&mut self, plugin_id: &str) -> NetctlResult<()> {
        self.update_config(plugin_id, |config| {
            config.enabled = false;
        }).await
    }

    /// Delete a plugin configuration
    pub async fn delete_config(&mut self, plugin_id: &str) -> NetctlResult<()> {
        let filename = format!("{}.toml", plugin_id);
        let path = self.config_dir.join(filename);

        if path.exists() {
            fs::remove_file(&path).await?;
        }

        self.configs.remove(plugin_id);
        info!("Deleted plugin config: {}", plugin_id);

        Ok(())
    }
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            enabled: true,
            settings: HashMap::new(),
            autoload: false,
            library_path: None,
        }
    }
}
