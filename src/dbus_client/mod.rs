//! D-Bus Client Library for Network Control
//!
//! This module provides a Rust client API for communicating with the netctld daemon
//! via D-Bus. It wraps all the CR D-Bus interfaces with convenient Rust methods.
//!
//! # Usage
//!
//! ```rust,no_run
//! use libnetctl::dbus_client::NetctlClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to the daemon
//!     let client = NetctlClient::connect().await?;
//!
//!     // Get network state
//!     let state = client.get_network_state().await?;
//!     println!("Network state: {}", state);
//!
//!     // List WiFi access points
//!     let aps = client.wifi_get_access_points().await?;
//!     for ap in aps {
//!         println!("SSID: {:?}", ap.get("SSID"));
//!     }
//!
//!     Ok(())
//! }
//! ```

use crate::cr_dbus::types::*;
use crate::error::{NetctlError, NetctlResult};
use std::collections::HashMap;
use zbus::{Connection, zvariant::OwnedValue};

/// Network Control D-Bus Client
///
/// Main client for communicating with the netctld daemon via D-Bus.
/// Provides access to all network control operations without requiring root privileges.
pub struct NetctlClient {
    connection: Connection,
}

impl NetctlClient {
    /// Connect to the netctld daemon via D-Bus
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Cannot connect to D-Bus system bus
    /// - netctld daemon is not running
    pub async fn connect() -> NetctlResult<Self> {
        let connection = Connection::system().await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to connect to D-Bus: {}", e)))?;

        // Verify the service is available
        let proxy = zbus::fdo::DBusProxy::new(&connection).await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to create D-Bus proxy: {}", e)))?;

        let service_name = CR_DBUS_SERVICE.try_into()
            .map_err(|_| NetctlError::ServiceError(
                format!("Invalid D-Bus service name: {}", CR_DBUS_SERVICE)
            ))?;
        match proxy.name_has_owner(service_name).await {
            Ok(has_owner) => {
                if !has_owner {
                    return Err(NetctlError::ServiceError(
                        format!("Service {} is not available. Is netctld running?", CR_DBUS_SERVICE)
                    ));
                }
            }
            Err(e) => {
                return Err(NetctlError::ServiceError(
                    format!("Failed to check service availability: {}", e)
                ));
            }
        }

        Ok(Self { connection })
    }

    /// Get a reference to the D-Bus connection
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    // ==================== Network Control Methods ====================

    /// Get global network state
    pub async fn get_network_state(&self) -> NetctlResult<u32> {
        self.call_method(CR_DBUS_PATH, "org.crrouter.NetworkControl", "GetState", &()).await
    }

    /// Get connectivity state
    pub async fn get_connectivity(&self) -> NetctlResult<u32> {
        self.call_method(CR_DBUS_PATH, "org.crrouter.NetworkControl", "GetConnectivity", &()).await
    }

    /// Get list of all network devices (returns device paths)
    pub async fn get_devices(&self) -> NetctlResult<Vec<String>> {
        self.call_method(CR_DBUS_PATH, "org.crrouter.NetworkControl", "GetDevices", &()).await
    }

    /// Enable or disable networking globally
    pub async fn set_networking_enabled(&self, enabled: bool) -> NetctlResult<()> {
        self.call_method(CR_DBUS_PATH, "org.crrouter.NetworkControl", "SetNetworkingEnabled", &(enabled,)).await
    }

    /// Check connectivity to the internet
    pub async fn check_connectivity(&self) -> NetctlResult<u32> {
        self.call_method(CR_DBUS_PATH, "org.crrouter.NetworkControl", "CheckConnectivity", &()).await
    }

    // ==================== WiFi Methods ====================

    /// Scan for WiFi networks
    pub async fn wifi_scan(&self) -> NetctlResult<()> {
        self.call_method(CR_WIFI_PATH, "org.crrouter.NetworkControl.WiFi", "Scan", &()).await
    }

    /// Get list of WiFi access points
    pub async fn wifi_get_access_points(&self) -> NetctlResult<Vec<HashMap<String, OwnedValue>>> {
        self.call_method(CR_WIFI_PATH, "org.crrouter.NetworkControl.WiFi", "GetAccessPoints", &()).await
    }

    /// Connect to a WiFi network
    pub async fn wifi_connect(&self, ssid: &str, password: &str) -> NetctlResult<()> {
        self.call_method(CR_WIFI_PATH, "org.crrouter.NetworkControl.WiFi", "Connect", &(ssid, password)).await
    }

    /// Disconnect from current WiFi network
    pub async fn wifi_disconnect(&self) -> NetctlResult<()> {
        self.call_method(CR_WIFI_PATH, "org.crrouter.NetworkControl.WiFi", "Disconnect", &()).await
    }

    /// Start WiFi access point (hotspot)
    pub async fn wifi_start_ap(&self, ssid: &str, password: &str, interface: &str) -> NetctlResult<()> {
        self.call_method(CR_WIFI_PATH, "org.crrouter.NetworkControl.WiFi", "StartAccessPoint", &(ssid, password, interface)).await
    }

    /// Stop WiFi access point
    pub async fn wifi_stop_ap(&self) -> NetctlResult<()> {
        self.call_method(CR_WIFI_PATH, "org.crrouter.NetworkControl.WiFi", "StopAccessPoint", &()).await
    }

    /// Enable/disable WiFi radio
    pub async fn wifi_set_enabled(&self, enabled: bool) -> NetctlResult<()> {
        self.call_method(CR_WIFI_PATH, "org.crrouter.NetworkControl.WiFi", "SetEnabled", &(enabled,)).await
    }

    // ==================== VPN Methods ====================

    /// Get list of VPN connections
    pub async fn vpn_get_connections(&self) -> NetctlResult<Vec<HashMap<String, OwnedValue>>> {
        self.call_method("/org/crrouter/NetworkControl/VPN", "org.crrouter.NetworkControl.VPN", "GetConnections", &()).await
    }

    /// Get VPN connection info
    pub async fn vpn_get_connection_info(&self, name: &str) -> NetctlResult<HashMap<String, OwnedValue>> {
        self.call_method("/org/crrouter/NetworkControl/VPN", "org.crrouter.NetworkControl.VPN", "GetConnectionInfo", &(name,)).await
    }

    /// Connect to a VPN (OpenVPN)
    pub async fn vpn_connect_openvpn(&self, name: &str, config_file: &str) -> NetctlResult<()> {
        self.call_method("/org/crrouter/NetworkControl/VPN", "org.crrouter.NetworkControl.VPN", "ConnectOpenVPN", &(name, config_file)).await
    }

    /// Connect to a VPN (WireGuard)
    pub async fn vpn_connect_wireguard(&self, name: &str, config_file: &str) -> NetctlResult<()> {
        self.call_method("/org/crrouter/NetworkControl/VPN", "org.crrouter.NetworkControl.VPN", "ConnectWireGuard", &(name, config_file)).await
    }

    /// Disconnect from VPN
    pub async fn vpn_disconnect(&self, name: &str) -> NetctlResult<()> {
        self.call_method("/org/crrouter/NetworkControl/VPN", "org.crrouter.NetworkControl.VPN", "Disconnect", &(name,)).await
    }

    /// Get VPN state
    pub async fn vpn_get_state(&self, name: &str) -> NetctlResult<u32> {
        self.call_method("/org/crrouter/NetworkControl/VPN", "org.crrouter.NetworkControl.VPN", "GetState", &(name,)).await
    }

    /// Delete VPN connection
    pub async fn vpn_delete_connection(&self, name: &str) -> NetctlResult<()> {
        self.call_method("/org/crrouter/NetworkControl/VPN", "org.crrouter.NetworkControl.VPN", "DeleteConnection", &(name,)).await
    }

    /// Import VPN configuration from file
    pub async fn vpn_import_config(&self, vpn_type: &str, config_file: &str, name: &str) -> NetctlResult<String> {
        self.call_method("/org/crrouter/NetworkControl/VPN", "org.crrouter.NetworkControl.VPN", "ImportConfig", &(vpn_type, config_file, name)).await
    }

    /// Export VPN configuration
    pub async fn vpn_export_config(&self, name: &str) -> NetctlResult<String> {
        self.call_method("/org/crrouter/NetworkControl/VPN", "org.crrouter.NetworkControl.VPN", "ExportConfig", &(name,)).await
    }

    // ==================== Connection Management Methods ====================

    /// List all connections
    pub async fn connection_list(&self) -> NetctlResult<Vec<HashMap<String, OwnedValue>>> {
        self.call_method(CR_CONNECTION_PATH, "org.crrouter.NetworkControl.Connection", "ListConnections", &()).await
    }

    /// Add a new connection
    pub async fn connection_add(&self, settings: HashMap<String, OwnedValue>) -> NetctlResult<String> {
        self.call_method(CR_CONNECTION_PATH, "org.crrouter.NetworkControl.Connection", "AddConnection", &(settings,)).await
    }

    /// Delete a connection
    pub async fn connection_delete(&self, uuid: &str) -> NetctlResult<()> {
        self.call_method(CR_CONNECTION_PATH, "org.crrouter.NetworkControl.Connection", "DeleteConnection", &(uuid,)).await
    }

    /// Activate a connection
    pub async fn connection_activate(&self, uuid: &str, device_path: &str) -> NetctlResult<()> {
        self.call_method(CR_CONNECTION_PATH, "org.crrouter.NetworkControl.Connection", "ActivateConnection", &(uuid, device_path)).await
    }

    /// Deactivate a connection
    pub async fn connection_deactivate(&self, uuid: &str) -> NetctlResult<()> {
        self.call_method(CR_CONNECTION_PATH, "org.crrouter.NetworkControl.Connection", "DeactivateConnection", &(uuid,)).await
    }

    // ==================== DHCP Methods ====================

    /// Start DHCP server
    pub async fn dhcp_start_server(
        &self,
        interface: &str,
        range_start: &str,
        range_end: &str,
        gateway: &str,
        dns_servers: Vec<String>,
    ) -> NetctlResult<()> {
        self.call_method(CR_DHCP_PATH, "org.crrouter.NetworkControl.DHCP", "StartServer", &(interface, range_start, range_end, gateway, dns_servers)).await
    }

    /// Stop DHCP server
    pub async fn dhcp_stop_server(&self) -> NetctlResult<()> {
        self.call_method(CR_DHCP_PATH, "org.crrouter.NetworkControl.DHCP", "StopServer", &()).await
    }

    /// Get DHCP server status
    pub async fn dhcp_get_status(&self) -> NetctlResult<HashMap<String, OwnedValue>> {
        self.call_method(CR_DHCP_PATH, "org.crrouter.NetworkControl.DHCP", "GetStatus", &()).await
    }

    /// Get DHCP leases
    pub async fn dhcp_get_leases(&self) -> NetctlResult<Vec<HashMap<String, OwnedValue>>> {
        self.call_method(CR_DHCP_PATH, "org.crrouter.NetworkControl.DHCP", "GetLeases", &()).await
    }

    /// Check if DHCP server is running
    pub async fn dhcp_is_running(&self) -> NetctlResult<bool> {
        self.call_method(CR_DHCP_PATH, "org.crrouter.NetworkControl.DHCP", "IsRunning", &()).await
    }

    // ==================== DNS Methods ====================

    /// Start DNS server
    pub async fn dns_start_server(
        &self,
        listen_address: &str,
        listen_port: u16,
        forwarders: Vec<String>,
    ) -> NetctlResult<()> {
        self.call_method(CR_DNS_PATH, "org.crrouter.NetworkControl.DNS", "StartServer", &(listen_address, listen_port, forwarders)).await
    }

    /// Stop DNS server
    pub async fn dns_stop_server(&self) -> NetctlResult<()> {
        self.call_method(CR_DNS_PATH, "org.crrouter.NetworkControl.DNS", "StopServer", &()).await
    }

    /// Add DNS forwarder
    pub async fn dns_add_forwarder(&self, forwarder: &str) -> NetctlResult<()> {
        self.call_method(CR_DNS_PATH, "org.crrouter.NetworkControl.DNS", "AddForwarder", &(forwarder,)).await
    }

    /// Remove DNS forwarder
    pub async fn dns_remove_forwarder(&self, forwarder: &str) -> NetctlResult<()> {
        self.call_method(CR_DNS_PATH, "org.crrouter.NetworkControl.DNS", "RemoveForwarder", &(forwarder,)).await
    }

    /// Get DNS forwarders
    pub async fn dns_get_forwarders(&self) -> NetctlResult<Vec<String>> {
        self.call_method(CR_DNS_PATH, "org.crrouter.NetworkControl.DNS", "GetForwarders", &()).await
    }

    /// Get DNS server status
    pub async fn dns_get_status(&self) -> NetctlResult<HashMap<String, OwnedValue>> {
        self.call_method(CR_DNS_PATH, "org.crrouter.NetworkControl.DNS", "GetStatus", &()).await
    }

    /// Check if DNS server is running
    pub async fn dns_is_running(&self) -> NetctlResult<bool> {
        self.call_method(CR_DNS_PATH, "org.crrouter.NetworkControl.DNS", "IsRunning", &()).await
    }

    // ==================== Routing Methods ====================

    /// Add a route
    pub async fn route_add(
        &self,
        destination: &str,
        gateway: &str,
        interface: &str,
        metric: u32,
    ) -> NetctlResult<()> {
        self.call_method(CR_ROUTING_PATH, "org.crrouter.NetworkControl.Routing", "AddRoute", &(destination, gateway, interface, metric)).await
    }

    /// Remove a route
    pub async fn route_remove(&self, destination: &str) -> NetctlResult<()> {
        self.call_method(CR_ROUTING_PATH, "org.crrouter.NetworkControl.Routing", "RemoveRoute", &(destination,)).await
    }

    /// Get all routes
    pub async fn route_get_routes(&self) -> NetctlResult<Vec<HashMap<String, OwnedValue>>> {
        self.call_method(CR_ROUTING_PATH, "org.crrouter.NetworkControl.Routing", "GetRoutes", &()).await
    }

    /// Set default gateway
    pub async fn route_set_default_gateway(&self, gateway: &str, interface: &str) -> NetctlResult<()> {
        self.call_method(CR_ROUTING_PATH, "org.crrouter.NetworkControl.Routing", "SetDefaultGateway", &(gateway, interface)).await
    }

    /// Get default gateway
    pub async fn route_get_default_gateway(&self) -> NetctlResult<HashMap<String, OwnedValue>> {
        self.call_method(CR_ROUTING_PATH, "org.crrouter.NetworkControl.Routing", "GetDefaultGateway", &()).await
    }

    /// Clear default gateway
    pub async fn route_clear_default_gateway(&self, ipv6: bool) -> NetctlResult<()> {
        self.call_method(CR_ROUTING_PATH, "org.crrouter.NetworkControl.Routing", "ClearDefaultGateway", &(ipv6,)).await
    }

    // ==================== Helper Methods ====================

    /// Call a D-Bus method
    async fn call_method<B, R>(&self, path: &str, interface: &str, method: &str, body: &B) -> NetctlResult<R>
    where
        B: serde::ser::Serialize + zbus::zvariant::DynamicType,
        R: serde::de::DeserializeOwned + zbus::zvariant::Type,
    {
        self.connection
            .call_method(
                Some(CR_DBUS_SERVICE),
                path,
                Some(interface),
                method,
                body,
            )
            .await
            .map_err(|e| NetctlError::ServiceError(format!("D-Bus method call failed: {}", e)))?
            .body()
            .deserialize()
            .map_err(|e| NetctlError::ServiceError(format!("Failed to deserialize response: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires running daemon
    async fn test_connect() {
        let result = NetctlClient::connect().await;
        assert!(result.is_ok(), "Failed to connect to daemon");
    }

    #[tokio::test]
    #[ignore] // Requires running daemon
    async fn test_get_network_state() {
        let client = NetctlClient::connect().await.unwrap();
        let state = client.get_network_state().await;
        assert!(state.is_ok(), "Failed to get network state");
    }
}
