# DHCP Client Integration with crdhcpc

This document describes how netctl integrates with the crdhcpc DHCP client to automatically manage DHCP leases on WiFi and ethernet interfaces.

## Overview

netctl now includes comprehensive DHCP client support through integration with [crdhcpc](../crdhpcd/), a comprehensive DHCP client written in Rust. The integration provides:

- **Automatic DHCP startup** when network links come up
- **Link state monitoring** to detect interface up/down events
- **WiFi connection management** with wpa_supplicant
- **Seamless NetworkManager compatibility** through the libcr_compat API

## Architecture

```
┌─────────────────────────────────────────────────┐
│         NetworkManager-Compatible API           │
│              (CRClient)                          │
└─────────────────┬───────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────┐
│         Connection Activation                    │
│  1. Bring interface up                           │
│  2. Connect to WiFi (if wireless)                │
│  3. Start DHCP client (if method=auto)           │
└─────────────────┬───────────────────────────────┘
                  │
        ┌─────────┴──────────┐
        │                    │
┌───────▼────────┐  ┌────────▼──────────┐
│ WpaSupplicant  │  │  DhcpClientController  │
│  Controller    │  │                   │
│                │  │  ┌────────────┐   │
│ - Connect      │  │  │  crdhcpc   │   │
│ - Disconnect   │  │  └────────────┘   │
│ - Config gen   │  │                   │
└────────────────┘  └───────────────────┘
```

## Components

### 1. DhcpClientController (`src/dhcp_client.rs`)

Controls the crdhcpc DHCP client daemon.

**Features:**
- Start/stop DHCP on individual interfaces
- Check DHCP status and lease information
- Graceful error handling (falls back if crdhcpc not installed)

**Example:**
```rust
use netctl::DhcpClientController;

let dhcp = DhcpClientController::new();

// Start DHCP on an interface
dhcp.start("eth0").await?;

// Check status
if let Some(lease) = dhcp.status("eth0").await? {
    println!("IP: {:?}", lease.ip_address);
}

// Stop DHCP
dhcp.stop("eth0").await?;
```

### 2. LinkMonitor (`src/link_monitor.rs`)

Monitors network interface link state and automatically starts DHCP when links come up.

**Features:**
- Poll-based link state detection
- Configurable per-interface DHCP auto-start
- Event-driven architecture
- Automatic DHCP lifecycle management

**Example:**
```rust
use netctl::{LinkMonitor, InterfaceConfig, InterfaceController, DhcpClientController};
use std::sync::Arc;

let interface_ctrl = Arc::new(InterfaceController::new());
let dhcp_client = Arc::new(DhcpClientController::new());

let (monitor, mut events) = LinkMonitor::new(interface_ctrl, dhcp_client);

// Add interface to monitor
monitor.add_interface(InterfaceConfig {
    interface: "eth0".to_string(),
    auto_dhcp: true,
}).await;

// Start monitoring in background
let monitor = Arc::new(monitor);
tokio::spawn(async move {
    monitor.start().await
});

// Listen for events
while let Some(event) = events.recv().await {
    println!("Link state changed: {:?} -> {:?}",
             event.previous_state, event.state);
}
```

### 3. WpaSupplicantController (`src/wpa_supplicant.rs`)

Manages WiFi connections using wpa_supplicant.

**Features:**
- Connect to WPA/WPA2 networks
- Support for open networks
- Automatic configuration generation
- Per-interface wpa_supplicant management

**Example:**
```rust
use netctl::WpaSupplicantController;

let wpa = WpaSupplicantController::new();

// Connect to WiFi network
wpa.connect("wlan0", "MyNetwork", Some("password123")).await?;

// Disconnect
wpa.disconnect("wlan0").await?;
```

### 4. Enhanced CRClient (`src/libcr_compat/client.rs`)

The NetworkManager-compatible client now includes automatic DHCP and WiFi support.

**Connection Activation Flow:**
1. Validate connection and device
2. Bring interface up
3. If WiFi connection:
   - Extract SSID and password from connection settings
   - Connect using wpa_supplicant
4. If IPv4 method is "auto":
   - Start crdhcpc DHCP client
5. If IPv4 method is "manual":
   - Configure static IP addresses
6. Return active connection

**Example:**
```rust
use netctl::libcr_compat::{CRClient, CRConnection};

let client = CRClient::new().await?;

// Create WiFi connection with DHCP
let mut connection = CRConnection::new();
connection.connection.connection_type = "802-11-wireless".to_string();
connection.connection.interface_name = Some("wlan0".to_string());

// Set WiFi settings
let mut wireless = CRSettingWireless::default();
wireless.ssid = b"MyNetwork".to_vec();
connection.wireless = Some(wireless);

// Set WiFi security
let mut wifi_security = CRSetting {
    name: "wifi-security".to_string(),
    properties: HashMap::new(),
};
wifi_security.properties.insert("psk".to_string(), "password123".to_string());
connection.settings.insert("wifi-security".to_string(), wifi_security);

// Set IPv4 to DHCP
let mut ipv4 = CRSettingIP4Config::default();
ipv4.method = "auto".to_string();
connection.ipv4 = Some(ipv4);

// Activate connection
let device = client.get_device_by_iface("wlan0").await?.unwrap();
let active_conn = client.activate_connection(&connection, Some(&device)).await?;
```

## Installation and Setup

### 1. Install crdhcpc

```bash
cd ../crdhpcd
cargo build --release
sudo cp target/release/crdhcpc /usr/local/bin/
sudo mkdir -p /etc
sudo cp dhcp-client-wifi-example.toml /etc/dhcp-client.toml
```

### 2. Configure crdhcpc

Edit `/etc/dhcp-client.toml`:

```toml
[general]
enabled = true
interfaces = []  # Will be managed dynamically by netctl

[dhcpv4]
enabled = true
hostname = "my-device"
send_hostname = true
timeout = 30
retry_count = 3

[security]
validate_server = true
allowed_servers = []  # Empty = allow any
min_lease_time = 300
max_lease_time = 604800
```

### 3. Build netctl with DHCP support

```bash
cd netctl
cargo build --release
```

## Usage Examples

### Example 1: Simple Ethernet with Auto-DHCP

```rust
use netctl::{InterfaceController, DhcpClientController};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let iface_ctrl = Arc::new(InterfaceController::new());
    let dhcp = Arc::new(DhcpClientController::new());

    // Bring interface up
    iface_ctrl.up("eth0").await?;

    // Start DHCP
    dhcp.start("eth0").await?;

    println!("DHCP started on eth0");
    Ok(())
}
```

### Example 2: WiFi with Link Monitoring

```rust
use netctl::{
    LinkMonitor, InterfaceConfig, InterfaceController,
    DhcpClientController, WpaSupplicantController
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let iface_ctrl = Arc::new(InterfaceController::new());
    let dhcp = Arc::new(DhcpClientController::new());
    let wpa = Arc::new(WpaSupplicantController::new());

    // Connect to WiFi first
    wpa.connect("wlan0", "MyNetwork", Some("password")).await?;

    // Set up link monitor
    let (monitor, mut events) = LinkMonitor::new(
        iface_ctrl.clone(),
        dhcp.clone()
    );

    monitor.add_interface(InterfaceConfig {
        interface: "wlan0".to_string(),
        auto_dhcp: true,
    }).await;

    // Start monitoring
    let monitor = Arc::new(monitor);
    let monitor_handle = tokio::spawn(async move {
        monitor.start().await
    });

    // Process events
    tokio::spawn(async move {
        while let Some(event) = events.recv().await {
            println!("Interface {}: {:?} -> {:?}",
                     event.interface, event.previous_state, event.state);
        }
    });

    monitor_handle.await??;
    Ok(())
}
```

### Example 3: NetworkManager-Compatible Connection

```rust
use netctl::libcr_compat::CRClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = CRClient::new().await?;

    // List devices
    let devices = client.get_devices().await?;
    for device in devices {
        println!("Device: {} ({})",
                 device.get_iface(),
                 device.get_device_type());
    }

    // Get WiFi device
    if let Some(device) = client.get_device_by_iface("wlan0").await? {
        // Create connection (see full example above)
        // ...

        // Activate connection (automatically starts WiFi and DHCP)
        let active_conn = client.activate_connection(&connection, Some(&device)).await?;
        println!("Connection activated: {}", active_conn.get_id());
    }

    Ok(())
}
```

## Configuration Files

### WiFi Connection with DHCP (`.nctl` format)

```toml
[connection]
name = "Home WiFi"
uuid = "550e8400-e29b-41d4-a716-446655440000"
type = "wifi"
autoconnect = true
interface-name = "wlan0"

[wifi]
ssid = "MyHomeNetwork"
mode = "infrastructure"

[wifi-security]
key-mgmt = "wpa-psk"
psk = "mypassword123"

[ipv4]
method = "auto"  # This triggers DHCP

[ipv6]
method = "auto"
```

### Ethernet Connection with DHCP

```toml
[connection]
name = "Wired Connection"
uuid = "550e8400-e29b-41d4-a716-446655440001"
type = "ethernet"
autoconnect = true
interface-name = "eth0"

[ipv4]
method = "auto"  # Automatic DHCP

[ipv6]
method = "auto"
```

## Troubleshooting

### DHCP Client Not Starting

1. **Check if crdhcpc is installed:**
   ```bash
   which crdhcpc
   # Should output: /usr/local/bin/crdhcpc
   ```

2. **Check configuration file:**
   ```bash
   ls -l /etc/dhcp-client.toml
   ```

3. **Test crdhcpc manually:**
   ```bash
   sudo crdhcpc -c /etc/dhcp-client.toml start eth0
   ```

4. **Check logs:**
   ```bash
   RUST_LOG=debug cargo run --bin netctl -- interface up eth0
   ```

### WiFi Not Connecting

1. **Check wpa_supplicant:**
   ```bash
   which wpa_supplicant
   # Should be in /usr/sbin/wpa_supplicant
   ```

2. **Check WiFi interface:**
   ```bash
   ip link show wlan0
   ```

3. **Manual connection test:**
   ```bash
   sudo wpa_supplicant -B -i wlan0 -c /tmp/test.conf
   ```

### Link Monitor Not Detecting Changes

1. **Check sysfs access:**
   ```bash
   cat /sys/class/net/eth0/operstate
   cat /sys/class/net/eth0/carrier
   ```

2. **Adjust poll interval** (default is 2 seconds):
   ```rust
   monitor.set_poll_interval(Duration::from_secs(1));
   ```

## Security Considerations

1. **WiFi Passwords:** Stored in connection configuration files. Ensure proper file permissions (600).

2. **DHCP Server Validation:** Configure `allowed_servers` in crdhcpc config to whitelist DHCP servers.

3. **Lease Time Validation:** Set reasonable min/max lease times in crdhcpc config.

4. **wpa_supplicant Config:** Configuration files are written to `/etc/wpa_supplicant/` with restricted permissions.

## Future Enhancements

- [ ] DHCPv6 support
- [ ] Static DNS configuration
- [ ] Lease renewal monitoring
- [ ] Integration with systemd-networkd
- [ ] WPA3 support in wpa_supplicant controller
- [ ] Enterprise WiFi (802.1X) support
- [ ] DHCP event callbacks/hooks
- [ ] Lease information caching

## Related Documentation

- [crdhcpc README](../crdhpcd/README.md)
- [libcr_compat API](LIBCR_COMPAT_API.md)
- [NetworkManager Compatibility](../README.md#networkmanager-compatibility)
