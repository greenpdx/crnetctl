# Testing Guide

This directory contains comprehensive tests for the LnxNetCtl project, including the libnccli CLI tool and the crrouter-web REST API.

## Test Types

### 1. Integration Tests

Automated tests that verify CLI functionality without requiring special hardware or root privileges.

**File:** `libnccli_integration_tests.rs`

**Run tests:**
```bash
cargo test --test libnccli_integration_tests
```

**What it tests:**
- Command-line argument parsing
- Help and version output
- Basic command structure
- Output format options
- Error handling
- Command help text

**Requirements:**
- Rust toolchain
- No special privileges needed
- No special hardware needed

### 2. Hardware Tests

Real-world tests using actual network hardware and multiple machines.

**File:** `hardware_test_suite.sh`

**Documentation:** `README_HARDWARE_TESTING.md`

**Run tests:**
```bash
sudo ./tests/hardware_test_suite.sh
```

**What it tests:**
- Physical network interfaces
- WiFi scanning and connections
- Access Point creation
- DHCP operations
- Real network communication
- Multi-machine scenarios

**Requirements:**
- Root privileges
- Physical network interfaces
- WiFi adapter (for WiFi tests)
- Multiple machines (for full suite)

### 3. API Curl Tests

Comprehensive REST API tests for the crrouter-web service using curl.

**File:** `api_curl_tests.sh`

**Run tests:**
```bash
# Start the API server first
cargo run --bin crrouter-web &

# Run the tests
./tests/api_curl_tests.sh

# Or specify a custom server URL
./tests/api_curl_tests.sh http://localhost:3000
```

**What it tests:**
- Health and info endpoints
- Device management (list, get, configure, delete, stats)
- DHCP testing (discover, request, release, test sequence)
- Interface management (legacy API)
- WiFi operations (scanning)
- Error handling and edge cases

**Requirements:**
- Running crrouter-web server
- curl command-line tool
- Network interfaces for testing
- Some tests may require elevated privileges

## Quick Start

### Running All Tests

```bash
# Run integration tests
cargo test

# Run hardware tests (requires root)
sudo ./tests/hardware_test_suite.sh

# Run API curl tests (requires running API server)
cargo run --bin crrouter-web &
sleep 2  # Wait for server to start
./tests/api_curl_tests.sh
pkill crrouter-web  # Stop server after tests

# Run with verbose output
VERBOSE=1 sudo ./tests/hardware_test_suite.sh

# Run with stress tests
STRESS_TEST=1 sudo ./tests/hardware_test_suite.sh
```

### Running Specific Tests

```bash
# Run only integration tests for general commands
cargo test --test libnccli_integration_tests test_general

# Run only device tests
cargo test --test libnccli_integration_tests test_device

# Run only connection tests
cargo test --test libnccli_integration_tests test_connection
```

## Test Environment Setup

### For Integration Tests

No special setup required. Just ensure you have:
```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build libnccli
cargo build --release --bin libnccli
```

### For Hardware Tests

See `README_HARDWARE_TESTING.md` for detailed hardware setup instructions.

**Quick setup:**
```bash
# Install dependencies
sudo apt-get install -y iproute2 iw hostapd dnsmasq iperf3

# Build and install libnccli
cargo build --release --bin libnccli
sudo cp target/release/libnccli /usr/local/bin/

# Create config directory
sudo mkdir -p /etc/crrouter/netctl
sudo chmod 755 /etc/crrouter/netctl

# Run tests
sudo ./tests/hardware_test_suite.sh
```

## Test Coverage

### Integration Tests Coverage

| Category | Tests | Description |
|----------|-------|-------------|
| Help & Version | 4 | Help text, version info |
| General Commands | 6 | Status, hostname, permissions, logging |
| Radio Commands | 3 | WiFi radio control |
| Connection Commands | 7 | Add, modify, delete, show connections |
| Device Commands | 8 | Status, show, connect, disconnect |
| Output Formats | 4 | Terse, tabular, multiline modes |
| Error Handling | 3 | Invalid commands, missing resources |
| **Total** | **35** | |

### Hardware Tests Coverage

| Category | Tests | Description |
|----------|-------|-------------|
| Basic CLI | 6 | Command execution and output |
| Device Management | 8 | Interface up/down, show details |
| Connection Management | 8 | Create, activate, delete connections |
| WiFi Operations | 5 | Scanning, AP creation |
| Networking | 2 | Connectivity checks |
| Output Formats | 3 | Different output modes |
| Error Handling | 3 | Invalid inputs, error messages |
| Stress Tests | 2 | Rapid operations, multiple iterations |
| **Total** | **37** | |

### API Curl Tests Coverage

| Category | Endpoints | Description |
|----------|-----------|-------------|
| Health & Info | 2 | Health check, API documentation |
| Device Management | 7 | List, get, configure, delete devices and stats |
| DHCP Testing | 5 | Discover, request, release, test, sequence |
| Interface Management | 2 | List interfaces, get interface info |
| WiFi Operations | 1 | WiFi network scanning |
| Error Handling | 4 | Invalid endpoints, malformed requests |
| **Total** | **21** | |

## Continuous Integration

### GitHub Actions (Example)

```yaml
name: libnccli Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run integration tests
        run: cargo test --test libnccli_integration_tests
```

### Pre-commit Hook

Create `.git/hooks/pre-commit`:
```bash
#!/bin/bash
cargo test --test libnccli_integration_tests
```

## Test Results and Logging

### Integration Tests

Test results are shown in terminal. To save results:
```bash
cargo test --test libnccli_integration_tests 2>&1 | tee test_results.txt
```

### Hardware Tests

Hardware tests automatically log to `/tmp/libnccli_test_TIMESTAMP.log`:
```bash
sudo ./tests/hardware_test_suite.sh
# Log file path will be shown in output

# View logs
cat /tmp/libnccli_test_*.log
```

## Debugging Failed Tests

### Integration Test Failures

```bash
# Run with verbose output
cargo test --test libnccli_integration_tests -- --nocapture

# Run specific failing test
cargo test --test libnccli_integration_tests test_name -- --nocapture

# Show backtraces
RUST_BACKTRACE=1 cargo test --test libnccli_integration_tests
```

### Hardware Test Failures

```bash
# Run with verbose output
VERBOSE=1 sudo ./tests/hardware_test_suite.sh

# Check specific interface
ip link show eth1

# Check libnccli directly
libnccli device status

# Check for conflicting services
sudo systemctl status NetworkManager
sudo systemctl status wpa_supplicant
```

## Writing New Tests

### Adding Integration Tests

Edit `libnccli_integration_tests.rs`:

```rust
#[test]
fn test_new_feature() {
    libnccli()
        .arg("new-command")
        .assert()
        .success()
        .stdout(predicate::str::contains("expected output"));
}
```

### Adding Hardware Tests

Edit `hardware_test_suite.sh`:

```bash
run_test "New feature test" \
    "$NCCLI new-command | grep -q 'expected'"
```

## Test Best Practices

### DO:
- ✅ Test both success and failure cases
- ✅ Test all output formats (terse, tabular, multiline)
- ✅ Clean up after tests (remove test files, reset interfaces)
- ✅ Document hardware requirements
- ✅ Use meaningful test names
- ✅ Test error messages are helpful

### DON'T:
- ❌ Leave test artifacts in `/etc/crrouter/netctl/`
- ❌ Assume specific network hardware exists
- ❌ Modify production network configurations
- ❌ Run hardware tests without root privileges
- ❌ Forget to restore network state after tests

## Troubleshooting

### Common Issues

**Issue: "Permission denied" during tests**
```bash
# Hardware tests need root
sudo ./tests/hardware_test_suite.sh
```

**Issue: "Interface not found"**
```bash
# Specify your interface
TEST_INTERFACE=eth0 sudo ./tests/hardware_test_suite.sh
```

**Issue: Integration tests fail to find libnccli**
```bash
# Ensure libnccli is built
cargo build --release --bin libnccli

# Check it's in PATH or use full path
export PATH=$PATH:$(pwd)/target/release
```

**Issue: WiFi tests fail**
```bash
# Check WiFi hardware
iw dev

# Unblock WiFi if needed
sudo rfkill unblock wifi

# Stop conflicting services
sudo systemctl stop NetworkManager
```

## Performance Benchmarks

Run performance tests:
```bash
# Time 100 device status calls
time for i in {1..100}; do libnccli device status > /dev/null; done

# Time 100 connection listings
time for i in {1..100}; do libnccli connection show > /dev/null; done
```

Expected performance:
- Device status: < 50ms per call
- Connection show: < 30ms per call
- WiFi scan: 2-5 seconds (hardware dependent)

## Test Reports

Generate test reports:

```bash
# HTML report from cargo test
cargo install cargo2junit
cargo test --test libnccli_integration_tests -- -Z unstable-options --format json | cargo2junit > report.xml

# Convert to HTML (requires junit2html)
junit2html report.xml report.html
```

## Contributing Tests

When contributing new features to libnccli:

1. Add integration tests in `libnccli_integration_tests.rs`
2. Add hardware tests in `hardware_test_suite.sh` if applicable
3. Update test coverage numbers in this README
4. Ensure all existing tests still pass
5. Document any new test requirements

## Further Reading

- **Integration Testing:** See Rust documentation on integration tests
- **Hardware Testing:** See `README_HARDWARE_TESTING.md` for detailed guide
- **CLI Testing:** See `assert_cmd` crate documentation
- **Network Testing:** See Linux networking documentation

## Support

For test-related issues:
1. Check this README
2. Check `README_HARDWARE_TESTING.md`
3. Review test logs
4. Open an issue on GitHub with test results
