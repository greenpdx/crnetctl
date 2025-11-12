# netctl - Network Control Tool

Async network management tool with NetworkManager D-Bus compatibility.

## Dependencies

- **Dora DHCP Server**: Modern DHCP server written in Rust
  - GitHub: https://github.com/greenpdx/CRdoraPub.git
  - Debian/Ubuntu: Not available in standard repos, build from source

- **Unbound DNS Resolver**: Validating, recursive DNS resolver
  - GitHub: https://github.com/NLnetLabs/unbound
  - Debian/Ubuntu: `sudo apt install unbound`

## Features

- **Async Architecture**: Built on tokio for high performance
- **Interface Management**: Control network interfaces (up/down, IP configuration)
- **WiFi Support**: Scan networks, manage connections, regulatory domain
- **Access Point Mode**: Create WiFi hotspots with hostapd
- **DHCP Server**: Configure dora DHCP server
- **DNS Management**: Manage DNS configuration
- **Routing Control**: Manage routing tables
- **NetworkManager Compatible**: D-Bus interface for drop-in replacement
- **Easy Rebranding**: Simple configuration file for customization

## Installation

```bash
cargo build --release
sudo cp target/release/netctl /usr/bin/
```

## Usage

### List network interfaces
```bash
netctl device list
netctl interface list
```

### WiFi operations
```bash
netctl wifi scan wlan0
netctl wifi info wlan0
netctl wifi set-reg US
```

### Access Point
```bash
netctl ap start wlan0 --ssid "MyAP" --password "secret123" --channel 6
netctl ap stop
netctl ap status
```

### DHCP Server
```bash
netctl dhcp start wlan0 --range-start 10.255.24.10 --range-end 10.255.24.250 --gateway 10.255.24.1 --dns 10.255.24.1
```

### Interface Control
```bash
netctl interface up wlan0
netctl interface down wlan0
netctl interface set-ip wlan0 192.168.1.100 --prefix 24
```

## Rebranding

Edit `branding.toml` to customize:
- Project name and display name
- Binary name
- D-Bus service names
- Paths and directories
- Feature flags
- CLI behavior
- User-facing messages

## Testing

```bash
# Run unit tests
cargo test --lib

# Run integration tests
cargo test --test '*'

# Run all tests
cargo test
```

## NetworkManager Compatibility

When built with the `dbus-nm` feature (default), netctl provides a NetworkManager-compatible D-Bus interface at:
- Service: `org.freedesktop.NetworkManager`
- Object: `/org/freedesktop/NetworkManager`

This allows applications expecting NetworkManager to work with netctl.

## License

MIT OR Apache-2.0
