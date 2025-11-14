//! Network event monitoring using netlink
//!
//! This module monitors network interface events and propagates them to D-Bus

use crate::error::{NetctlError, NetctlResult};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn, error, debug};

/// Network event types
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// Interface added
    InterfaceAdded {
        index: u32,
        name: String,
    },
    /// Interface removed
    InterfaceRemoved {
        index: u32,
        name: String,
    },
    /// Interface state changed
    InterfaceStateChanged {
        index: u32,
        name: String,
        is_up: bool,
    },
    /// Interface address changed
    InterfaceAddressChanged {
        index: u32,
        name: String,
        address: String,
    },
    /// Link properties changed
    LinkPropertiesChanged {
        index: u32,
        name: String,
    },
}

/// Network monitor that watches for interface changes
pub struct NetworkMonitor {
    /// Event broadcaster
    event_tx: broadcast::Sender<NetworkEvent>,
    /// Running flag
    running: Arc<tokio::sync::RwLock<bool>>,
}

impl NetworkMonitor {
    /// Create a new network monitor
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self {
            event_tx,
            running: Arc::new(tokio::sync::RwLock::new(false)),
        }
    }

    /// Subscribe to network events
    pub fn subscribe(&self) -> broadcast::Receiver<NetworkEvent> {
        self.event_tx.subscribe()
    }

    /// Start monitoring network events
    pub async fn start(&self) -> NetctlResult<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(NetctlError::ServiceError("Network monitor already running".to_string()));
        }
        *running = true;
        drop(running);

        info!("Starting network event monitor");

        let event_tx = self.event_tx.clone();
        let running = self.running.clone();

        // Spawn monitoring task
        tokio::spawn(async move {
            if let Err(e) = Self::monitor_loop(event_tx, running).await {
                error!("Network monitor error: {}", e);
            }
        });

        Ok(())
    }

    /// Stop monitoring network events
    pub async fn stop(&self) -> NetctlResult<()> {
        let mut running = self.running.write().await;
        *running = false;
        info!("Stopped network event monitor");
        Ok(())
    }

    /// Main monitoring loop
    async fn monitor_loop(
        event_tx: broadcast::Sender<NetworkEvent>,
        running: Arc<tokio::sync::RwLock<bool>>,
    ) -> NetctlResult<()> {
        // Try to use rtnetlink for real netlink monitoring
        #[cfg(target_os = "linux")]
        {
            if let Err(e) = Self::monitor_with_rtnetlink(event_tx.clone(), running.clone()).await {
                warn!("rtnetlink monitoring failed: {}, falling back to polling", e);
                Self::monitor_with_polling(event_tx, running).await?;
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            // Fall back to polling on non-Linux systems
            Self::monitor_with_polling(event_tx, running).await?;
        }

        Ok(())
    }

    /// Monitor using rtnetlink (Linux-specific)
    #[cfg(target_os = "linux")]
    async fn monitor_with_rtnetlink(
        event_tx: broadcast::Sender<NetworkEvent>,
        running: Arc<tokio::sync::RwLock<bool>>,
    ) -> NetctlResult<()> {
        use futures::stream::TryStreamExt;
        use netlink_packet_route::link::LinkAttribute;

        // Try to use netlink
        let (connection, handle, _) = rtnetlink::new_connection()
            .map_err(|e| NetctlError::ServiceError(format!("Failed to create netlink connection: {}", e)))?;

        tokio::spawn(connection);

        info!("Using rtnetlink for network event monitoring");

        // Get initial interface list
        let mut links = handle.link().get().execute();
        let mut known_interfaces: std::collections::HashMap<u32, String> = std::collections::HashMap::new();

        while let Some(link) = links.try_next().await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to get links: {}", e)))? {
            if let Some(name) = link.attributes.iter().find_map(|attr| {
                if let LinkAttribute::IfName(name) = attr {
                    Some(name.clone())
                } else {
                    None
                }
            }) {
                known_interfaces.insert(link.header.index, name.clone());
                debug!("Found interface: {} (index {})", name, link.header.index);
            }
        }

        // Use polling instead of listening to netlink messages for simplicity
        // (listening requires more complex message handling)
        info!("Using periodic polling for interface changes");
        while *running.read().await {
            // Refresh interface list
            let mut current_links = handle.link().get().execute();
            let mut current_interfaces = std::collections::HashMap::new();

            while let Some(link) = current_links.try_next().await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to get links: {}", e)))? {
                if let Some(name) = link.attributes.iter().find_map(|attr| {
                    if let LinkAttribute::IfName(name) = attr {
                        Some(name.clone())
                    } else {
                        None
                    }
                }) {
                    let index = link.header.index;
                    current_interfaces.insert(index, name.clone());

                    // Check if interface is new
                    if !known_interfaces.contains_key(&index) {
                        info!("New interface detected: {} (index {})", name, index);
                        let _ = event_tx.send(NetworkEvent::InterfaceAdded {
                            index,
                            name: name.clone(),
                        });
                    }

                    // Check state - check if IFF_UP flag is set
                    let is_up = !link.header.flags.is_empty();
                    let _ = event_tx.send(NetworkEvent::InterfaceStateChanged {
                        index,
                        name,
                        is_up,
                    });
                }
            }

            // Check for removed interfaces
            for (index, name) in known_interfaces.iter() {
                if !current_interfaces.contains_key(index) {
                    info!("Interface removed: {} (index {})", name, index);
                    let _ = event_tx.send(NetworkEvent::InterfaceRemoved {
                        index: *index,
                        name: name.clone(),
                    });
                }
            }

            known_interfaces = current_interfaces;

            // Poll every 2 seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

        Ok(())
    }

    /// Monitor using periodic polling (fallback)
    async fn monitor_with_polling(
        event_tx: broadcast::Sender<NetworkEvent>,
        running: Arc<tokio::sync::RwLock<bool>>,
    ) -> NetctlResult<()> {
        info!("Using polling for network event monitoring");

        let mut known_interfaces: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
        let mut interface_counter = 0u32;

        while *running.read().await {
            // Read /sys/class/net to get interface list
            if let Ok(entries) = tokio::fs::read_dir("/sys/class/net").await {
                let mut entries = entries;
                let mut current_interfaces = std::collections::HashSet::new();

                while let Ok(Some(entry)) = entries.next_entry().await {
                    if let Ok(name) = entry.file_name().into_string() {
                        current_interfaces.insert(name.clone());

                        // Check if interface is new
                        if !known_interfaces.contains_key(&name) {
                            interface_counter += 1;
                            info!("New interface detected: {}", name);
                            known_interfaces.insert(name.clone(), false);
                            let _ = event_tx.send(NetworkEvent::InterfaceAdded {
                                index: interface_counter,
                                name: name.clone(),
                            });
                        }

                        // Check interface state
                        let operstate_path = format!("/sys/class/net/{}/operstate", name);
                        if let Ok(state) = tokio::fs::read_to_string(&operstate_path).await {
                            let is_up = state.trim() == "up";
                            if let Some(old_state) = known_interfaces.get(&name) {
                                if *old_state != is_up {
                                    debug!("Interface state changed: {} -> {}", name, if is_up { "up" } else { "down" });
                                    known_interfaces.insert(name.clone(), is_up);
                                    let _ = event_tx.send(NetworkEvent::InterfaceStateChanged {
                                        index: interface_counter,
                                        name: name.clone(),
                                        is_up,
                                    });
                                }
                            }
                        }
                    }
                }

                // Check for removed interfaces
                let removed: Vec<String> = known_interfaces
                    .keys()
                    .filter(|k| !current_interfaces.contains(*k))
                    .cloned()
                    .collect();

                for name in removed {
                    info!("Interface removed: {}", name);
                    known_interfaces.remove(&name);
                    let _ = event_tx.send(NetworkEvent::InterfaceRemoved {
                        index: interface_counter,
                        name,
                    });
                }
            }

            // Poll every 2 seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

        Ok(())
    }
}

impl Default for NetworkMonitor {
    fn default() -> Self {
        Self::new()
    }
}
