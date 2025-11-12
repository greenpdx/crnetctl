//! Dynamic plugin loader - loads plugins from shared libraries (.so files)

use super::traits::{NetworkPlugin, PluginFactory};
use crate::error::{NetctlError, NetctlResult};
use std::path::{Path, PathBuf};
use tracing::{info, warn, error};

/// Plugin loader handles dynamic loading of plugin modules
pub struct PluginLoader {
    /// Directory to search for plugin modules
    plugin_dirs: Vec<PathBuf>,
    /// Loaded plugin libraries (keeps them in memory)
    #[allow(dead_code)]
    libraries: Vec<libloading::Library>,
}

/// Plugin module entry point signature
/// Each plugin .so must export a function:
/// ```
/// #[no_mangle]
/// pub extern "C" fn netctl_plugin_create() -> *mut Box<dyn NetworkPlugin> {
///     Box::into_raw(Box::new(Box::new(MyPlugin::new())))
/// }
/// ```
type PluginCreateFn = unsafe extern "C" fn() -> *mut Box<dyn NetworkPlugin>;

impl PluginLoader {
    /// Create a new plugin loader
    pub fn new() -> Self {
        let plugin_dirs = vec![
            PathBuf::from("/usr/lib/netctl/plugins"),
            PathBuf::from("/usr/local/lib/netctl/plugins"),
            PathBuf::from("./plugins"),
        ];

        Self {
            plugin_dirs,
            libraries: Vec::new(),
        }
    }

    /// Add a plugin search directory
    pub fn add_plugin_dir<P: AsRef<Path>>(&mut self, dir: P) {
        self.plugin_dirs.push(dir.as_ref().to_path_buf());
    }

    /// Discover all available plugins in search directories
    pub async fn discover_plugins(&self) -> Vec<PathBuf> {
        let mut plugins = Vec::new();

        for dir in &self.plugin_dirs {
            if !dir.exists() {
                continue;
            }

            info!("Searching for plugins in: {}", dir.display());

            match tokio::fs::read_dir(dir).await {
                Ok(mut entries) => {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let path = entry.path();

                        // Look for .so files
                        if let Some(ext) = path.extension() {
                            if ext == "so" {
                                info!("Found plugin module: {}", path.display());
                                plugins.push(path);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to read plugin directory {}: {}", dir.display(), e);
                }
            }
        }

        plugins
    }

    /// Load a plugin from a shared library file
    pub unsafe fn load_plugin<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> NetctlResult<Box<dyn NetworkPlugin>> {
        let path = path.as_ref();
        info!("Loading plugin from: {}", path.display());

        // Load the library
        let lib = libloading::Library::new(path)
            .map_err(|e| NetctlError::ServiceError(format!("Failed to load plugin: {}", e)))?;

        // Get the plugin creation function
        let create_fn: libloading::Symbol<PluginCreateFn> = lib
            .get(b"netctl_plugin_create")
            .map_err(|e| NetctlError::ServiceError(
                format!("Plugin missing 'netctl_plugin_create' function: {}", e)
            ))?;

        // Create the plugin instance
        let plugin_ptr = create_fn();
        if plugin_ptr.is_null() {
            return Err(NetctlError::ServiceError("Plugin creation returned null".to_string()));
        }

        let plugin = *Box::from_raw(plugin_ptr);

        // Keep the library loaded
        self.libraries.push(lib);

        info!("Successfully loaded plugin: {}", plugin.metadata().name);
        Ok(plugin)
    }

    /// Load all discovered plugins
    pub async fn load_all_plugins(&mut self) -> Vec<Box<dyn NetworkPlugin>> {
        let plugin_paths = self.discover_plugins().await;
        let mut plugins = Vec::new();

        for path in plugin_paths {
            match unsafe { self.load_plugin(&path) } {
                Ok(plugin) => {
                    plugins.push(plugin);
                }
                Err(e) => {
                    error!("Failed to load plugin from {}: {}", path.display(), e);
                }
            }
        }

        plugins
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}
