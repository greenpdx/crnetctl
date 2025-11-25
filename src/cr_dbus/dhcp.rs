//! CR DHCP Server D-Bus interface
//!
//! D-Bus interface for DHCP server management

use super::types::*;
use crate::error::{NetctlError, NetctlResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug};
use zbus::{Connection, fdo, interface};
use zbus::object_server::SignalEmitter;
use zbus::zvariant::Value;

/// CR DHCP Server D-Bus interface
#[derive(Clone)]
pub struct CRDhcp {
    /// Whether DHCP server is running
    running: Arc<RwLock<bool>>,
    /// Current configuration
    config: Arc<RwLock<Option<DhcpConfig>>>,
    /// Active DHCP leases
    leases: Arc<RwLock<Vec<CRDhcpLease>>>,
}

#[derive(Debug, Clone)]
struct DhcpConfig {
    interface: String,
    range_start: String,
    range_end: String,
    gateway: String,
    dns_servers: Vec<String>,
}

impl CRDhcp {
    /// Create a new CR DHCP interface
    pub fn new() -> Self {
        Self {
            running: Arc::new(RwLock::new(false)),
            config: Arc::new(RwLock::new(None)),
            leases: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Set running state
    pub async fn set_running(&self, running: bool) {
        let mut r = self.running.write().await;
        *r = running;
        info!("CR DHCP: Server running state set to {}", running);
    }

    /// Add a lease
    pub async fn add_lease(&self, lease: CRDhcpLease) {
        let mut leases = self.leases.write().await;
        leases.push(lease);
    }

    /// Remove expired leases
    pub async fn remove_expired_leases(&self) {
        let mut leases = self.leases.write().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("System time is before UNIX epoch")
            .as_secs();
        leases.retain(|lease| lease.expiry > now);
    }
}

#[interface(name = "org.crrouter.NetworkControl.DHCP")]
impl CRDhcp {
    /// Start DHCP server
    async fn start_server(
        &self,
        interface: &str,
        range_start: &str,
        range_end: &str,
        gateway: &str,
        dns_servers: Vec<String>,
    ) -> fdo::Result<()> {
        info!(
            "CR DHCP: Starting DHCP server on {} (range: {} - {})",
            interface, range_start, range_end
        );

        // Validate parameters
        if interface.is_empty() {
            return Err(fdo::Error::InvalidArgs("Interface cannot be empty".to_string()));
        }

        if range_start.is_empty() || range_end.is_empty() {
            return Err(fdo::Error::InvalidArgs("IP range cannot be empty".to_string()));
        }

        if gateway.is_empty() {
            return Err(fdo::Error::InvalidArgs("Gateway cannot be empty".to_string()));
        }

        // Check if already running
        let running = self.running.read().await;
        if *running {
            return Err(fdo::Error::Failed("DHCP server already running".to_string()));
        }
        drop(running);

        // Store configuration
        let config = DhcpConfig {
            interface: interface.to_string(),
            range_start: range_start.to_string(),
            range_end: range_end.to_string(),
            gateway: gateway.to_string(),
            dns_servers,
        };

        let mut cfg = self.config.write().await;
        *cfg = Some(config);
        drop(cfg);

        // Set running state
        self.set_running(true).await;

        // Actual DHCP server start will be handled by integration layer

        Ok(())
    }

    /// Stop DHCP server
    async fn stop_server(&self) -> fdo::Result<()> {
        info!("CR DHCP: Stopping DHCP server");

        let running = self.running.read().await;
        if !*running {
            return Err(fdo::Error::Failed("DHCP server not running".to_string()));
        }
        drop(running);

        // Clear configuration
        let mut cfg = self.config.write().await;
        *cfg = None;
        drop(cfg);

        // Set running state
        self.set_running(false).await;

        // Actual DHCP server stop will be handled by integration layer

        Ok(())
    }

    /// Get DHCP server status
    async fn get_status(&self) -> HashMap<String, Value<'static>> {
        let mut status = HashMap::new();

        let running = self.running.read().await;
        status.insert("Running".to_string(), Value::new(*running));

        if let Some(ref config) = *self.config.read().await {
            status.insert("Interface".to_string(), Value::new(config.interface.clone()));
            status.insert("RangeStart".to_string(), Value::new(config.range_start.clone()));
            status.insert("RangeEnd".to_string(), Value::new(config.range_end.clone()));
            status.insert("Gateway".to_string(), Value::new(config.gateway.clone()));
            status.insert("DNSServers".to_string(), Value::new(config.dns_servers.clone()));
        }

        let leases = self.leases.read().await;
        status.insert("LeaseCount".to_string(), Value::new(leases.len() as u32));

        debug!("CR DHCP: Returning status");
        status
    }

    /// Get all DHCP leases
    async fn get_leases(&self) -> Vec<HashMap<String, Value<'static>>> {
        let leases = self.leases.read().await;
        let mut result = Vec::new();

        for lease in leases.iter() {
            let mut lease_info = HashMap::new();
            lease_info.insert("MACAddress".to_string(), Value::new(lease.mac_address.clone()));
            lease_info.insert("IPAddress".to_string(), Value::new(lease.ip_address.clone()));

            if let Some(ref hostname) = lease.hostname {
                lease_info.insert("Hostname".to_string(), Value::new(hostname.clone()));
            }

            lease_info.insert("Expiry".to_string(), Value::new(lease.expiry));
            lease_info.insert("StartTime".to_string(), Value::new(lease.start_time));

            result.push(lease_info);
        }

        debug!("CR DHCP: Returning {} leases", result.len());
        result
    }

    /// Check if DHCP server is running
    async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    // ============ D-Bus Signals ============

    /// ServerStarted signal - emitted when DHCP server starts
    #[zbus(signal)]
    async fn server_started(signal_emitter: &SignalEmitter<'_>, interface: &str) -> zbus::Result<()>;

    /// ServerStopped signal - emitted when DHCP server stops
    #[zbus(signal)]
    async fn server_stopped(signal_emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    /// LeaseAssigned signal - emitted when a lease is assigned
    #[zbus(signal)]
    async fn lease_assigned(
        signal_emitter: &SignalEmitter<'_>,
        mac_address: &str,
        ip_address: &str,
        hostname: &str,
    ) -> zbus::Result<()>;

    /// LeaseExpired signal - emitted when a lease expires
    #[zbus(signal)]
    async fn lease_expired(
        signal_emitter: &SignalEmitter<'_>,
        mac_address: &str,
        ip_address: &str,
    ) -> zbus::Result<()>;
}

impl Default for CRDhcp {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper module for emitting DHCP signals
pub mod signals {
    use super::*;

    /// Emit ServerStarted signal
    pub async fn emit_server_started(
        conn: &Connection,
        interface: &str,
    ) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRDhcp>(CR_DHCP_PATH)
            .await
        {
            CRDhcp::server_started(iface_ref.signal_emitter(), interface)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit ServerStarted: {}", e)))?;
        }
        Ok(())
    }

    /// Emit ServerStopped signal
    pub async fn emit_server_stopped(conn: &Connection) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRDhcp>(CR_DHCP_PATH)
            .await
        {
            CRDhcp::server_stopped(iface_ref.signal_emitter())
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit ServerStopped: {}", e)))?;
        }
        Ok(())
    }

    /// Emit LeaseAssigned signal
    pub async fn emit_lease_assigned(
        conn: &Connection,
        mac_address: &str,
        ip_address: &str,
        hostname: &str,
    ) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRDhcp>(CR_DHCP_PATH)
            .await
        {
            CRDhcp::lease_assigned(iface_ref.signal_emitter(), mac_address, ip_address, hostname)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit LeaseAssigned: {}", e)))?;
        }
        Ok(())
    }

    /// Emit LeaseExpired signal
    pub async fn emit_lease_expired(
        conn: &Connection,
        mac_address: &str,
        ip_address: &str,
    ) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRDhcp>(CR_DHCP_PATH)
            .await
        {
            CRDhcp::lease_expired(iface_ref.signal_emitter(), mac_address, ip_address)
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit LeaseExpired: {}", e)))?;
        }
        Ok(())
    }
}
