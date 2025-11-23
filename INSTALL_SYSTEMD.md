# Installing netctld as a systemd service

## Quick Installation

### 1. Build and install the binaries

```bash
# Build release binaries
cargo build --release

# Install binaries (requires sudo)
sudo install -m 755 target/release/netctld /usr/bin/netctld
sudo install -m 755 target/release/nccli /usr/bin/nccli
```

### 2. Install the systemd service

```bash
# Copy service file
sudo cp systemd/netctld.service /etc/systemd/system/

# Create required directories
sudo mkdir -p /etc/netctl/connections
sudo mkdir -p /run/netctl
sudo mkdir -p /var/lib/netctl

# Reload systemd
sudo systemctl daemon-reload
```

### 3. Enable and start the service

```bash
# Enable to start on boot
sudo systemctl enable netctld.service

# Start the service now
sudo systemctl start netctld.service

# Check status
sudo systemctl status netctld.service
```

### 4. Verify D-Bus interface

```bash
# Check that the D-Bus service is registered
busctl list | grep crrouter

# Test with nccli
nccli --use-dbus device wifi list
```

## Development Mode (Running from source directory)

For testing without system installation:

```bash
# Run the daemon manually
sudo ./target/debug/netctld --foreground

# In another terminal, test with nccli
./target/debug/nccli --use-dbus device wifi list
```

## Logs and Troubleshooting

### View service logs
```bash
# Follow logs in real-time
sudo journalctl -u netctld.service -f

# View recent logs
sudo journalctl -u netctld.service -n 50

# View logs since boot
sudo journalctl -u netctld.service -b
```

### Check D-Bus interface
```bash
# List all methods on the D-Bus interface
busctl introspect org.crrouter.NetworkControl /org/crrouter/NetworkControl

# Check WiFi interface
busctl introspect org.crrouter.NetworkControl /org/crrouter/NetworkControl/WiFi
```

### Common issues

**Service fails to start:**
- Check logs: `sudo journalctl -u netctld.service -n 50`
- Verify binary exists: `ls -l /usr/bin/netctld`
- Check permissions: Binary should be executable

**D-Bus connection fails:**
- Verify service is running: `sudo systemctl status netctld.service`
- Check D-Bus registration: `busctl list | grep crrouter`
- Ensure D-Bus system bus is running: `systemctl status dbus.service`

**Permission errors:**
- netctld requires CAP_NET_ADMIN capability to manage network
- Run with sudo or ensure proper capabilities are set

## Uninstallation

```bash
# Stop and disable the service
sudo systemctl stop netctld.service
sudo systemctl disable netctld.service

# Remove service file
sudo rm /etc/systemd/system/netctld.service

# Remove binaries
sudo rm /usr/bin/netctld
sudo rm /usr/bin/nccli

# Reload systemd
sudo systemctl daemon-reload
```

## Comparison with netctl.service

There are now two systemd services:

- **netctld.service**: CR D-Bus interface (`org.crrouter.NetworkControl`)
  - Use with: `nccli --use-dbus`
  - Modern, clean D-Bus API
  - Dedicated daemon binary

- **netctl.service**: NetworkManager compatibility (`org.freedesktop.NetworkManager`)
  - Use with: `nmcli` or other NetworkManager clients
  - Drop-in replacement for NetworkManager
  - Uses `netctl daemon` mode

You can run either service, but not both simultaneously as they provide different D-Bus interfaces.
