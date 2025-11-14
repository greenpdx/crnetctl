//! D-Bus integration for network events
//!
//! This module connects network monitoring events to D-Bus signal emission

#[cfg(feature = "dbus-nm")]
use crate::dbus::{DeviceInfo, DeviceState, NetworkManagerDBus, NM_DBUS_PATH, signals};
#[cfg(feature = "dbus-nm")]
use crate::network_monitor::{NetworkEvent, NetworkMonitor};
use crate::error::NetctlResult;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};

#[cfg(feature = "dbus-nm")]
/// Integrate network monitor with D-Bus signals
pub async fn integrate_network_monitor_with_dbus(
    monitor: Arc<NetworkMonitor>,
    nm_dbus: Arc<NetworkManagerDBus>,
    dbus_conn: Arc<zbus::Connection>,
) -> NetctlResult<()> {
    info!("Integrating network monitor with D-Bus");

    let mut event_rx = monitor.subscribe();

    // Spawn task to handle network events and emit D-Bus signals
    tokio::spawn(async move {
        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    if let Err(e) = handle_network_event(event, &nm_dbus, &dbus_conn).await {
                        warn!("Failed to handle network event: {}", e);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!("Network event receiver lagged, {} events skipped", skipped);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    info!("Network monitor channel closed");
                    break;
                }
            }
        }
    });

    Ok(())
}

#[cfg(feature = "dbus-nm")]
/// Handle a network event and emit corresponding D-Bus signals
async fn handle_network_event(
    event: NetworkEvent,
    nm_dbus: &NetworkManagerDBus,
    dbus_conn: &zbus::Connection,
) -> NetctlResult<()> {
    match event {
        NetworkEvent::InterfaceAdded { index, name } => {
            info!("Handling InterfaceAdded event: {} (index {})", name, index);

            let device_path = format!("/org/freedesktop/NetworkManager/Devices/{}", index);
            let device = DeviceInfo {
                path: device_path.clone(),
                interface: name.clone(),
                device_type: 1, // TYPE_ETHERNET
                state: DeviceState::Disconnected,
                ip4_address: None,
                ip6_address: None,
            };

            nm_dbus.add_device(device).await;

            // Emit DeviceAdded signal
            if let Err(e) = signals::emit_device_added(dbus_conn, &device_path).await {
                warn!("Failed to emit DeviceAdded signal: {}", e);
            }
        }

        NetworkEvent::InterfaceRemoved { index, name } => {
            info!("Handling InterfaceRemoved event: {} (index {})", name, index);

            let device_path = format!("/org/freedesktop/NetworkManager/Devices/{}", index);
            nm_dbus.remove_device(&device_path).await;

            // Emit DeviceRemoved signal
            if let Err(e) = signals::emit_device_removed(dbus_conn, &device_path).await {
                warn!("Failed to emit DeviceRemoved signal: {}", e);
            }
        }

        NetworkEvent::InterfaceStateChanged { index, name, is_up } => {
            info!("Handling InterfaceStateChanged event: {} -> {}", name, if is_up { "up" } else { "down" });

            let device_path = format!("/org/freedesktop/NetworkManager/Devices/{}", index);
            let new_state = if is_up {
                DeviceState::Activated
            } else {
                DeviceState::Disconnected
            };

            if let Err(e) = nm_dbus.update_device_state(&device_path, new_state).await {
                warn!("Failed to update device state: {}", e);
            }

            // Emit StateChanged signal
            let state_value = if is_up { 70 } else { 30 }; // CONNECTED_GLOBAL or DISCONNECTED
            if let Err(e) = signals::emit_state_changed(dbus_conn, state_value).await {
                warn!("Failed to emit StateChanged signal: {}", e);
            }
        }

        NetworkEvent::InterfaceAddressChanged { index, name, address } => {
            info!("Handling InterfaceAddressChanged event: {} -> {}", name, address);

            let device_path = format!("/org/freedesktop/NetworkManager/Devices/{}", index);

            // Update device IP address
            if let Some(mut device) = nm_dbus.get_device(&device_path).await {
                if address.contains(':') {
                    device.ip6_address = Some(address.clone());
                } else {
                    device.ip4_address = Some(address.clone());
                }
                nm_dbus.add_device(device).await;
            }

            // Emit PropertiesChanged signal
            let mut props = std::collections::HashMap::new();
            props.insert(
                "Ip4Address".to_string(),
                zbus::zvariant::Value::new(address.clone()),
            );
            if let Err(e) = signals::emit_properties_changed(dbus_conn, props).await {
                warn!("Failed to emit PropertiesChanged signal: {}", e);
            }
        }

        NetworkEvent::LinkPropertiesChanged { index, name } => {
            info!("Handling LinkPropertiesChanged event: {} (index {})", name, index);

            // Emit PropertiesChanged signal
            let props = std::collections::HashMap::new();
            if let Err(e) = signals::emit_properties_changed(dbus_conn, props).await {
                warn!("Failed to emit PropertiesChanged signal: {}", e);
            }
        }
    }

    Ok(())
}
