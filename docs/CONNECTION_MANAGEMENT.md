# Connection Management with netctl

This guide explains how to use netctl to manage network connections through configuration files. netctl reads `.nctl` configuration files and automatically controls WiFi, DHCP client (crdhcpc), and static IP configuration.

## Overview

netctl provides a complete connection management system that:

1. **Reads `.nctl` configuration files** from `/etc/netctl/connections/`
2. **Activates WiFi connections** using wpa_supplicant
3. **Controls crdhcpc** DHCP client automatically
4. **Configures static IPs** when needed
5. **Manages connection lifecycle** (activate, deactivate, monitor)

## Quick Start

### 1. Create a WiFi Connection with DHCP

Create `/etc/netctl/connections/home-wifi.nctl`:

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
method = "auto"  # This triggers crdhcpc DHCP client

[ipv6]
method = "auto"
```

### 2. Activate the Connection

```bash
# Activate the connection
sudo netctl connection up home-wifi

# Check active connections
netctl connection active

# View connection details
netctl connection show home-wifi
```

**What happens when you activate:**
1. netctl brings interface `wlan0` up
2. Connects to "MyHomeNetwork" via wpa_supplicant
3. Starts crdhcpc DHCP client on wlan0
4. Obtains IP address automatically

## Connection Configuration

### Ethernet with DHCP

`/etc/netctl/connections/wired.nctl`:

```toml
[connection]
name = "Wired Connection"
uuid = "550e8400-e29b-41d4-a716-446655440001"
type = "ethernet"
autoconnect = true
interface-name = "eth0"

[ipv4]
method = "auto"  # DHCP via crdhcpc
```

### WiFi with Static IP

`/etc/netctl/connections/office-wifi.nctl`:

```toml
[connection]
name = "Office WiFi"
uuid = "550e8400-e29b-41d4-a716-446655440002"
type = "wifi"
autoconnect = false
interface-name = "wlan0"

[wifi]
ssid = "OfficeNetwork"
mode = "infrastructure"

[wifi-security]
key-mgmt = "wpa-psk"
psk = "officepassword"

[ipv4]
method = "manual"  # Static IP
address = "192.168.1.100/24"
gateway = "192.168.1.1"
dns = ["8.8.8.8", "8.8.4.4"]
```

### Open WiFi Network

`/etc/netctl/connections/coffee-shop.nctl`:

```toml
[connection]
name = "Coffee Shop WiFi"
uuid = "550e8400-e29b-41d4-a716-446655440003"
type = "wifi"
autoconnect = false
interface-name = "wlan0"

[wifi]
ssid = "FreeWiFi"
mode = "infrastructure"

# No [wifi-security] section = open network

[ipv4]
method = "auto"
```

## CLI Commands

### List Connections

```bash
# List all available connection configurations
netctl connection list

# Output:
# Available connections:
#   home-wifi
#   office-wifi
#   wired
```

### Show Connection Details

```bash
# Show connection configuration
netctl connection show home-wifi

# Output:
# Connection: Home WiFi
# UUID: 550e8400-e29b-41d4-a716-446655440000
# Type: wifi
# Autoconnect: true
# Interface: wlan0
#
# WiFi:
#   SSID: MyHomeNetwork
#   Mode: infrastructure
#
# IPv4:
#   Method: auto
```

### Activate a Connection

```bash
# Activate by name
sudo netctl connection up home-wifi

# Output:
# Activating connection: home-wifi
# Connection 'home-wifi' activated successfully
```

This will:
1. Bring interface up
2. Connect to WiFi (if wifi type)
3. Start DHCP client (if method=auto)
4. Configure static IP (if method=manual)

### Deactivate a Connection

```bash
# Deactivate by interface name
sudo netctl connection down wlan0

# Output:
# Deactivating connection on: wlan0
# Connection on 'wlan0' deactivated
```

This will:
1. Stop DHCP client (if running)
2. Disconnect WiFi (if wifi type)
3. Bring interface down

### List Active Connections

```bash
# Show all active connections
netctl connection active

# Output:
# Active connections:
#   home-wifi on wlan0 (wifi, DHCP)
#   wired on eth0 (ethernet, DHCP)
```

With JSON output:
```bash
netctl -o json connection active
```

### Auto-Connect

```bash
# Auto-connect all connections with autoconnect=true
sudo netctl connection auto-connect

# This is useful on boot
```

## How It Works

### Connection Activation Flow

```
┌──────────────────────────────────────────────┐
│ netctl connection up home-wifi               │
└───────────────┬──────────────────────────────┘
                │
                ▼
┌───────────────────────────────────────────────┐
│ 1. Load /etc/netctl/connections/home-wifi.nctl│
└───────────────┬───────────────────────────────┘
                │
                ▼
┌───────────────────────────────────────────────┐
│ 2. Bring interface up (ip link set wlan0 up) │
└───────────────┬───────────────────────────────┘
                │
                ▼
┌───────────────────────────────────────────────┐
│ 3. WiFi Connection (wpa_supplicant)          │
│    - Generate wpa_supplicant.conf            │
│    - Start wpa_supplicant on wlan0           │
│    - Wait for association                    │
└───────────────┬───────────────────────────────┘
                │
                ▼
┌───────────────────────────────────────────────┐
│ 4. IP Configuration                           │
│    If method="auto":                          │
│      - Start crdhcpc on wlan0                 │
│      - DHCP obtains IP address                │
│    If method="manual":                        │
│      - Configure static IP                    │
│      - Set gateway and DNS                    │
└───────────────┬───────────────────────────────┘
                │
                ▼
┌───────────────────────────────────────────────┐
│ 5. Connection Active                          │
│    - Stored in active connections             │
│    - Ready for use                            │
└───────────────────────────────────────────────┘
```

### crdhcpc Integration

When `method = "auto"` in `[ipv4]` section:

1. netctl waits 2 seconds for link to stabilize
2. Executes: `crdhcpc -c /etc/dhcp-client.toml start wlan0`
3. crdhcpc performs DHCP DORA cycle
4. IP address is assigned to interface
5. netctl marks DHCP as active in connection state

When deactivating:

1. Executes: `crdhcpc release wlan0` (releases lease)
2. Executes: `crdhcpc stop wlan0` (stops client)
3. Disconnects WiFi (if applicable)
4. Brings interface down

## Configuration File Format

### Complete Example

```toml
[connection]
name = "Connection Name"              # Display name
uuid = "unique-uuid-here"             # Unique identifier
type = "wifi"                         # Type: wifi, ethernet, vpn
autoconnect = true                    # Auto-connect on boot
interface-name = "wlan0"              # Network interface (required)

[wifi]  # Only for type="wifi"
ssid = "NetworkName"                  # WiFi SSID
mode = "infrastructure"               # Mode: infrastructure, adhoc, ap
bssid = "00:11:22:33:44:55"          # Specific AP (optional)
channel = 6                          # WiFi channel (optional)

[wifi-security]  # Only for secured WiFi
key-mgmt = "wpa-psk"                 # Key management: wpa-psk, wpa-eap, none
psk = "password"                     # Pre-shared key
# OR
password = "password"                # Alternative field name

[ethernet]  # Only for type="ethernet"
mac-address = "00:11:22:33:44:55"   # MAC address (optional)
mtu = 1500                           # MTU size (optional)

[ipv4]
method = "auto"                      # Method: auto (DHCP), manual, ignore
address = "192.168.1.100/24"        # IP with prefix (manual only)
gateway = "192.168.1.1"             # Default gateway (manual only)
dns = ["8.8.8.8", "8.8.4.4"]        # DNS servers (manual only)
routes = ["10.0.0.0/8"]             # Additional routes (optional)

[ipv6]
method = "auto"                      # Method: auto, manual, ignore
# Similar fields as ipv4
```

### Field Descriptions

#### [connection] Section

- `name` (required): Human-readable connection name
- `uuid` (required): Unique identifier (use `uuidgen` to generate)
- `type` (required): Connection type - `wifi`, `ethernet`, or `vpn`
- `autoconnect` (optional): Auto-connect on boot (default: false)
- `interface-name` (required): Network interface to use

#### [wifi] Section

- `ssid` (required): WiFi network name
- `mode` (optional): Connection mode (default: infrastructure)
- `bssid` (optional): Specific access point MAC address
- `channel` (optional): WiFi channel number

#### [wifi-security] Section

- `key-mgmt` (required): `wpa-psk` for WPA/WPA2, `none` for open networks
- `psk` or `password` (optional): WiFi password

#### [ipv4] Section

- `method` (required):
  - `auto`: Use DHCP (crdhcpc)
  - `manual`: Static IP configuration
  - `ignore`: Don't configure IPv4
- `address` (manual only): IP address with CIDR prefix (e.g., "192.168.1.100/24")
- `gateway` (manual only): Default gateway IP
- `dns` (manual only): Array of DNS server IPs

## Integration with System Services

### Auto-Connect on Boot

Create a systemd service:

`/etc/systemd/system/netctl-autoconnect.service`:

```ini
[Unit]
Description=netctl Auto-Connect
After=network-pre.target
Before=network.target

[Service]
Type=oneshot
ExecStart=/usr/bin/netctl connection auto-connect
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
```

Enable it:
```bash
sudo systemctl enable netctl-autoconnect.service
sudo systemctl start netctl-autoconnect.service
```

### With Link Monitor

For automatic DHCP on link up, use the link monitor (separate service):

```bash
# The link monitor watches for interface up/down events
# and automatically manages DHCP
netctl-daemon --link-monitor
```

## Troubleshooting

### Connection Won't Activate

```bash
# Check if config file exists
ls -l /etc/netctl/connections/*.nctl

# Verify config syntax
netctl connection show my-connection

# Check interface exists
ip link show

# Try with verbose logging
RUST_LOG=debug netctl connection up my-connection
```

### WiFi Not Connecting

```bash
# Verify wpa_supplicant is installed
which wpa_supplicant

# Check WiFi interface
iw dev

# Test WiFi scan
iw dev wlan0 scan

# Check wpa_supplicant logs
journalctl -u wpa_supplicant
```

### DHCP Not Working

```bash
# Check if crdhcpc is installed
which crdhcpc

# Verify crdhcpc config
cat /etc/dhcp-client.toml

# Test crdhcpc manually
sudo crdhcpc -c /etc/dhcp-client.toml start eth0

# Check crdhcpc status
crdhcpc status eth0

# Check crdhcpc logs
RUST_LOG=debug crdhcpc ...
```

### Check Active State

```bash
# List what's actually running
netctl connection active

# Check interface state
ip addr show

# Check routes
ip route show

# Check DNS
cat /etc/resolv.conf
```

## Best Practices

1. **Use UUIDs**: Generate unique UUIDs for each connection (`uuidgen`)
2. **Descriptive Names**: Use clear, descriptive connection names
3. **Security**: Protect config files with passwords (chmod 600)
4. **Testing**: Test connections manually before setting autoconnect=true
5. **Backups**: Keep backups of working configurations
6. **Documentation**: Comment your configs for complex setups

## Examples

See example configurations in:
- `/usr/share/doc/netctl/examples/wifi-dhcp.nctl`
- `/usr/share/doc/netctl/examples/wifi-wpa.nctl`
- `/usr/share/doc/netctl/examples/ethernet-dhcp.nctl`

## Related Documentation

- [DHCP Client Integration](DHCP_CLIENT_INTEGRATION.md)
- [netctl.nctl(5)](../docs/netctl.nctl.5) - Configuration file format
- [netctl(1)](../docs/netctl.1) - Command reference
