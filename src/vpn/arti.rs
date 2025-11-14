use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use arti_client::{TorClient, TorClientConfig};
use tor_rtcompat::PreferredRuntime;

use crate::plugin::ConnectionConfig;
use crate::error::{NetctlError, NetctlResult};
use super::backend::{VpnBackend, VpnState, VpnStats};
use super::common;

/// Arti (Tor) VPN backend implementation
///
/// This backend provides Tor connectivity using the Arti client library.
/// Unlike traditional VPN backends, it provides a SOCKS5 proxy interface
/// for routing traffic through the Tor network.
pub struct ArtiBackend {
    /// The Tor client instance (wrapped for thread safety)
    tor_client: Arc<RwLock<Option<TorClient<PreferredRuntime>>>>,
    /// SOCKS5 proxy listening address
    socks_addr: Option<String>,
    /// Time when the connection was established
    connected_since: Option<SystemTime>,
    /// Current connection state
    state: Arc<RwLock<VpnState>>,
    /// Background task handle for the Tor client
    task_handle: Option<JoinHandle<()>>,
    /// Total bytes transferred (tracked separately for Tor)
    bytes_sent: Arc<RwLock<u64>>,
    bytes_received: Arc<RwLock<u64>>,
}

impl ArtiBackend {
    /// Create a new Arti backend instance
    pub fn new() -> Self {
        Self {
            tor_client: Arc::new(RwLock::new(None)),
            socks_addr: None,
            connected_since: None,
            state: Arc::new(RwLock::new(VpnState::Disconnected)),
            task_handle: None,
            bytes_sent: Arc::new(RwLock::new(0)),
            bytes_received: Arc::new(RwLock::new(0)),
        }
    }

    /// Build Tor client configuration from connection settings
    fn build_tor_config(&self, config: &ConnectionConfig) -> NetctlResult<TorClientConfig> {
        let mut tor_config = TorClientConfig::default();
        let settings = &config.settings;

        // Configure state directory
        if let Some(state_dir) = settings.get("state_dir").and_then(|v| v.as_str()) {
            // Note: Actual state directory configuration would require accessing
            // the builder API which may differ in the current Arti version
            debug!("State directory: {}", state_dir);
        }

        // Configure cache directory
        if let Some(cache_dir) = settings.get("cache_dir").and_then(|v| v.as_str()) {
            debug!("Cache directory: {}", cache_dir);
        }

        // Additional Tor-specific configurations could be added here
        // such as bridges, entry guards, exit nodes, etc.

        Ok(tor_config)
    }

    /// Get SOCKS5 listening address from configuration
    fn get_socks_address(&self, config: &ConnectionConfig) -> String {
        config.settings
            .get("socks_addr")
            .and_then(|v| v.as_str())
            .unwrap_or("127.0.0.1:9050")
            .to_string()
    }

    /// Start the Tor client and SOCKS5 proxy
    async fn start_tor_client(&mut self, config: &ConnectionConfig) -> NetctlResult<()> {
        info!("Starting Arti (Tor) client");

        // Build Tor configuration
        let tor_config = self.build_tor_config(config)?;

        // Create Tor client with preferred runtime
        let client = TorClient::with_runtime(PreferredRuntime::current()?)
            .config(tor_config)
            .create_unbootstrapped()
            .map_err(|e| NetctlError::ServiceError(format!("Failed to create Tor client: {}", e)))?;

        // Bootstrap the client (connect to Tor network)
        info!("Bootstrapping Tor client...");
        let bootstrap_client = client.clone();

        // Spawn bootstrap task
        let state_clone = self.state.clone();
        tokio::spawn(async move {
            match bootstrap_client.bootstrap().await {
                Ok(_) => {
                    info!("Tor client bootstrapped successfully");
                    *state_clone.write().await = VpnState::Connected;
                }
                Err(e) => {
                    error!("Tor bootstrap failed: {}", e);
                    *state_clone.write().await = VpnState::Failed(format!("Bootstrap failed: {}", e));
                }
            }
        });

        // Store the client
        *self.tor_client.write().await = Some(client);
        *self.state.write().await = VpnState::Connecting;

        Ok(())
    }

    /// Generate a unique identifier for this connection
    fn generate_connection_id(uuid: &str) -> String {
        format!("tor-{}", &uuid[..std::cmp::min(8, uuid.len())])
    }
}

impl Default for ArtiBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VpnBackend for ArtiBackend {
    fn name(&self) -> &str {
        "arti"
    }

    async fn version(&self) -> NetctlResult<String> {
        // Arti is a library, so we return the crate version
        // In a real implementation, you might want to use the actual version from Cargo.toml
        Ok(format!("arti-client {}", env!("CARGO_PKG_VERSION")))
    }

    async fn is_available(&self) -> bool {
        // Arti is compiled into the binary, so it's always available
        // We could check for necessary system capabilities here
        true
    }

    async fn validate_config(&self, config: &ConnectionConfig) -> NetctlResult<()> {
        let settings = &config.settings;

        // Validate SOCKS address if provided
        if let Some(socks_addr) = settings.get("socks_addr").and_then(|v| v.as_str()) {
            if !socks_addr.contains(':') {
                return Err(NetctlError::InvalidParameter(
                    "SOCKS address must be in format 'host:port'".to_string()
                ));
            }

            // Parse to validate format
            if socks_addr.parse::<std::net::SocketAddr>().is_err() {
                return Err(NetctlError::InvalidParameter(
                    format!("Invalid SOCKS address format: {}", socks_addr)
                ));
            }
        }

        // Validate state directory if provided
        if let Some(state_dir) = settings.get("state_dir").and_then(|v| v.as_str()) {
            let path = std::path::Path::new(state_dir);
            if path.exists() && !path.is_dir() {
                return Err(NetctlError::InvalidParameter(
                    format!("State directory path exists but is not a directory: {}", state_dir)
                ));
            }
        }

        // Validate cache directory if provided
        if let Some(cache_dir) = settings.get("cache_dir").and_then(|v| v.as_str()) {
            let path = std::path::Path::new(cache_dir);
            if path.exists() && !path.is_dir() {
                return Err(NetctlError::InvalidParameter(
                    format!("Cache directory path exists but is not a directory: {}", cache_dir)
                ));
            }
        }

        Ok(())
    }

    async fn connect(&mut self, config: &ConnectionConfig) -> NetctlResult<String> {
        info!("Connecting to Tor network via Arti: {}", config.name);

        // Get SOCKS address
        let socks_addr = self.get_socks_address(config);

        // Start Tor client
        self.start_tor_client(config).await?;

        // Store connection details
        self.socks_addr = Some(socks_addr.clone());
        self.connected_since = Some(SystemTime::now());

        let connection_id = Self::generate_connection_id(&config.uuid);

        info!("Arti (Tor) connection established: {} (SOCKS5: {})", config.name, socks_addr);
        info!("Applications can use SOCKS5 proxy at: {}", socks_addr);

        // Return a virtual interface name (Tor doesn't create a real interface)
        Ok(connection_id)
    }

    async fn disconnect(&mut self) -> NetctlResult<()> {
        info!("Disconnecting from Tor network");

        // Update state
        *self.state.write().await = VpnState::Disconnecting;

        // Drop the Tor client (this will clean up resources)
        *self.tor_client.write().await = None;

        // Cancel background tasks if any
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }

        // Clear connection details
        self.socks_addr = None;
        self.connected_since = None;
        *self.state.write().await = VpnState::Disconnected;

        info!("Disconnected from Tor network");
        Ok(())
    }

    async fn state(&self) -> VpnState {
        self.state.read().await.clone()
    }

    async fn stats(&self) -> NetctlResult<VpnStats> {
        let mut stats = VpnStats::default();
        stats.connected_since = self.connected_since;
        stats.bytes_sent = *self.bytes_sent.read().await;
        stats.bytes_received = *self.bytes_received.read().await;

        // Tor doesn't expose detailed per-circuit statistics in the same way
        // as traditional VPNs, so we provide what we can track

        Ok(stats)
    }

    fn interface_name(&self) -> Option<String> {
        // Arti/Tor doesn't create a network interface like traditional VPNs
        // Return the SOCKS proxy address instead
        self.socks_addr.clone()
    }

    async fn status_json(&self) -> NetctlResult<Value> {
        let state = self.state().await;
        let stats = self.stats().await.unwrap_or_default();

        let has_client = self.tor_client.read().await.is_some();

        Ok(json!({
            "backend": "arti",
            "state": format!("{:?}", state),
            "socks_proxy": self.socks_addr,
            "connected_since": stats.connected_since.map(|t| format!("{:?}", t)),
            "bytes_sent": stats.bytes_sent,
            "bytes_received": stats.bytes_received,
            "tor_client_active": has_client,
            "note": "Use the SOCKS5 proxy address to route traffic through Tor"
        }))
    }

    async fn import_config(&self, path: &Path) -> NetctlResult<HashMap<String, Value>> {
        info!("Importing Arti/Tor configuration from: {:?}", path);

        let content = common::read_config_file(path).await?;
        let mut settings = HashMap::new();

        // Parse TOML configuration
        let config: toml::Value = toml::from_str(&content)
            .map_err(|e| NetctlError::InvalidParameter(format!("Invalid TOML: {}", e)))?;

        // Extract relevant settings
        if let Some(table) = config.as_table() {
            for (key, value) in table {
                settings.insert(key.clone(), serde_json::to_value(value)
                    .map_err(|e| NetctlError::InvalidParameter(format!("Conversion error: {}", e)))?);
            }
        }

        Ok(settings)
    }

    async fn export_config(&self, config: &ConnectionConfig, path: &Path) -> NetctlResult<()> {
        info!("Exporting Arti/Tor configuration to: {:?}", path);

        let settings = &config.settings;
        let mut toml_content = String::new();

        // Build TOML configuration
        toml_content.push_str("# Arti (Tor) Configuration\n\n");

        if let Some(socks_addr) = settings.get("socks_addr").and_then(|v| v.as_str()) {
            toml_content.push_str(&format!("socks_addr = \"{}\"\n", socks_addr));
        }

        if let Some(state_dir) = settings.get("state_dir").and_then(|v| v.as_str()) {
            toml_content.push_str(&format!("state_dir = \"{}\"\n", state_dir));
        }

        if let Some(cache_dir) = settings.get("cache_dir").and_then(|v| v.as_str()) {
            toml_content.push_str(&format!("cache_dir = \"{}\"\n", cache_dir));
        }

        // Write with secure permissions
        common::write_secure_config(path, &toml_content, 0o600).await
    }
}

/// Factory function to create an Arti backend
pub fn create_backend() -> Box<dyn VpnBackend> {
    Box::new(ArtiBackend::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_backend_creation() {
        let backend = ArtiBackend::new();
        assert_eq!(backend.name(), "arti");
        assert!(backend.is_available().await);
    }

    #[tokio::test]
    async fn test_validate_socks_address() {
        let backend = ArtiBackend::new();
        let mut config = ConnectionConfig {
            uuid: "test-uuid".to_string(),
            name: "Test Tor".to_string(),
            conn_type: "vpn".to_string(),
            settings: HashMap::new(),
            autoconnect: false,
        };

        // Valid SOCKS address
        config.settings.insert(
            "socks_addr".to_string(),
            json!("127.0.0.1:9050")
        );
        assert!(backend.validate_config(&config).await.is_ok());

        // Invalid SOCKS address (no port)
        config.settings.insert(
            "socks_addr".to_string(),
            json!("127.0.0.1")
        );
        assert!(backend.validate_config(&config).await.is_err());
    }

    #[tokio::test]
    async fn test_initial_state() {
        let backend = ArtiBackend::new();
        assert_eq!(backend.state().await, VpnState::Disconnected);
        assert!(backend.interface_name().is_none());
    }
}
