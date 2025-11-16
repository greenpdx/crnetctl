# libnm-Compatible API (CR Prefix)

This document describes the libnm-compatible API provided by libnetctl using the `CR` prefix.

## Overview

The libnm-compatible API provides a NetworkManager libnm-like interface for managing network connections and devices. It uses the `CR` prefix (instead of `NM`) for all types and functions, providing the same conceptual API structure as NetworkManager's libnm library.

## Main Components

### CRClient

The main entry point for network management, equivalent to `NMClient` in libnm.

```rust
use netctl::CRClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new client
    let client = CRClient::new().await?;

    // Get version
    println!("Version: {}", client.get_version());

    // Get all devices
    let devices = client.get_devices().await?;

    // Get network state
    let state = client.get_state().await;

    Ok(())
}
```

#### Key Methods

- `new()` / `new_async()` - Create a new client
- `get_devices()` - Get all network devices
- `get_device_by_iface(name)` - Get device by interface name
- `get_active_connections()` - Get all active connections
- `activate_connection(connection, device)` - Activate a connection
- `deactivate_connection(active_connection)` - Deactivate a connection
- `get_state()` - Get current network state
- `get_connectivity()` - Get connectivity state
- `networking_get_enabled()` / `networking_set_enabled()` - Control networking
- `wireless_get_enabled()` / `wireless_set_enabled()` - Control wireless

### CRDevice

Represents a network device, equivalent to `NMDevice` in libnm.

```rust
// Get device information
let iface = device.get_iface();
let device_type = device.get_device_type();
let state = device.get_state();
let mac = device.get_hw_address();
let mtu = device.get_mtu();

// Get IP configuration
if let Some(ip4_config) = device.get_ip4_config().await {
    for addr in ip4_config.get_addresses() {
        println!("IP: {}", addr.to_cidr());
    }
}

// WiFi-specific operations
if device.get_device_type() == CRDeviceType::Wifi {
    let access_points = device.wifi_get_access_points().await?;
    device.wifi_request_scan().await?;
}

// Connect/disconnect
device.disconnect().await?;
```

#### Device Types

- `Ethernet` - Wired Ethernet
- `Wifi` - Wireless WiFi
- `Bridge` - Bridge device
- `Bond` - Bond device
- `Vlan` - VLAN device
- `Tun` - TUN/TAP device
- `Generic` - Generic/other
- And more...

#### Device States

- `Unknown` - State unknown
- `Unmanaged` - Device not managed
- `Unavailable` - Device unavailable
- `Disconnected` - Device disconnected
- `Activated` - Device active
- `Failed` - Device failed
- And more...

### CRConnection

Represents a connection configuration, equivalent to `NMConnection` in libnm.

```rust
use netctl::CRConnection;

// Create a new connection
let mut connection = CRConnection::new();

// Set basic properties
connection.connection.id = "My Connection".to_string();
connection.connection.connection_type = "802-3-ethernet".to_string();

// Configure IPv4
let mut ipv4 = CRSettingIP4Config::default();
ipv4.method = "auto".to_string(); // DHCP

connection.ipv4 = Some(ipv4);

// Verify and normalize
connection.verify()?;
connection.normalize()?;
```

#### Connection Settings

- `CRSettingConnection` - Basic connection settings
- `CRSettingWired` - Wired Ethernet settings
- `CRSettingWireless` - WiFi settings
- `CRSettingIP4Config` - IPv4 configuration
- `CRSettingIP6Config` - IPv6 configuration

### CRActiveConnection

Represents an active connection, equivalent to `NMActiveConnection` in libnm.

```rust
// Get active connection details
let id = active_conn.get_id();
let uuid = active_conn.get_uuid();
let state = active_conn.get_state();
let is_default = active_conn.get_default();

// Get the device
if let Some(device) = active_conn.get_device() {
    println!("Device: {}", device.get_iface());
}

// Get IP configuration
if let Some(ip4_config) = active_conn.get_ip4_config().await {
    println!("Gateway: {:?}", ip4_config.get_gateway());
}
```

### CRAccessPoint

Represents a WiFi access point, equivalent to `NMAccessPoint` in libnm.

```rust
// Get access point information
let ssid = ap.get_ssid_string();
let bssid = ap.get_bssid();
let frequency = ap.get_frequency();
let channel = ap.get_channel();
let strength = ap.get_strength();
let security = ap.get_security_type();

// Check security
if ap.is_secured() {
    println!("Secured with {}", security);
}
```

### CRIPConfig

Represents IP configuration, equivalent to `NMIPConfig` in libnm.

```rust
// Get addresses
for addr in ip_config.get_addresses() {
    println!("Address: {}", addr.get_address());
    println!("Prefix: {}", addr.get_prefix());
    println!("CIDR: {}", addr.to_cidr());
}

// Get gateway and DNS
if let Some(gateway) = ip_config.get_gateway() {
    println!("Gateway: {}", gateway);
}

for dns in ip_config.get_nameservers() {
    println!("DNS: {}", dns);
}

// Get routes
for route in ip_config.get_routes() {
    println!("Route: {}", route.to_string_format());
}
```

## API Mapping

| libnm Type | libnetctl CR Type |
|------------|-------------------|
| NMClient | CRClient |
| NMDevice | CRDevice |
| NMConnection | CRConnection |
| NMRemoteConnection | CRRemoteConnection |
| NMActiveConnection | CRActiveConnection |
| NMAccessPoint | CRAccessPoint |
| NMIPConfig | CRIPConfig |
| NMIPAddress | CRIPAddress |
| NMIPRoute | CRIPRoute |
| NMSetting | CRSetting |
| NMSettingConnection | CRSettingConnection |
| NMSettingWired | CRSettingWired |
| NMSettingWireless | CRSettingWireless |
| NMSettingIP4Config | CRSettingIP4Config |
| NMSettingIP6Config | CRSettingIP6Config |
| NMState | CRState |
| NMConnectivityState | CRConnectivityState |
| NMDeviceType | CRDeviceType |
| NMDeviceState | CRDeviceState |

## Example Usage

See `examples/libcr_compat_example.rs` for a comprehensive example demonstrating:

- Creating a CRClient
- Enumerating devices
- Getting device information
- Checking IP configuration
- Scanning for WiFi networks
- Managing connections

## Running the Example

```bash
cargo run --example libcr_compat_example
```

## Implementation Notes

- The CR API is built on top of libnetctl's existing functionality
- It provides a familiar interface for developers used to NetworkManager's libnm
- The implementation uses async/await (unlike libnm which uses GMainLoop)
- Not all libnm features are implemented - this provides the core functionality
- The CR prefix is used instead of NM to clearly distinguish this as a compatible API

## Differences from libnm

1. **Async/Await**: Uses Rust async/await instead of GLib event loops
2. **Rust Native**: Pure Rust implementation without GObject dependencies
3. **Simplified**: Focuses on core functionality without all libnm features
4. **CR Prefix**: Uses `CR` instead of `NM` to avoid confusion
5. **No D-Bus Dependency**: Direct system integration without requiring NetworkManager daemon

## Benefits

- Familiar API for NetworkManager/libnm developers
- Easy migration from libnm-based code
- No runtime dependency on NetworkManager
- Pure Rust implementation with better performance
- Full async/await support
