//! Network link state monitoring
//!
//! This module monitors network interface link state changes (up/down)
//! and triggers actions like starting DHCP when links come up.
//!
//! It subscribes to NetworkMonitor events for real-time notifications
//! instead of polling.

use crate::error::NetctlResult;
use crate::dhcp_client::DhcpClientController;
use crate::interface::InterfaceController;
use crate::network_monitor::{NetworkMonitor, NetworkEvent};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

/// Link state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkState {
    /// Link is down
    Down,
    /// Link is up (carrier detected)
    Up,
    /// Link state unknown
    Unknown,
}

/// Link state change event
#[derive(Debug, Clone)]
pub struct LinkStateEvent {
    /// Interface name
    pub interface: String,
    /// New link state
    pub state: LinkState,
    /// Previous link state
    pub previous_state: LinkState,
}

/// Configuration for an interface
#[derive(Debug, Clone)]
pub struct InterfaceConfig {
    /// Interface name
    pub interface: String,
    /// Whether to auto-start DHCP when link comes up
    pub auto_dhcp: bool,
}

/// Link state monitor that subscribes to NetworkMonitor events
pub struct LinkMonitor {
    /// Interface controller
    interface_controller: Arc<InterfaceController>,
    /// DHCP client controller
    dhcp_client: Arc<DhcpClientController>,
    /// Current link states
    link_states: Arc<RwLock<HashMap<String, LinkState>>>,
    /// Interface configurations
    interface_configs: Arc<RwLock<HashMap<String, InterfaceConfig>>>,
    /// Event sender
    event_tx: mpsc::UnboundedSender<LinkStateEvent>,
    /// Network monitor to subscribe to
    network_monitor: Option<Arc<NetworkMonitor>>,
}

impl LinkMonitor {
    /// Create a new link monitor
    pub fn new(
        interface_controller: Arc<InterfaceController>,
        dhcp_client: Arc<DhcpClientController>,
    ) -> (Self, mpsc::UnboundedReceiver<LinkStateEvent>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        (
            Self {
                interface_controller,
                dhcp_client,
                link_states: Arc::new(RwLock::new(HashMap::new())),
                interface_configs: Arc::new(RwLock::new(HashMap::new())),
                event_tx,
                network_monitor: None,
            },
            event_rx,
        )
    }

    /// Set the network monitor to subscribe to for events
    pub fn set_network_monitor(&mut self, monitor: Arc<NetworkMonitor>) {
        self.network_monitor = Some(monitor);
    }

    /// Add an interface to monitor
    pub async fn add_interface(&self, config: InterfaceConfig) {
        let interface = config.interface.clone();
        info!("Adding interface {} to link monitor (auto_dhcp: {})",
              interface, config.auto_dhcp);

        // Initialize link state
        let state = self.get_link_state(&interface).await;
        self.link_states.write().await.insert(interface.clone(), state);
        self.interface_configs.write().await.insert(interface, config);
    }

    /// Remove an interface from monitoring
    pub async fn remove_interface(&self, interface: &str) {
        info!("Removing interface {} from link monitor", interface);
        self.link_states.write().await.remove(interface);
        self.interface_configs.write().await.remove(interface);
    }

    /// Get current link state for an interface
    async fn get_link_state(&self, interface: &str) -> LinkState {
        match self.interface_controller.get_link_state(interface).await {
            Ok(is_up) => {
                if is_up {
                    LinkState::Up
                } else {
                    LinkState::Down
                }
            }
            Err(e) => {
                debug!("Failed to get link state for {}: {}", interface, e);
                LinkState::Unknown
            }
        }
    }

    /// Start monitoring using NetworkMonitor events (preferred method)
    pub async fn start_with_events(self: Arc<Self>) -> NetctlResult<()> {
        let monitor = match &self.network_monitor {
            Some(m) => m.clone(),
            None => {
                warn!("No NetworkMonitor configured, cannot start event-based monitoring");
                return Ok(());
            }
        };

        info!("Starting link state monitor (event-driven via NetworkMonitor)");

        let mut event_rx = monitor.subscribe();

        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    if let Err(e) = self.handle_network_event(event).await {
                        warn!("Error handling network event: {}", e);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!("LinkMonitor lagged behind by {} events", n);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    info!("NetworkMonitor channel closed, stopping LinkMonitor");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle a network event from NetworkMonitor
    async fn handle_network_event(&self, event: NetworkEvent) -> NetctlResult<()> {
        match event {
            NetworkEvent::InterfaceStateChanged { name, is_up, .. } => {
                // Check if we're monitoring this interface
                let configs = self.interface_configs.read().await;
                if !configs.contains_key(&name) {
                    return Ok(());
                }
                drop(configs);

                let new_state = if is_up { LinkState::Up } else { LinkState::Down };
                let previous_state = {
                    let states = self.link_states.read().await;
                    states.get(&name).copied().unwrap_or(LinkState::Unknown)
                };

                if new_state != previous_state {
                    // Update stored state
                    self.link_states.write().await.insert(name.clone(), new_state);

                    // Send event
                    let link_event = LinkStateEvent {
                        interface: name.clone(),
                        state: new_state,
                        previous_state,
                    };
                    let _ = self.event_tx.send(link_event.clone());

                    // Handle state change (DHCP actions)
                    self.handle_state_change(&link_event).await?;
                }
            }
            NetworkEvent::InterfaceAdded { name, .. } => {
                // Check if this interface should be auto-monitored
                debug!("Interface added: {}", name);
            }
            NetworkEvent::InterfaceRemoved { name, .. } => {
                // Remove from our tracking if present
                self.link_states.write().await.remove(&name);
                debug!("Interface removed: {}", name);
            }
            _ => {
                // Ignore other events
            }
        }

        Ok(())
    }

    /// Start monitoring (runs in background) - legacy polling mode
    /// Prefer start_with_events() when a NetworkMonitor is available
    pub async fn start(self: Arc<Self>) -> NetctlResult<()> {
        // If we have a network monitor, use event-driven mode
        if self.network_monitor.is_some() {
            return self.start_with_events().await;
        }

        // Otherwise fall back to polling (legacy behavior)
        info!("Starting link state monitor (polling mode - no NetworkMonitor configured)");

        loop {
            // Get list of interfaces to monitor
            let interfaces: Vec<String> = self.interface_configs.read().await
                .keys()
                .cloned()
                .collect();

            // Check each interface
            for interface in interfaces {
                if let Err(e) = self.check_interface(&interface).await {
                    warn!("Error checking interface {}: {}", interface, e);
                }
            }

            sleep(Duration::from_secs(2)).await;
        }
    }

    /// Check an interface for state changes (polling mode only)
    async fn check_interface(&self, interface: &str) -> NetctlResult<()> {
        let current_state = self.get_link_state(interface).await;
        let previous_state = {
            let states = self.link_states.read().await;
            states.get(interface).copied().unwrap_or(LinkState::Unknown)
        };

        // Check if state changed
        if current_state != previous_state {
            info!("Link state changed on {}: {:?} -> {:?}",
                  interface, previous_state, current_state);

            // Update stored state
            self.link_states.write().await.insert(interface.to_string(), current_state);

            // Send event
            let event = LinkStateEvent {
                interface: interface.to_string(),
                state: current_state,
                previous_state,
            };
            let _ = self.event_tx.send(event.clone());

            // Handle state change
            self.handle_state_change(&event).await?;
        }

        Ok(())
    }

    /// Handle a link state change
    async fn handle_state_change(&self, event: &LinkStateEvent) -> NetctlResult<()> {
        let configs = self.interface_configs.read().await;
        let config = match configs.get(&event.interface) {
            Some(c) => c,
            None => return Ok(()),
        };

        match event.state {
            LinkState::Up => {
                if event.previous_state == LinkState::Down {
                    info!("Link up on {}", event.interface);

                    // Start DHCP if configured
                    if config.auto_dhcp {
                        self.start_dhcp(&event.interface).await?;
                    }
                }
            }
            LinkState::Down => {
                if event.previous_state == LinkState::Up {
                    info!("Link down on {}", event.interface);

                    // Stop DHCP if configured
                    if config.auto_dhcp {
                        self.stop_dhcp(&event.interface).await?;
                    }
                }
            }
            LinkState::Unknown => {
                // Do nothing
            }
        }

        Ok(())
    }

    /// Start DHCP on an interface
    async fn start_dhcp(&self, interface: &str) -> NetctlResult<()> {
        info!("Starting DHCP client on {} (link up)", interface);

        // Small delay to let the interface stabilize
        sleep(Duration::from_millis(500)).await;

        match self.dhcp_client.start(interface).await {
            Ok(()) => {
                info!("DHCP client started successfully on {}", interface);
                Ok(())
            }
            Err(e) => {
                error!("Failed to start DHCP client on {}: {}", interface, e);
                // Don't propagate error - this is best-effort
                Ok(())
            }
        }
    }

    /// Stop DHCP on an interface
    async fn stop_dhcp(&self, interface: &str) -> NetctlResult<()> {
        info!("Stopping DHCP client on {} (link down)", interface);

        match self.dhcp_client.release(interface).await {
            Ok(()) => {
                debug!("DHCP lease released on {}", interface);
            }
            Err(e) => {
                warn!("Failed to release DHCP lease on {}: {}", interface, e);
            }
        }

        match self.dhcp_client.stop(interface).await {
            Ok(()) => {
                info!("DHCP client stopped on {}", interface);
                Ok(())
            }
            Err(e) => {
                warn!("Failed to stop DHCP client on {}: {}", interface, e);
                Ok(())
            }
        }
    }

    /// Get current link states
    pub async fn get_link_states(&self) -> HashMap<String, LinkState> {
        self.link_states.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_state_equality() {
        assert_eq!(LinkState::Up, LinkState::Up);
        assert_eq!(LinkState::Down, LinkState::Down);
        assert_ne!(LinkState::Up, LinkState::Down);
    }

    #[test]
    fn test_interface_config() {
        let config = InterfaceConfig {
            interface: "eth0".to_string(),
            auto_dhcp: true,
        };
        assert_eq!(config.interface, "eth0");
        assert!(config.auto_dhcp);
    }
}
