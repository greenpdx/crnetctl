# Access Point Setup Guide

This document describes how to set up a WiFi Access Point using crnetctl with DHCP, DNS, and NTP services.

## Prerequisites

### Required Binaries

Install the following servers to `/usr/local/bin/`:

1. **dora** (DHCP Server)
   ```bash
   cd /tmp
   git clone https://github.com/greenpdx/CRdoraPub.git dora-build
   cd dora-build
   sqlite3 em.db < migrations/20210824204854_initial.sql
   cargo build --release --bin dora
   sudo cp target/release/dora /usr/local/bin/
   ```

2. **hickory-dns** (DNS Server)
   ```bash
   cd /tmp
   git clone https://github.com/greenpdx/hickory-dns.git
   cd hickory-dns
   cargo build --release -p hickory-dns
   sudo cp target/release/hickory-dns /usr/local/bin/
   ```

3. **ntpd-rs** (NTP Server)
   ```bash
   cd /tmp
   git clone https://github.com/greenpdx/ntpd-rs.git
   cd ntpd-rs
   cargo build --release
   sudo cp target/release/ntp-daemon /usr/local/bin/
   sudo cp target/release/ntp-ctl /usr/local/bin/
   ```

## Configuration Files

### DHCP Server Config (`/etc/dora/config.yaml`)

```yaml
wlan0:
  interfaces:
    - wlan0
  ranges:
    - start: 10.255.24.100
      end: 10.255.24.200
  options:
    - opt: 1
      val: !ip 255.255.255.0
    - opt: 3
      val: !ip 10.255.24.1
    - opt: 6
      val: !ip 10.255.24.1
    - opt: 42
      val: !ip 10.255.24.1
  lease_time: 3600
```

### DNS Server Config (`/etc/hickory-dns/named.toml`)

```toml
listen_addrs_ipv4 = ["10.255.24.1"]
listen_addrs_ipv6 = []
listen_port = 53

[[zones]]
zone = "."
zone_type = "Forward"
stores = { type = "forward", name_servers = [{ socket_addr = "8.8.8.8:53", protocol = "udp", trust_negative_responses = true }] }
```

### NTP Server Config (`/etc/ntpd-rs/ntp.server.toml`)

```toml
[[source]]
mode = "server"
address = "time.cloudflare.com:123"

[[source]]
mode = "server"
address = "time.google.com:123"

[[server]]
listen = "10.255.24.1:123"
```

## Setup Procedure

### Step 1: Stop existing WiFi connection

```bash
# Stop DHCP client
sudo /usr/local/bin/crdhcpc stop wlan0

# Disconnect from WiFi
sudo nccli device wifi disconnect
```

### Step 2: Reset wlan0 interface

```bash
# Stop wpa_supplicant if running
sudo pkill -f "wpa_supplicant.*wlan0"

# Bring interface down and up
sudo ip link set wlan0 down
sudo ip link set wlan0 up
```

### Step 3: Start Access Point

```bash
sudo nccli ap start wlan0 --ssid test1 --password "test.123" --ip 10.255.24.1/24
```

### Step 4: Start DHCP Server

```bash
sudo mkdir -p /etc/dora /var/lib/dora
# Create config.yaml as shown above
sudo /usr/local/bin/dora -c /etc/dora/config.yaml -d /var/lib/dora/leases.db
```

### Step 5: Start DNS Server

```bash
sudo mkdir -p /etc/hickory-dns
# Create named.toml as shown above
sudo /usr/local/bin/hickory-dns -c /etc/hickory-dns/named.toml
```

### Step 6: Start NTP Server

```bash
sudo mkdir -p /etc/ntpd-rs
# Create ntp.server.toml as shown above
sudo /usr/local/bin/ntp-daemon -c /etc/ntpd-rs/ntp.server.toml
```

## Verification

```bash
# Check AP is running
iwconfig wlan0

# Check IP address
ip addr show wlan0

# Test DHCP (from client)
# Connect to "test1" WiFi and check for IP in 10.255.24.100-200 range

# Test DNS
dig @10.255.24.1 google.com

# Test NTP
ntpdate -q 10.255.24.1
```

## Teardown

```bash
# Stop services
sudo pkill -f dora
sudo pkill -f hickory-dns
sudo pkill -f ntp-daemon

# Stop AP
sudo nccli ap stop wlan0

# Reset interface
sudo ip link set wlan0 down
sudo ip link set wlan0 up
```
