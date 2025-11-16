# netctl - Network Control Tool

Async network management tool with NetworkManager D-Bus compatibility.

## Dependencies

### Required
- **iproute2**: Network interface management
  - Debian/Ubuntu: `sudo apt install iproute2` (usually pre-installed)

- **iw or wireless-tools**: WiFi configuration
  - Debian/Ubuntu: `sudo apt install iw` or `sudo apt install wireless-tools`

- **wpasupplicant**: WPA/WPA2 authentication
  - Debian/Ubuntu: `sudo apt install wpasupplicant`

- **Unbound DNS Resolver**: Validating, recursive DNS resolver
  - GitHub: https://github.com/NLnetLabs/unbound
  - Debian/Ubuntu: `sudo apt install unbound`

### Recommended
- **Dora DHCP Server**: Modern DHCP server written in Rust
  - GitHub: https://github.com/greenpdx/CRdoraPub.git
  - Debian/Ubuntu: Not available in standard repos, build from source

### Optional (for specific features)
- **hostapd**: For access point mode
  - Debian/Ubuntu: `sudo apt install hostapd`

- **openvpn**: For OpenVPN VPN support
  - Debian/Ubuntu: `sudo apt install openvpn`

- **wireguard-tools**: For WireGuard VPN support
  - Debian/Ubuntu: `sudo apt install wireguard-tools`

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

### Option 1: Automated Install Script (Recommended)

```bash
# Run the installation script
sudo ./install.sh
```

The install script will:
- Build the project with cargo
- Install binaries to /usr/bin
- Install systemd service files
- Create configuration directories
- Install man pages and documentation
- Install example configurations
- Set up runtime directories

To uninstall:
```bash
sudo ./uninstall.sh
```

### Option 2: Debian Package

```bash
# Build the Debian package
dpkg-buildpackage -us -uc -b

# Install the package
sudo dpkg -i ../netctl_*.deb
sudo apt --fix-broken install  # If needed for dependencies
```

See `debian/BUILD.md` for detailed build instructions.

### Option 3: Manual Installation

```bash
# Build the project
cargo build --release

# Install binaries
sudo cp target/release/netctl /usr/bin/
sudo cp target/release/nm-converter /usr/bin/
sudo cp target/release/libnccli /usr/bin/

# Install example configuration files
sudo mkdir -p /usr/share/doc/netctl/examples
sudo cp config/examples/*.nctl /usr/share/doc/netctl/examples/
sudo cp config/examples/*.nmconnection /usr/share/doc/netctl/examples/

# Install systemd service files
sudo mkdir -p /lib/systemd/system
sudo cp systemd/*.service /lib/systemd/system/
sudo systemctl daemon-reload

# Install man pages
sudo mkdir -p /usr/share/man/man1 /usr/share/man/man5 /usr/share/man/man7
sudo cp docs/netctl.1 /usr/share/man/man1/
sudo cp docs/nm-converter.1 /usr/share/man/man1/
sudo cp docs/libnccli.1 /usr/share/man/man1/
sudo cp docs/netctl.nctl.5 /usr/share/man/man5/
sudo cp docs/netctl-plugin.7 /usr/share/man/man7/
sudo mandb  # Update man database
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

## Documentation

Complete documentation is available via man pages:

- `man netctl` - Command reference and usage
- `man netctl.nctl` - Configuration file format and examples
- `man nm-converter` - NetworkManager configuration converter
- `man netctl-plugin` - Plugin development guide

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

### Unit and Integration Tests

```bash
# Run unit tests
cargo test --lib

# Run integration tests
cargo test --test '*'

# Run all tests
cargo test
```

### D-Bus Interface Testing

A comprehensive standalone D-Bus test program is available to exercise the CR D-Bus interface in both mock and real modes:

```bash
# Run in mock mode (simulated D-Bus service with test data)
cargo run --example dbus_test -- --mode mock

# Run in real mode (connect to actual netctl daemon)
cargo run --example dbus_test -- --mode real

# Test specific interfaces
cargo run --example dbus_test -- --mode mock --test network-control
cargo run --example dbus_test -- --mode mock --test wifi
cargo run --example dbus_test -- --mode mock --test vpn
cargo run --example dbus_test -- --mode mock --test signals
```

The test program provides:
- **Mock Mode**: Runs a simulated D-Bus service with pre-populated test data (no root required)
- **Real Mode**: Connects to an actual netctl daemon for integration testing
- **Comprehensive Coverage**: Tests all D-Bus methods and signals across NetworkControl, WiFi, and VPN interfaces
- **Detailed Output**: Shows all method calls, return values, and signal emissions

See [docs/DBUS_TEST_GUIDE.md](docs/DBUS_TEST_GUIDE.md) for complete documentation.

## NetworkManager Compatibility

When built with the `dbus-nm` feature (default), netctl provides a NetworkManager-compatible D-Bus interface at:
- Service: `org.freedesktop.NetworkManager`
- Object: `/org/freedesktop/NetworkManager`

This allows applications expecting NetworkManager to work with netctl.

## License

MIT OR Apache-2.0
