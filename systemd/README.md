# Netctl Systemd Integration

This directory contains systemd service unit files for managing netctl at boot time.

## Service Files

### netctl.service
Main netctl daemon service that provides D-Bus interface and manages network connections.

**Installation:**
```bash
sudo cp systemd/netctl.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable netctl.service
sudo systemctl start netctl.service
```

**Features:**
- Provides NetworkManager D-Bus compatibility
- Automatic restart on failure
- Security hardening with capabilities
- Runs as system service with network management privileges

### netctl@.service
Template service for managing individual network connections by name.

**Usage:**
```bash
# Start a specific connection profile
sudo systemctl start netctl@wifi-wpa.service

# Enable a connection to start at boot
sudo systemctl enable netctl@wifi-wpa.service

# Check status
sudo systemctl status netctl@wifi-wpa.service

# Stop a connection
sudo systemctl stop netctl@wifi-wpa.service
```

**Note:** The connection name should match a `.nctl` file in `/etc/netctl/connections/`.

### netctl-auto@.service
Automatic connection management service that monitors an interface and automatically connects to available networks based on configured profiles.

**Usage:**
```bash
# Enable automatic connection management for wlan0
sudo systemctl enable netctl-auto@wlan0.service
sudo systemctl start netctl-auto@wlan0.service

# Check which network is active
sudo systemctl status netctl-auto@wlan0.service

# Disable automatic management
sudo systemctl stop netctl-auto@wlan0.service
sudo systemctl disable netctl-auto@wlan0.service
```

## Quick Start

### Option 1: NetworkManager Replacement (D-Bus mode)
If you want netctl to act as a drop-in replacement for NetworkManager:

```bash
# Stop and disable NetworkManager
sudo systemctl stop NetworkManager
sudo systemctl disable NetworkManager

# Install and enable netctl daemon
sudo cp systemd/netctl.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now netctl.service
```

### Option 2: Profile-based Manual Control
For manual control of specific network profiles:

```bash
# Install the template service
sudo cp systemd/netctl@.service /etc/systemd/system/
sudo systemctl daemon-reload

# Create a connection profile
sudo netctl connection add wifi-wpa /etc/netctl/connections/wifi-wpa.nctl

# Enable and start the connection
sudo systemctl enable --now netctl@wifi-wpa.service
```

### Option 3: Automatic WiFi Management
For automatic WiFi connection management:

```bash
# Install the auto-connection service
sudo cp systemd/netctl-auto@.service /etc/systemd/system/
sudo systemctl daemon-reload

# Enable automatic WiFi management on wlan0
sudo systemctl enable --now netctl-auto@wlan0.service
```

## Directory Structure

Netctl expects the following directories:
- `/etc/netctl/connections/` - Connection profile configuration files (`.nctl`)
- `/etc/netctl/plugins/` - Plugin shared libraries (`.so`)
- `/run/netctl/` - Runtime state and PID files
- `/var/lib/netctl/` - Persistent state and connection history

## Security

All service files include security hardening:
- Restricted filesystem access (ProtectSystem=strict, ProtectHome=yes)
- Minimal capabilities (CAP_NET_ADMIN, CAP_NET_RAW, CAP_NET_BIND_SERVICE)
- Private /tmp directory
- Read-only root filesystem with specific writable paths

## Troubleshooting

### View service logs
```bash
# Main daemon logs
sudo journalctl -u netctl.service -f

# Specific connection logs
sudo journalctl -u netctl@wifi-wpa.service -f

# Auto-connection logs
sudo journalctl -u netctl-auto@wlan0.service -f
```

### Check service status
```bash
sudo systemctl status netctl.service
sudo systemctl status netctl@*.service
sudo systemctl status netctl-auto@*.service
```

### List all netctl services
```bash
systemctl list-units 'netctl*'
```

## Migration from NetworkManager

To migrate from NetworkManager to netctl:

1. Export existing NetworkManager connections:
   ```bash
   # Convert all NetworkManager profiles
   nm-converter -d /etc/NetworkManager/system-connections --output-dir /etc/netctl/connections/
   ```

2. Stop NetworkManager:
   ```bash
   sudo systemctl stop NetworkManager
   sudo systemctl disable NetworkManager
   ```

3. Start netctl:
   ```bash
   sudo systemctl enable --now netctl.service
   ```

4. Verify connections:
   ```bash
   netctl connection list
   ```

## See Also

- Main netctl documentation: `/usr/share/doc/netctl/README.md`
- Configuration examples: `/usr/share/doc/netctl/examples/`
- Plugin development: `/usr/share/doc/netctl/PLUGINS.md`
