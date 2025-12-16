//! Network event monitoring using netlink
//!
//! This module monitors network interface events using rtnetlink event streams
//! and propagates them to D-Bus and other subscribers.

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
            if let Err(e) = Self::monitor_with_rtnetlink_events(event_tx.clone(), running.clone()).await {
                warn!("rtnetlink event monitoring failed: {}, falling back to polling", e);
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

    /// Monitor using rtnetlink events (Linux-specific)
    #[cfg(target_os = "linux")]
    async fn monitor_with_rtnetlink_events(
        event_tx: broadcast::Sender<NetworkEvent>,
        running: Arc<tokio::sync::RwLock<bool>>,
    ) -> NetctlResult<()> {
        use futures::stream::TryStreamExt;
        use netlink_sys::{protocols::NETLINK_ROUTE, Socket, SocketAddr};

        // Create a netlink socket and join multicast groups for link events
        let mut socket = Socket::new(NETLINK_ROUTE)
            .map_err(|e| NetctlError::ServiceError(format!("Failed to create netlink socket: {}", e)))?;

        // Bind to the socket
        let kernel_addr = SocketAddr::new(0, 0);
        socket.bind(&kernel_addr)
            .map_err(|e| NetctlError::ServiceError(format!("Failed to bind netlink socket: {}", e)))?;

        // Join RTNLGRP_LINK multicast group (group 1)
        // RTNLGRP_LINK = 1, so we use 1 << (1-1) = 1
        const RTNLGRP_LINK: u32 = 1;
        socket.add_membership(RTNLGRP_LINK)
            .map_err(|e| NetctlError::ServiceError(format!("Failed to join RTNLGRP_LINK: {}", e)))?;

        // Also join RTNLGRP_IPV4_IFADDR for address changes (group 5)
        const RTNLGRP_IPV4_IFADDR: u32 = 5;
        socket.add_membership(RTNLGRP_IPV4_IFADDR)
            .map_err(|e| NetctlError::ServiceError(format!("Failed to join RTNLGRP_IPV4_IFADDR: {}", e)))?;

        // Set socket to non-blocking for async operation
        socket.set_non_blocking(true)
            .map_err(|e| NetctlError::ServiceError(format!("Failed to set non-blocking: {}", e)))?;

        info!("Using rtnetlink events for network monitoring (joined RTNLGRP_LINK, RTNLGRP_IPV4_IFADDR)");

        // Also create an rtnetlink handle for querying initial state
        let (connection, handle, _) = rtnetlink::new_connection()
            .map_err(|e| NetctlError::ServiceError(format!("Failed to create rtnetlink connection: {}", e)))?;

        tokio::spawn(connection);

        // Track interfaces: index -> (name, is_up)
        let mut known_interfaces: std::collections::HashMap<u32, (String, bool)> = std::collections::HashMap::new();

        // Get initial interface list and their states
        let mut links = handle.link().get().execute();
        while let Some(link) = links.try_next().await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to get links: {}", e)))? {
            if let Some(name) = extract_interface_name(&link) {
                let is_up = extract_operstate(&link);
                known_interfaces.insert(link.header.index, (name.clone(), is_up));
                debug!("Found interface: {} (index {}) state: {}", name, link.header.index, if is_up { "up" } else { "down" });
            }
        }

        // Buffer for receiving netlink messages
        let mut buf = vec![0u8; 16384];

        // Convert socket to async using tokio
        let async_fd = tokio::io::unix::AsyncFd::new(socket)
            .map_err(|e| NetctlError::ServiceError(format!("Failed to create async fd: {}", e)))?;

        info!("Network monitor ready, listening for events...");

        while *running.read().await {
            // Wait for the socket to be readable
            let mut guard = match tokio::time::timeout(
                tokio::time::Duration::from_secs(1),
                async_fd.readable()
            ).await {
                Ok(Ok(guard)) => guard,
                Ok(Err(e)) => {
                    error!("AsyncFd error: {}", e);
                    continue;
                }
                Err(_) => {
                    // Timeout - just continue to check running flag
                    continue;
                }
            };

            // Try to receive data
            match guard.get_inner().recv(&mut buf, 0) {
                Ok(len) if len > 0 => {
                    // Parse the netlink messages
                    if let Err(e) = process_netlink_messages(
                        &buf[..len],
                        &mut known_interfaces,
                        &event_tx,
                    ) {
                        warn!("Error processing netlink message: {}", e);
                    }
                }
                Ok(_) => {
                    // No data, continue
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No data available, this is expected for non-blocking
                }
                Err(e) => {
                    warn!("Error receiving netlink message: {}", e);
                }
            }

            guard.clear_ready();
        }

        Ok(())
    }

    /// Read operstate from sysfs - returns true if interface is "up"
    #[allow(dead_code)]
    async fn read_operstate(interface: &str) -> bool {
        let operstate_path = format!("/sys/class/net/{}/operstate", interface);
        match tokio::fs::read_to_string(&operstate_path).await {
            Ok(state) => state.trim() == "up",
            Err(_) => false,
        }
    }

    /// Monitor using periodic polling (fallback)
    async fn monitor_with_polling(
        event_tx: broadcast::Sender<NetworkEvent>,
        running: Arc<tokio::sync::RwLock<bool>>,
    ) -> NetctlResult<()> {
        info!("Using polling for network event monitoring (fallback)");

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

/// Extract interface name from a LinkMessage
#[cfg(target_os = "linux")]
fn extract_interface_name(link: &netlink_packet_route::link::LinkMessage) -> Option<String> {
    use netlink_packet_route::link::LinkAttribute;
    link.attributes.iter().find_map(|attr| {
        if let LinkAttribute::IfName(name) = attr {
            Some(name.clone())
        } else {
            None
        }
    })
}

/// Extract operstate from a LinkMessage - returns true if up
#[cfg(target_os = "linux")]
fn extract_operstate(link: &netlink_packet_route::link::LinkMessage) -> bool {
    use netlink_packet_route::link::{LinkAttribute, LinkFlags, State};

    // First check operstate attribute
    for attr in &link.attributes {
        if let LinkAttribute::OperState(state) = attr {
            return *state == State::Up;
        }
    }

    // Fallback: check IFF_UP and IFF_RUNNING flags
    let flags = link.header.flags;
    flags.contains(LinkFlags::Up) && flags.contains(LinkFlags::Running)
}

/// Process raw netlink messages from the socket
#[cfg(target_os = "linux")]
fn process_netlink_messages(
    data: &[u8],
    known_interfaces: &mut std::collections::HashMap<u32, (String, bool)>,
    event_tx: &broadcast::Sender<NetworkEvent>,
) -> NetctlResult<()> {
    use netlink_packet_core::{NetlinkMessage, NetlinkPayload};
    use netlink_packet_route::RouteNetlinkMessage;

    let mut offset = 0;
    while offset < data.len() {
        // Parse the netlink message header
        let msg: NetlinkMessage<RouteNetlinkMessage> = match NetlinkMessage::deserialize(&data[offset..]) {
            Ok(msg) => msg,
            Err(e) => {
                debug!("Failed to parse netlink message: {}", e);
                break;
            }
        };

        let msg_len = msg.header.length as usize;
        if msg_len == 0 {
            break;
        }

        match msg.payload {
            NetlinkPayload::InnerMessage(RouteNetlinkMessage::NewLink(link)) => {
                if let Some(name) = extract_interface_name(&link) {
                    let index = link.header.index;
                    let is_up = extract_operstate(&link);

                    if let Some((_, old_is_up)) = known_interfaces.get(&index) {
                        // Existing interface - check for state change
                        if *old_is_up != is_up {
                            info!("Interface {} state changed: {} -> {}",
                                  name,
                                  if *old_is_up { "up" } else { "down" },
                                  if is_up { "up" } else { "down" });
                            let _ = event_tx.send(NetworkEvent::InterfaceStateChanged {
                                index,
                                name: name.clone(),
                                is_up,
                            });
                        }
                    } else {
                        // New interface
                        info!("New interface detected: {} (index {})", name, index);
                        let _ = event_tx.send(NetworkEvent::InterfaceAdded {
                            index,
                            name: name.clone(),
                        });
                    }

                    known_interfaces.insert(index, (name, is_up));
                }
            }
            NetlinkPayload::InnerMessage(RouteNetlinkMessage::DelLink(link)) => {
                let index = link.header.index;
                if let Some((name, _)) = known_interfaces.remove(&index) {
                    info!("Interface removed: {} (index {})", name, index);
                    let _ = event_tx.send(NetworkEvent::InterfaceRemoved {
                        index,
                        name,
                    });
                }
            }
            NetlinkPayload::InnerMessage(RouteNetlinkMessage::NewAddress(addr)) => {
                let index = addr.header.index;
                if let Some((name, _)) = known_interfaces.get(&index) {
                    // Extract address from attributes
                    use netlink_packet_route::address::AddressAttribute;
                    for attr in &addr.attributes {
                        if let AddressAttribute::Address(ip) = attr {
                            let addr_str = format!("{}/{}", ip, addr.header.prefix_len);
                            info!("Address added on {}: {}", name, addr_str);
                            let _ = event_tx.send(NetworkEvent::InterfaceAddressChanged {
                                index,
                                name: name.clone(),
                                address: addr_str,
                            });
                            break;
                        }
                    }
                }
            }
            NetlinkPayload::InnerMessage(RouteNetlinkMessage::DelAddress(addr)) => {
                let index = addr.header.index;
                if let Some((name, _)) = known_interfaces.get(&index) {
                    use netlink_packet_route::address::AddressAttribute;
                    for attr in &addr.attributes {
                        if let AddressAttribute::Address(ip) = attr {
                            let addr_str = format!("{}/{}", ip, addr.header.prefix_len);
                            info!("Address removed from {}: {}", name, addr_str);
                            // We use the same event for add/remove, could add a separate event type
                            let _ = event_tx.send(NetworkEvent::InterfaceAddressChanged {
                                index,
                                name: name.clone(),
                                address: format!("-{}", addr_str), // Prefix with - to indicate removal
                            });
                            break;
                        }
                    }
                }
            }
            _ => {
                // Ignore other message types
            }
        }

        offset += msg_len;
    }

    Ok(())
}
