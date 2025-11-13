# nccli - Network Control CLI

`nccli` is a comprehensive command-line interface for network management. It provides complete network device and connection management using the netctl backend.

## Features

- **Connection management**: Create, modify, activate, and delete network connections
- **Device management**: Control network interfaces, WiFi devices, and more
- **WiFi support**: Scan, connect, and create hotspots
- **Multiple output formats**: Terse, tabular, and multiline modes
- **Based on LnxNetCtl**: Uses the robust netctl library for all operations

## Command Structure

```bash
nccli [OPTIONS] <COMMAND>
```

### Available Commands

- **general** - Show overall status and system information
  - `status` - Show network system status
  - `hostname` - Get or set system hostname
  - `permissions` - Show current user capabilities
  - `logging` - Get or set logging level and domains

- **networking** - Overall networking control
  - `on` - Enable networking (all interfaces up)
  - `off` - Disable networking (all interfaces down)
  - `connectivity` - Get network connectivity state

- **radio** - Radio switch control
  - `all` - Show all radio switches status
  - `wifi` - Get or set WiFi radio state

- **connection** - Network connection management
  - `show` - List configured connections
  - `up` - Activate a connection
  - `down` - Deactivate a connection
  - `add` - Add a new connection
  - `modify` - Modify an existing connection
  - `edit` - Edit a connection interactively
  - `delete` - Delete a connection
  - `reload` - Reload all connection files
  - `load` - Load or reload a connection file
  - `import` - Import an external configuration
  - `export` - Export a connection
  - `clone` - Clone a connection

- **device** - Network device management
  - `status` - Show device status
  - `show` - Show detailed device information
  - `set` - Set device properties
  - `connect` - Connect a device
  - `reapply` - Reapply connection to device
  - `modify` - Modify active connection
  - `disconnect` - Disconnect a device
  - `delete` - Delete a software device
  - `monitor` - Monitor device activity
  - `wifi` - Manage WiFi devices
  - `lldp` - Show LLDP neighbors

- **monitor** - Monitor network activity

## WiFi Device Commands

The `device wifi` subcommand provides comprehensive WiFi management:

- `list` - List available WiFi access points
- `connect` - Connect to a WiFi network
- `hotspot` - Create WiFi hotspot
- `radio` - Turn WiFi on or off

## Output Options

- `-t, --terse` - Terse output mode (machine-readable)
- `-p, --pretty` - Pretty output mode (human-readable, default)
- `-m, --mode <MODE>` - Output mode: tabular, multiline, or terse
- `-f, --fields <FIELDS>` - Specify fields to output (comma-separated)
- `-c, --colors <COLOR>` - Use colors in output (yes/no/auto)

## Examples

### Show general status
```bash
nccli
# or explicitly
nccli general status
```

### List all connections
```bash
nccli connection show
```

### Create a new WiFi connection
```bash
nccli connection add --type wifi --con-name MyWiFi --ifname wlan0 \
  --ssid "MyNetwork" --password "MyPassword" --ip4 auto
```

### List all devices
```bash
nccli device status
```

### Show device details
```bash
nccli device show eth0
```

### Scan for WiFi networks
```bash
nccli device wifi list
```

### Connect to a WiFi network
```bash
nccli device wifi connect "NetworkName" --password "password"
```

### Create a WiFi hotspot
```bash
nccli device wifi hotspot --ssid "MyHotspot" --password "mypassword"
```

### Enable/disable networking
```bash
nccli networking on
nccli networking off
```

### Control WiFi radio
```bash
nccli radio wifi on
nccli radio wifi off
```

### Activate a connection
```bash
nccli connection up MyConnection
```

### Deactivate a connection
```bash
nccli connection down MyConnection
```

### Monitor network changes
```bash
nccli monitor
```

### Terse output (machine-readable)
```bash
nccli -t device status
```

## Connection Configuration Files

Connections are stored in `/etc/crrouter/netctl/` in the NCTL format (`.nctl` files). These are TOML-like configuration files that define connection parameters.

Example WiFi connection file (`MyWiFi.nctl`):
```ini
[connection]
name = "MyWiFi"
type = "wifi"
interface-name = "wlan0"
autoconnect = true

[wifi]
ssid = "MyNetwork"
mode = "infrastructure"

[wifi-security]
key-mgmt = "wpa-psk"
psk = "MyPassword"

[ipv4]
method = "auto"
```

## Security

- All input is validated to prevent command injection
- Configuration files are properly sanitized
- MAC addresses, hostnames, and interface names are validated
- Network operations require appropriate permissions

## See Also

- `netctl` - The underlying network control tool
- Connection configuration examples in `/usr/share/doc/netctl/examples/`
