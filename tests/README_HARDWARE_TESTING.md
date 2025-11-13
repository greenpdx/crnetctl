# Hardware Testing Guide for nccli

This guide describes how to set up and perform hardware testing for nccli using multiple machines to test real network operations.

## Overview

Hardware testing validates nccli functionality with real network hardware, including:
- Physical network interfaces
- WiFi adapters and connections
- Multi-machine networking scenarios
- Real DHCP, DNS, and routing operations

## Test Environment Requirements

### Minimum Setup

**2 Machines Required:**
1. **Test Controller** (Machine A) - Runs nccli commands
2. **Test Target** (Machine B) - Acts as network peer/client

**Network Hardware:**
- At least one Ethernet interface on each machine
- At least one WiFi adapter on Machine A (for AP/WiFi tests)
- One WiFi adapter on Machine B (for client tests)
- Network switch or crossover cable for wired tests

### Recommended Setup

**3+ Machines:**
1. **Test Controller** (Machine A) - Primary test machine
2. **WiFi Client** (Machine B) - WiFi connection tests
3. **Network Client** (Machine C) - Additional network operations
4. **Test Observer** (Optional) - Monitoring and validation

## Hardware Test Topology

```
┌─────────────────────────────────────────────────────────────┐
│                    Test Network Topology                     │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  Machine A (Test Controller)                                 │
│  ┌───────────────────────────────────┐                      │
│  │  eth0: 192.168.100.1/24          │                      │
│  │  wlan0: WiFi AP (10.42.0.1/24)   │                      │
│  │  Runs: nccli, hostapd, DHCP      │                      │
│  └───────┬──────────────┬────────────┘                      │
│          │              │                                     │
│   Ethernet │              WiFi│                               │
│          │              │                                     │
│  ┌───────┴──────────┐   │  ┌────────────────────────┐       │
│  │  Network Switch  │   │  │   Machine B (Client)   │       │
│  │                  │   │  │  wlan0: WiFi Client    │       │
│  └───────┬──────────┘   └──│  Tests: Connection,    │       │
│          │                  │  Scan, Data transfer   │       │
│          │                  └────────────────────────┘       │
│  ┌───────┴──────────────┐                                    │
│  │  Machine C (Client)  │                                    │
│  │  eth0: DHCP client   │                                    │
│  │  Tests: Routing, DNS │                                    │
│  └──────────────────────┘                                    │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

## Pre-Test Setup

### Machine A (Test Controller) Setup

```bash
# Install required packages
sudo apt-get update
sudo apt-get install -y iproute2 iw hostapd dnsmasq iperf3

# Build nccli
cd /path/to/LnxNetCtl
cargo build --release --bin nccli
sudo cp target/release/nccli /usr/local/bin/

# Verify installation
nccli --help

# Create test configuration directory
sudo mkdir -p /etc/crrouter/netctl
sudo chmod 755 /etc/crrouter/netctl
```

### Machine B (WiFi Client) Setup

```bash
# Install required packages
sudo apt-get update
sudo apt-get install -y iw wpasupplicant iperf3

# Ensure WiFi interface is available
ip link show | grep wlan
```

### Machine C (Network Client) Setup

```bash
# Install testing tools
sudo apt-get update
sudo apt-get install -y iputils-ping traceroute iperf3

# Configure for DHCP
sudo dhclient eth0
```

## Test Categories

### 1. Basic Interface Tests

Test basic interface management operations.

**Test 1.1: List All Devices**
```bash
# Machine A
nccli device status

# Expected: Shows all network interfaces
# Verify: eth0, wlan0, lo are listed
```

**Test 1.2: Show Device Details**
```bash
# Machine A
nccli device show eth0
nccli device show wlan0

# Expected: Detailed info including MAC, MTU, IPs
# Verify: All fields populated correctly
```

**Test 1.3: Interface Up/Down**
```bash
# Machine A
nccli device disconnect eth1
ip link show eth1 | grep DOWN

nccli device connect eth1
ip link show eth1 | grep UP

# Expected: Interface state changes correctly
```

### 2. WiFi Scanning Tests

Test WiFi scanning and detection.

**Test 2.1: WiFi Device List**
```bash
# Machine A
nccli device wifi list

# Expected: Shows available WiFi networks
# Verify: Nearby SSIDs are detected
```

**Test 2.2: WiFi Scan with Specific Interface**
```bash
# Machine A
nccli device wifi list --ifname wlan0

# Expected: Scan results from wlan0
# Verify: Signal strength, channel, BSSID shown
```

**Test 2.3: Terse WiFi Output**
```bash
# Machine A
nccli -t device wifi list

# Expected: Machine-readable output
# Verify: Format is BSSID:SSID:Mode:Signal:*
```

### 3. WiFi Hotspot (AP) Tests

Test WiFi Access Point functionality.

**Test 3.1: Create WiFi Hotspot**
```bash
# Machine A
nccli device wifi hotspot \
  --ifname wlan0 \
  --ssid "nccli-test-ap" \
  --password "testpass123" \
  --channel 6

# Wait 5 seconds for AP to start

# Machine B
# Scan for the AP
sudo iw dev wlan0 scan | grep "nccli-test-ap"

# Expected: Machine B sees the AP
```

**Test 3.2: Connect Client to Hotspot**
```bash
# Machine B
# Create wpa_supplicant config
cat > /tmp/wpa_test.conf <<EOF
network={
    ssid="nccli-test-ap"
    psk="testpass123"
}
EOF

sudo wpa_supplicant -B -i wlan0 -c /tmp/wpa_test.conf
sleep 3
sudo dhclient wlan0

# Verify connection
ip addr show wlan0 | grep "inet "
ping -c 3 10.42.0.1

# Expected: Client gets IP and can ping AP
```

**Test 3.3: Test Data Transfer Through AP**
```bash
# Machine A
iperf3 -s

# Machine B
iperf3 -c 10.42.0.1 -t 10

# Expected: Data transfers successfully
# Verify: Throughput > 1 Mbps
```

### 4. Connection Management Tests

Test connection configuration and management.

**Test 4.1: Add Ethernet Connection**
```bash
# Machine A
nccli connection add \
  --type ethernet \
  --con-name test-eth \
  --ifname eth1 \
  --ip4 192.168.200.10/24 \
  --gw4 192.168.200.1

# Verify config created
ls -la /etc/crrouter/netctl/test-eth.nctl
cat /etc/crrouter/netctl/test-eth.nctl

# Expected: Config file created with correct settings
```

**Test 4.2: Add WiFi Connection**
```bash
# Machine A
nccli connection add \
  --type wifi \
  --con-name test-wifi \
  --ifname wlan0 \
  --ssid "TestNetwork" \
  --password "testpassword" \
  --ip4 auto

# Verify config
cat /etc/crrouter/netctl/test-wifi.nctl

# Expected: WiFi config with WPA-PSK security
```

**Test 4.3: Activate Connection**
```bash
# Machine A
nccli connection up test-eth

# Verify
ip addr show eth1 | grep 192.168.200.10

# Expected: Interface has configured IP
```

**Test 4.4: Deactivate Connection**
```bash
# Machine A
nccli connection down test-eth

# Expected: Success message
```

**Test 4.5: List Connections**
```bash
# Machine A
nccli connection show

# Expected: Shows all configured connections
# Verify: test-eth and test-wifi are listed
```

**Test 4.6: Delete Connection**
```bash
# Machine A
nccli connection delete test-eth

# Verify
ls /etc/crrouter/netctl/test-eth.nctl

# Expected: File deleted, command succeeds
```

### 5. Network Connectivity Tests

Test actual network communication.

**Test 5.1: Ethernet Communication**
```bash
# Machine A - Set static IP
nccli connection add \
  --type ethernet \
  --con-name test-direct \
  --ifname eth0 \
  --ip4 192.168.100.1/24

nccli connection up test-direct

# Machine C - Set static IP
sudo ip addr add 192.168.100.2/24 dev eth0
sudo ip link set eth0 up

# Test connectivity
# Machine A
ping -c 5 192.168.100.2

# Machine C
ping -c 5 192.168.100.1

# Expected: Both machines can ping each other
```

**Test 5.2: DHCP Server Test**
```bash
# Machine A - Start DHCP server (using netctl dhcp command)
# Note: This uses the underlying netctl command
netctl dhcp start eth0 \
  --range-start 192.168.100.10 \
  --range-end 192.168.100.50 \
  --gateway 192.168.100.1 \
  --dns 8.8.8.8

# Start dora DHCP server
sudo dora -c /run/crrouter/netctl/dora.yaml &

# Machine C - Request DHCP
sudo dhclient -r eth0
sudo dhclient eth0

# Verify
ip addr show eth0 | grep "inet 192.168.100"

# Expected: Machine C gets IP in range 192.168.100.10-50
```

### 6. Radio Control Tests

Test WiFi radio on/off functionality.

**Test 6.1: WiFi Radio Status**
```bash
# Machine A
nccli radio wifi

# Expected: Shows "enabled"
```

**Test 6.2: Turn WiFi Off**
```bash
# Machine A
nccli radio wifi off

# Verify
ip link show wlan0 | grep DOWN

# Expected: WiFi interface is down
```

**Test 6.3: Turn WiFi On**
```bash
# Machine A
nccli radio wifi on

# Verify
ip link show wlan0 | grep UP

# Expected: WiFi interface comes back up
```

### 7. Output Format Tests

Test different output modes.

**Test 7.1: Terse Mode**
```bash
# Machine A
nccli -t general status
nccli -t device status
nccli -t connection show

# Expected: Colon-separated machine-readable output
```

**Test 7.2: Tabular Mode**
```bash
# Machine A
nccli -m tabular device status

# Expected: Table format with headers
```

**Test 7.3: Pretty Mode**
```bash
# Machine A
nccli -p device show eth0

# Expected: Human-readable formatted output
```

### 8. Stress and Performance Tests

Test system under load.

**Test 8.1: Rapid Connection Changes**
```bash
# Machine A
for i in {1..20}; do
  echo "Iteration $i"
  nccli device disconnect eth1
  sleep 1
  nccli device connect eth1
  sleep 1
done

# Expected: All operations succeed without errors
```

**Test 8.2: Multiple WiFi Scans**
```bash
# Machine A
for i in {1..10}; do
  echo "Scan $i"
  nccli device wifi list > /tmp/scan_$i.txt
  sleep 2
done

# Verify scans completed
ls /tmp/scan_*.txt | wc -l

# Expected: 10 scan files created
```

**Test 8.3: High Throughput Test**
```bash
# Machine A
iperf3 -s &

# Machine B (connected to Machine A's hotspot)
iperf3 -c 10.42.0.1 -t 60 -P 4

# Expected: Sustained data transfer for 60 seconds
# Verify: No connection drops
```

### 9. Error Handling Tests

Test error conditions and recovery.

**Test 9.1: Invalid Interface**
```bash
# Machine A
nccli device show nonexistent-iface

# Expected: Error message about interface not found
```

**Test 9.2: Invalid Connection**
```bash
# Machine A
nccli connection up nonexistent-connection

# Expected: Error message about connection not found
```

**Test 9.3: Permission Tests**
```bash
# Machine A (as non-root user)
nccli device connect eth0

# Expected: May fail with permission error
# This tests proper permission handling
```

## Automated Test Script

See `hardware_test_suite.sh` for an automated test runner that executes all tests sequentially.

## Test Results Documentation

### Recording Results

Create a test results file for each test run:

```bash
# Start test session
echo "Test Session: $(date)" > test_results.txt
echo "nccli version: $(nccli --version)" >> test_results.txt
echo "Machine: $(hostname)" >> test_results.txt
echo "" >> test_results.txt

# Run tests and capture results
./hardware_test_suite.sh 2>&1 | tee -a test_results.txt
```

### Expected Success Criteria

- ✅ All interface commands execute without errors
- ✅ WiFi scanning detects nearby networks
- ✅ WiFi hotspot allows client connections
- ✅ Connection configuration persists correctly
- ✅ Network communication works between machines
- ✅ DHCP server provides IP addresses
- ✅ Radio controls affect WiFi state
- ✅ All output formats are parseable
- ✅ Error messages are clear and helpful

## Troubleshooting

### WiFi Tests Fail

```bash
# Check WiFi adapter status
iw dev
rfkill list

# Unblock if needed
sudo rfkill unblock wifi

# Check for conflicting services
sudo systemctl stop NetworkManager
sudo systemctl stop wpa_supplicant
```

### DHCP Tests Fail

```bash
# Check if DHCP server is running
ps aux | grep dora

# Check firewall
sudo iptables -L -n

# Check interface has IP
ip addr show eth0
```

### Permission Errors

```bash
# Run with sudo for network operations
sudo nccli device connect eth0

# Or add capabilities
sudo setcap cap_net_admin,cap_net_raw+ep /usr/local/bin/nccli
```

## Multi-Site Testing

For testing across different network environments:

### Home Network
- Test WiFi client connectivity
- Test DHCP client operations
- Test internet connectivity

### Lab Network
- Test multiple VLANs
- Test complex routing scenarios
- Test with managed switches

### Isolated Network
- Test without internet access
- Test pure peer-to-peer scenarios
- Test network bootstrapping

## Continuous Testing

### Daily Test Suite

Run these tests daily to catch regressions:

```bash
#!/bin/bash
# daily_tests.sh

nccli general status
nccli device status
nccli device wifi list
nccli connection show
```

### Weekly Full Test

Run complete hardware test suite weekly:

```bash
#!/bin/bash
# weekly_full_test.sh

./hardware_test_suite.sh --full
```

## Reporting Issues

When reporting issues from hardware testing:

1. Include test results file
2. Provide machine configurations
3. Include network topology diagram
4. Capture any error logs
5. Describe expected vs actual behavior

## Safety and Cleanup

### Before Testing

```bash
# Backup network configuration
sudo cp -r /etc/network /etc/network.backup
sudo cp -r /etc/crrouter /etc/crrouter.backup
```

### After Testing

```bash
# Remove test connections
for conn in /etc/crrouter/netctl/test-*.nctl; do
  nccli connection delete $(basename $conn .nctl)
done

# Stop test services
sudo killall hostapd
sudo killall dora

# Reset interfaces
sudo ip addr flush dev eth1
sudo ip link set eth1 down
```

## See Also

- `hardware_test_suite.sh` - Automated test runner
- `test_scenarios.md` - Additional test scenarios
- `../docs/nccli.md` - nccli user documentation
