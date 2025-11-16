# D-Bus Test Program Guide

## Overview

The `dbus_test` example program is a comprehensive standalone test utility for exercising the CR D-Bus interface in both **real mode** (connecting to an actual D-Bus service) and **mock mode** (running a simulated D-Bus service).

## Features

- **Dual Mode Operation**: Run in mock mode for testing without a live service, or real mode to test against an actual netctl daemon
- **Comprehensive Testing**: Exercises all D-Bus interfaces (NetworkControl, WiFi, VPN)
- **Signal Testing**: Monitors and tests D-Bus signals emitted by the service
- **Selective Test Suites**: Run all tests or focus on specific interfaces
- **Detailed Logging**: Verbose output shows exactly what's happening during tests

## Building

```bash
# Build the test program
cargo build --example dbus_test

# Build in release mode for better performance
cargo build --release --example dbus_test
```

## Usage

### Basic Usage

```bash
# Run in mock mode (default)
cargo run --example dbus_test

# Run in real mode (connects to actual D-Bus service)
cargo run --example dbus_test -- --mode real

# Run with verbose logging
cargo run --example dbus_test -- --verbose
```

### Running Specific Test Suites

```bash
# Test only NetworkControl interface
cargo run --example dbus_test -- --mode mock --test network-control

# Test only WiFi interface
cargo run --example dbus_test -- --mode mock --test wifi

# Test only VPN interface
cargo run --example dbus_test -- --mode mock --test vpn

# Test only signal emission
cargo run --example dbus_test -- --mode mock --test signals

# Test all interfaces (default)
cargo run --example dbus_test -- --mode mock --test all
```

### Command Line Options

```
Options:
  -m, --mode <MODE>      Operating mode [default: mock] [possible values: mock, real]
  -t, --test <TEST>      Test suite to run [default: all] [possible values: all, network-control, wifi, vpn, signals]
  -v, --verbose          Enable verbose logging
      --service <SERVICE>  D-Bus service name for real mode [default: org.crrouter.NetworkControl]
  -h, --help             Print help
```

## Mock Mode

Mock mode runs a simulated D-Bus service with pre-populated test data. This is useful for:

- Testing the D-Bus interface without requiring root permissions
- Developing and debugging D-Bus client applications
- Verifying D-Bus method calls and signal emissions
- Integration testing

### Mock Data

The mock service is populated with:

**Network Devices:**
- `eth0` - Ethernet device (activated, with IP addresses)
- `wlan0` - WiFi device (disconnected)

**WiFi Access Points:**
- `TestNetwork1` - WPA2, 85% signal strength, 2.4 GHz
- `TestNetwork2` - WPA3, 65% signal strength, 5 GHz
- `OpenNetwork` - No security, 45% signal strength, 2.4 GHz

**VPN Connections:**
- `work-vpn` - OpenVPN (disconnected)
- `home-vpn` - WireGuard (disconnected)

### Example Output (Mock Mode)

```bash
$ cargo run --example dbus_test -- --mode mock

D-Bus Test Program
==================
Mode: Mock
Test Suite: All

Starting D-Bus test in MOCK mode
Starting mock CR D-Bus service...
Mock service started successfully
Populating mock service with test data
Mock data populated successfully

=== Testing NetworkControl Interface (Mock Mode) ===
✓ GetVersion: 0.1.0
✓ GetDevices: 2 devices
  - /org/crrouter/NetworkControl/Devices/eth0
    Interface: Some(String("eth0"))
    DeviceType: Some(U32(1))
    State: Some(U32(100))
    IPv4Address: Some(Str("192.168.1.100"))
    HwAddress: Some(Str("00:11:22:33:44:55"))
  - /org/crrouter/NetworkControl/Devices/wlan0
✓ GetState: 60
✓ GetConnectivity: 4
✓ CheckConnectivity: 4
✓ GetNetworkingEnabled: true
✓ GetWirelessEnabled: true
NetworkControl tests completed

=== Testing WiFi Interface (Mock Mode) ===
✓ GetEnabled: true
✓ IsScanning: false
✓ GetAccessPoints: 3 access points
  AP 1:
    SSID: Some(Str("TestNetwork1"))
    BSSID: Some(Str("00:11:22:33:44:55"))
    Strength: Some(U8(85))
    Security: Some(U32(3))
    Frequency: Some(U32(2437))
✓ GetCurrentSSID: Not connected
✓ Scan: Scan initiated
WiFi tests completed

=== Testing VPN Interface (Mock Mode) ===
✓ GetConnections: 2 VPN connections
  VPN: work-vpn
    Type: Some(U32(1))
    State: Some(U32(1))
    RemoteAddress: Some(Str("vpn.example.com:1194"))
  VPN: home-vpn
    Type: Some(U32(2))
    State: Some(U32(1))
    RemoteAddress: Some(Str("home.example.com:51820"))
VPN tests completed

All mock mode tests completed successfully!
```

## Real Mode

Real mode connects to an actual netctl D-Bus service running on the system. This requires:

1. The netctl daemon to be running with D-Bus support
2. Appropriate D-Bus permissions
3. The service to be registered at `org.crrouter.NetworkControl`

### Running Real Mode Tests

```bash
# Start the netctl daemon (requires root)
sudo netctl --daemon

# In another terminal, run the test program
cargo run --example dbus_test -- --mode real

# Test specific interface
cargo run --example dbus_test -- --mode real --test wifi
```

### Example Output (Real Mode)

```bash
$ cargo run --example dbus_test -- --mode real

D-Bus Test Program
==================
Mode: Real
Test Suite: All

Starting D-Bus test in REAL mode
Connecting to D-Bus service: org.crrouter.NetworkControl
Connected to D-Bus service successfully

=== Testing NetworkControl Interface (Real Mode) ===
✓ API Version: 0.1.0
✓ Found 3 devices
  - /org/crrouter/NetworkControl/Devices/lo
  - /org/crrouter/NetworkControl/Devices/eth0
  - /org/crrouter/NetworkControl/Devices/wlan0
    Interface: Some(String("wlan0"))
    State: Some(U32(100))
    IPv4: Some(Str("192.168.1.50"))
✓ Network State: 60
✓ Connectivity: 4
✓ Networking Enabled: true
✓ Wireless Enabled: true

=== Testing WiFi Interface (Real Mode) ===
✓ WiFi Enabled: true
✓ Scanning in progress: false
✓ Found 12 access points
  AP 1: SSID=Some(Str("MyHomeNetwork")), Strength=Some(U8(92))
  AP 2: SSID=Some(Str("NeighborNetwork")), Strength=Some(U8(45))
  ...
✓ Connected to: MyHomeNetwork

All real mode tests completed successfully!
```

## Testing with dbus-monitor

You can monitor D-Bus signals in real-time while running the tests:

```bash
# In one terminal, start monitoring D-Bus signals
dbus-monitor --system "type='signal',sender='org.crrouter.NetworkControl'"

# In another terminal, run the test program
cargo run --example dbus_test -- --mode mock --test signals
```

You should see signal emissions like:

```
signal time=1234567890.123456 sender=:1.42 -> destination=(null destination) serial=5 path=/org/crrouter/NetworkControl; interface=org.crrouter.NetworkControl; member=StateChanged
   uint32 10

signal time=1234567890.234567 sender=:1.42 -> destination=(null destination) serial=6 path=/org/crrouter/NetworkControl; interface=org.crrouter.NetworkControl; member=StateChanged
   uint32 60
```

## Testing with D-Bus Send

You can also manually invoke D-Bus methods using `dbus-send`:

```bash
# Get API version
dbus-send --system --print-reply \
  --dest=org.crrouter.NetworkControl \
  /org/crrouter/NetworkControl \
  org.crrouter.NetworkControl.GetVersion

# Get devices
dbus-send --system --print-reply \
  --dest=org.crrouter.NetworkControl \
  /org/crrouter/NetworkControl \
  org.crrouter.NetworkControl.GetDevices

# Get WiFi access points
dbus-send --system --print-reply \
  --dest=org.crrouter.NetworkControl \
  /org/crrouter/NetworkControl/WiFi \
  org.crrouter.NetworkControl.WiFi.GetAccessPoints
```

## Integration with CI/CD

The mock mode makes this test program ideal for CI/CD pipelines:

```yaml
# Example GitHub Actions workflow
- name: Test D-Bus Interface
  run: |
    cargo run --example dbus_test -- --mode mock --test all
```

## Troubleshooting

### Permission Denied Errors

If you get permission errors in real mode:

```bash
# Grant your user access to the system bus
sudo usermod -a -G messagebus $USER

# Or run with sudo
sudo -E cargo run --example dbus_test -- --mode real
```

### Service Not Found

If the service is not found:

1. Verify the netctl daemon is running:
   ```bash
   systemctl status netctl
   ```

2. Check D-Bus service registration:
   ```bash
   dbus-send --system --print-reply \
     --dest=org.freedesktop.DBus \
     /org/freedesktop/DBus \
     org.freedesktop.DBus.ListNames
   ```

3. Look for `org.crrouter.NetworkControl` in the output

### Mock Service Fails to Start

If mock mode fails:

1. Check if another instance is already running
2. Verify D-Bus system bus is available
3. Check system logs for D-Bus errors

## Development

### Adding New Tests

To add new test cases:

1. Add new methods to the proxy traits (if testing new D-Bus methods)
2. Add test logic to the appropriate test function
3. Update mock data if needed

Example:

```rust
/// Test a new D-Bus method
async fn test_new_method(proxy: &NetworkControlProxy<'_>) -> Result<(), Box<dyn std::error::Error>> {
    info!("Testing new method...");
    let result = proxy.new_method().await?;
    info!("✓ NewMethod returned: {}", result);
    Ok(())
}
```

### Debugging

Enable verbose logging to see detailed D-Bus communication:

```bash
# Rust logging
RUST_LOG=debug cargo run --example dbus_test -- --mode mock --verbose

# D-Bus monitoring
dbus-monitor --system
```

## See Also

- [CR D-Bus Interface Documentation](../src/cr_dbus/mod.rs)
- [NetworkControl API](../src/cr_dbus/network_control.rs)
- [WiFi API](../src/cr_dbus/wifi.rs)
- [VPN API](../src/cr_dbus/vpn.rs)
- [zbus Documentation](https://docs.rs/zbus/)
