//! Connection manager - orchestrates WiFi, DHCP, and network configuration
//!
//! This module reads .nctl configuration files and manages the complete
//! connection lifecycle including WiFi association, DHCP client control,
//! and static IP configuration.

use crate::error::{NetctlError, NetctlResult};
use crate::connection_config::{ConnectionConfigManager, NetctlConnectionConfig};
use crate::interface::InterfaceController;
use crate::wpa_supplicant::WpaSupplicantController;
use crate::dhcp_client::DhcpClientController;
use crate::vpn::{VpnManager, wireguard, openvpn};
use std::collections::HashMap;
use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Active connection state
#[derive(Debug, Clone, serde::Serialize)]
pub struct ActiveConnection {
    /// Connection name
    pub name: String,
    /// Connection UUID
    pub uuid: String,
    /// Interface name
    pub interface: String,
    /// Connection type (wifi, ethernet, vpn)
    pub conn_type: String,
    /// Whether DHCP is running
    pub dhcp_active: bool,
    /// Configuration
    pub config: NetctlConnectionConfig,
}

/// Connection manager
pub struct ConnectionManager {
    /// Configuration manager
    config_manager: ConnectionConfigManager,
    /// Interface controller
    interface_controller: Arc<InterfaceController>,
    /// WPA Supplicant controller
    wpa_supplicant: Arc<WpaSupplicantController>,
    /// DHCP client controller
    dhcp_client: Arc<DhcpClientController>,
    /// VPN manager
    vpn_manager: Arc<VpnManager>,
    /// Active connections (interface/uuid -> connection)
    active_connections: Arc<RwLock<HashMap<String, ActiveConnection>>>,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new(config_dir: Option<&str>) -> Self {
        let config_manager = if let Some(dir) = config_dir {
            ConnectionConfigManager::new(dir)
        } else {
            ConnectionConfigManager::default()
        };

        // Initialize VPN manager with backends
        let mut vpn_manager = VpnManager::new(PathBuf::from("/etc/netctl"));
        vpn_manager.register_backend("wireguard", wireguard::create_backend);
        vpn_manager.register_backend("openvpn", openvpn::create_backend);

        Self {
            config_manager,
            interface_controller: Arc::new(InterfaceController::new()),
            wpa_supplicant: Arc::new(WpaSupplicantController::new()),
            dhcp_client: Arc::new(DhcpClientController::new()),
            vpn_manager: Arc::new(vpn_manager),
            active_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize the connection manager
    pub async fn initialize(&self) -> NetctlResult<()> {
        info!("Initializing connection manager");
        self.config_manager.initialize().await?;
        Ok(())
    }

    /// List available connection configurations
    pub async fn list_connections(&self) -> NetctlResult<Vec<String>> {
        self.config_manager.list_configs().await
    }

    /// Load a connection configuration
    pub async fn load_connection(&self, name: &str) -> NetctlResult<NetctlConnectionConfig> {
        self.config_manager.load_config(name).await
    }

    /// Activate a connection by name
    pub async fn activate_connection(&self, name: &str) -> NetctlResult<()> {
        info!("Activating connection: {}", name);

        // Load configuration
        let config = self.config_manager.load_config(name).await?;

        // Determine interface
        let interface = config.connection.interface_name.clone()
            .ok_or_else(|| NetctlError::ConfigError(
                "Connection must specify interface-name".to_string()
            ))?;

        info!("Activating connection '{}' on interface {}", name, &interface);

        // Check if already active
        {
            let active = self.active_connections.read().await;
            if active.contains_key(&interface) {
                warn!("Interface {} already has an active connection", interface);
                return Err(NetctlError::AlreadyExists(
                    format!("Connection already active on {}", interface)
                ));
            }
        }

        // Bring interface up
        info!("Bringing interface {} up", interface);
        self.interface_controller.up(&interface).await?;

        // Handle WiFi connection
        if config.connection.conn_type == "wifi" {
            self.activate_wifi(&config, &interface).await?;
        }

        // Handle VPN connection
        if config.connection.conn_type == "vpn" {
            return self.activate_vpn(name, &config).await;
        }

        // Handle IP configuration
        let dhcp_active = self.configure_ip(&config, &interface).await?;

        // Store active connection
        let active_conn = ActiveConnection {
            name: name.to_string(),
            uuid: config.connection.uuid.clone(),
            interface: interface.clone(),
            conn_type: config.connection.conn_type.clone(),
            dhcp_active,
            config,
        };

        self.active_connections.write().await.insert(
            interface.clone(),
            active_conn
        );

        info!("Connection '{}' activated successfully on {}", name, interface);
        Ok(())
    }

    /// Activate WiFi connection
    async fn activate_wifi(&self, config: &NetctlConnectionConfig, interface: &str) -> NetctlResult<()> {
        let wifi = config.wifi.as_ref()
            .ok_or_else(|| NetctlError::ConfigError(
                "WiFi connection must have [wifi] section".to_string()
            ))?;

        info!("Connecting to WiFi network '{}' on {}", wifi.ssid, interface);

        // Get password from wifi-security section
        let password = config.wifi_security.as_ref()
            .and_then(|sec| sec.psk.as_ref().or(sec.password.as_ref()))
            .map(|s| s.as_str());

        // Connect to WiFi
        self.wpa_supplicant.connect(interface, &wifi.ssid, password).await?;

        info!("WiFi connected: {}", wifi.ssid);
        Ok(())
    }

    /// Configure IP (DHCP or static)
    async fn configure_ip(&self, config: &NetctlConnectionConfig, interface: &str) -> NetctlResult<bool> {
        let mut dhcp_active = false;

        // Check IPv4 configuration
        if let Some(ipv4) = &config.ipv4 {
            match ipv4.method.as_str() {
                "auto" => {
                    info!("Starting DHCP client on {} (IPv4 method: auto)", interface);

                    // Wait a moment for link to be fully up (especially for WiFi)
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                    match self.dhcp_client.start(interface).await {
                        Ok(()) => {
                            info!("DHCP client started successfully on {}", interface);
                            dhcp_active = true;
                        }
                        Err(e) => {
                            error!("Failed to start DHCP client on {}: {}", interface, e);
                            // Don't fail the entire connection if DHCP fails
                            warn!("Continuing without DHCP on {}", interface);
                        }
                    }
                }
                "manual" => {
                    info!("Configuring static IP on {} (IPv4 method: manual)", interface);

                    if let Some(address) = &ipv4.address {
                        // Parse address/prefix (e.g., "192.168.1.100/24")
                        let parts: Vec<&str> = address.split('/').collect();
                        if parts.len() == 2 {
                            let ip = parts[0];
                            if let Ok(prefix) = parts[1].parse::<u8>() {
                                debug!("Setting IP address {}/{} on {}", ip, prefix, interface);
                                self.interface_controller.set_ip(interface, ip, prefix).await?;
                            } else {
                                return Err(NetctlError::ConfigError(
                                    format!("Invalid prefix length: {}", parts[1])
                                ));
                            }
                        } else {
                            return Err(NetctlError::ConfigError(
                                format!("Invalid IP address format: {}", address)
                            ));
                        }
                    }

                    // TODO: Configure gateway and DNS
                    if let Some(gateway) = &ipv4.gateway {
                        debug!("Gateway configured: {}", gateway);
                        // self.routing_controller.add_default_route(gateway, interface).await?;
                    }
                }
                "ignore" => {
                    info!("IPv4 method is 'ignore', skipping IP configuration");
                }
                "link-local" => {
                    info!("IPv4 method is 'link-local', relying on auto-configuration");
                }
                method => {
                    warn!("Unknown IPv4 method '{}', treating as ignore", method);
                }
            }
        } else {
            // Default to DHCP if no IPv4 config specified
            info!("No IPv4 config specified, defaulting to DHCP on {}", interface);

            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            if let Err(e) = self.dhcp_client.start(interface).await {
                warn!("Failed to start DHCP client on {}: {}", interface, e);
            } else {
                dhcp_active = true;
            }
        }

        Ok(dhcp_active)
    }

    /// Activate VPN connection
    async fn activate_vpn(&self, name: &str, config: &NetctlConnectionConfig) -> NetctlResult<()> {
        let vpn = config.vpn.as_ref()
            .ok_or_else(|| NetctlError::ConfigError(
                "VPN connection must have [vpn] section".to_string()
            ))?;

        info!("Activating VPN connection '{}' (type: {})", name, vpn.connection_type);

        // Convert to plugin ConnectionConfig format
        let conn_config = config.to_plugin_config();

        // Create VPN connection
        let uuid = self.vpn_manager.create_connection(conn_config).await?;

        // Connect VPN
        let interface = self.vpn_manager.connect(&uuid).await?;

        info!("VPN connected on interface: {}", interface);

        // Store active connection
        let active_conn = ActiveConnection {
            name: name.to_string(),
            uuid: config.connection.uuid.clone(),
            interface: interface.clone(),
            conn_type: "vpn".to_string(),
            dhcp_active: false,
            config: config.clone(),
        };

        self.active_connections.write().await.insert(
            interface.clone(),
            active_conn
        );

        info!("VPN connection '{}' activated successfully on {}", name, interface);
        Ok(())
    }

    /// Deactivate a connection on an interface
    pub async fn deactivate_connection(&self, interface: &str) -> NetctlResult<()> {
        info!("Deactivating connection on interface {}", interface);

        // Get active connection
        let active_conn = {
            let mut active = self.active_connections.write().await;
            active.remove(interface)
        };

        if let Some(conn) = active_conn {
            // Stop DHCP if active
            if conn.dhcp_active {
                info!("Stopping DHCP client on {}", interface);
                if let Err(e) = self.dhcp_client.release(interface).await {
                    warn!("Failed to release DHCP lease on {}: {}", interface, e);
                }
                if let Err(e) = self.dhcp_client.stop(interface).await {
                    warn!("Failed to stop DHCP client on {}: {}", interface, e);
                }
            }

            // Disconnect WiFi if it's a WiFi connection
            if conn.conn_type == "wifi" {
                info!("Disconnecting WiFi on {}", interface);
                if let Err(e) = self.wpa_supplicant.disconnect(interface).await {
                    warn!("Failed to disconnect WiFi on {}: {}", interface, e);
                }
            }

            // Disconnect VPN if it's a VPN connection
            if conn.conn_type == "vpn" {
                info!("Disconnecting VPN on {}", interface);
                if let Err(e) = self.vpn_manager.disconnect(&conn.uuid).await {
                    warn!("Failed to disconnect VPN {}: {}", conn.uuid, e);
                }
            }

            // Bring interface down
            info!("Bringing interface {} down", interface);
            self.interface_controller.down(interface).await?;

            info!("Connection '{}' deactivated on {}", conn.name, interface);
        } else {
            warn!("No active connection found on interface {}", interface);
        }

        Ok(())
    }

    /// Get active connection on an interface
    pub async fn get_active_connection(&self, interface: &str) -> Option<ActiveConnection> {
        self.active_connections.read().await.get(interface).cloned()
    }

    /// List all active connections
    pub async fn list_active_connections(&self) -> Vec<ActiveConnection> {
        self.active_connections.read().await.values().cloned().collect()
    }

    /// Auto-connect all connections marked with autoconnect=true
    pub async fn auto_connect(&self) -> NetctlResult<()> {
        info!("Auto-connecting configured connections");

        let configs = self.list_connections().await?;

        for name in configs {
            let config = match self.load_connection(&name).await {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to load connection '{}': {}", name, e);
                    continue;
                }
            };

            if config.connection.autoconnect {
                info!("Auto-connecting: {}", name);
                if let Err(e) = self.activate_connection(&name).await {
                    error!("Failed to activate connection '{}': {}", name, e);
                    // Continue with next connection
                }
            }
        }

        Ok(())
    }

    /// Save a new connection configuration
    pub async fn save_connection(
        &self,
        name: &str,
        config: &NetctlConnectionConfig
    ) -> NetctlResult<()> {
        self.config_manager.save_config(name, config).await
    }

    /// Delete a connection configuration
    pub async fn delete_connection(&self, name: &str) -> NetctlResult<()> {
        self.config_manager.delete_config(name).await
    }

    /// Get interface controller reference
    pub fn interface_controller(&self) -> Arc<InterfaceController> {
        self.interface_controller.clone()
    }

    /// Get DHCP client controller reference
    pub fn dhcp_client(&self) -> Arc<DhcpClientController> {
        self.dhcp_client.clone()
    }

    /// Get WPA supplicant controller reference
    pub fn wpa_supplicant(&self) -> Arc<WpaSupplicantController> {
        self.wpa_supplicant.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_manager_creation() {
        let manager = ConnectionManager::new(None);
        assert!(manager.interface_controller.list().await.is_ok());
    }
}
