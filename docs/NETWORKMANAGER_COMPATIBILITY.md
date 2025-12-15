# NetworkManager Compatibility Analysis for crnetctl

**Author:** Shaun Savage <savages@crmep.com>
**Date:** 2025-12-15

## Overview

This document analyzes packages that depend on NetworkManager and how crnetctl/netctld can provide compatibility to support these programs.

---

## 1. NetworkManager Dependency Categories

### 1.1 Hard Dependencies (Require network-manager package)

| Package | Purpose | D-Bus API Usage |
|---------|---------|-----------------|
| `network-manager-l10n` | Localization files | None (data only) |
| `network-manager-openvpn` | OpenVPN VPN plugin | VPN settings D-Bus API |
| `network-manager-vpnc` | Cisco VPN plugin | VPN settings D-Bus API |
| `network-manager-pptp` | PPTP VPN plugin | VPN settings D-Bus API |
| `network-manager-l2tp` | L2TP VPN plugin | VPN settings D-Bus API |
| `network-manager-strongswan` | IPsec/IKEv2 plugin | VPN settings D-Bus API |
| `network-manager-openconnect` | Cisco AnyConnect plugin | VPN settings D-Bus API |
| `network-manager-sstp` | SSTP VPN plugin | VPN settings D-Bus API |
| `nm-connection-editor` | GUI connection editor | Full D-Bus API |
| `network-manager-applet` | System tray applet | Full D-Bus API |
| `plasma-nm` | KDE network applet | Full D-Bus API (via libnm) |

### 1.2 Soft/Optional Dependencies

| Package | Purpose | Usage Pattern |
|---------|---------|---------------|
| `wpasupplicant` | WiFi authentication | Can work standalone |
| `netplan-generator` | Netplan integration | Alternative to NM |
| `chrony` | NTP client | Optional NM integration |
| `resolvconf` | DNS resolver | Optional NM hook |

### 1.3 Desktop Environment Dependencies

| Package | Environment | Integration Level |
|---------|-------------|-------------------|
| `gnome`, `gnome-core` | GNOME | Deep integration |
| `phosh-core` | Phosh (mobile) | Requires NM |
| `plasma-nm` | KDE Plasma | Full NM client |
| `lomiri-indicator-network` | Lomiri/Unity8 | NM indicator |

### 1.4 Raspberry Pi Specific

| Package | Purpose | Criticality |
|---------|---------|-------------|
| `raspberrypi-net-mods` | Pi network config | Medium |
| `rpd-common` | Pi Desktop common | Low |
| `rpi-usb-gadget` | USB OTG networking | Low |

---

## 2. NetworkManager D-Bus API Analysis

### 2.1 Main Service

- **Service:** `org.freedesktop.NetworkManager`
- **Object Path:** `/org/freedesktop/NetworkManager`

### 2.2 Core Interfaces Required

#### org.freedesktop.NetworkManager (Main Interface)

| Method | crnetctl Status | Notes |
|--------|----------------|-------|
| `GetDevices()` | ✅ Implemented | Returns device paths |
| `GetAllDevices()` | ✅ Implemented | Same as GetDevices |
| `GetDeviceByIpIface(iface)` | ✅ Implemented | Find device by name |
| `ActivateConnection(conn, dev, obj)` | ✅ Implemented | Activate connection |
| `DeactivateConnection(active_conn)` | ✅ Implemented | Deactivate connection |
| `state` property | ✅ Implemented | Global network state |
| `connectivity` property | ✅ Implemented | Connectivity state |
| `CheckConnectivity()` | ✅ Implemented | Check internet |
| `Version` property | ✅ Implemented | Returns "1.46.0" |
| `NetworkingEnabled` property | ✅ Implemented | Always true |
| `WirelessEnabled` property | ✅ Implemented | Always true |
| `AddAndActivateConnection()` | ⚠️ Partial | Needs full impl |
| `AddAndActivateConnection2()` | ❌ Not implemented | New API |
| `Reload(flags)` | ❌ Not implemented | Config reload |
| `GetPermissions()` | ❌ Not implemented | PolicyKit |
| `Sleep(sleep)` | ❌ Not implemented | Suspend/resume |

#### org.freedesktop.NetworkManager.Device

| Method/Property | crnetctl Status | Notes |
|-----------------|----------------|-------|
| `Udi` property | ❌ Not implemented | Device UDI |
| `Interface` property | ✅ Via device path | Interface name |
| `IpInterface` property | ⚠️ Partial | May differ from Interface |
| `Driver` property | ✅ Implemented | Driver name |
| `State` property | ✅ Implemented | Device state enum |
| `StateReason` property | ❌ Not implemented | Detailed state |
| `ActiveConnection` property | ⚠️ Partial | Connection path |
| `Ip4Config` property | ⚠️ Partial | IPv4 config path |
| `Ip6Config` property | ⚠️ Partial | IPv6 config path |
| `Dhcp4Config` property | ❌ Not implemented | DHCP info |
| `Dhcp6Config` property | ❌ Not implemented | DHCP info |
| `Managed` property | ✅ Implemented | Always true |
| `Autoconnect` property | ⚠️ Partial | Not persisted |
| `DeviceType` property | ✅ Implemented | Type enum |
| `Disconnect()` | ✅ Implemented | Disconnect device |
| `Delete()` | ⚠️ Partial | Virtual devices |
| `Reapply()` | ❌ Not implemented | Apply changes |

#### org.freedesktop.NetworkManager.Device.Wireless

| Method/Property | crnetctl Status | Notes |
|-----------------|----------------|-------|
| `GetAccessPoints()` | ✅ Via CR WiFi API | Get AP list |
| `GetAllAccessPoints()` | ✅ Via CR WiFi API | Same as above |
| `RequestScan(options)` | ✅ Implemented | Trigger scan |
| `AccessPoints` property | ✅ Implemented | AP object paths |
| `ActiveAccessPoint` property | ⚠️ Partial | Current AP |
| `HwAddress` property | ✅ Via device | MAC address |
| `Mode` property | ⚠️ Partial | WiFi mode |
| `Bitrate` property | ❌ Not implemented | Current rate |
| `WirelessCapabilities` property | ❌ Not implemented | WiFi caps |
| `LastScan` property | ❌ Not implemented | Scan timestamp |

#### org.freedesktop.NetworkManager.Settings

| Method | crnetctl Status | Notes |
|--------|----------------|-------|
| `ListConnections()` | ❌ Not implemented | Get all connections |
| `GetConnectionByUuid(uuid)` | ❌ Not implemented | Find by UUID |
| `AddConnection(settings)` | ❌ Not implemented | Add new connection |
| `AddConnectionUnsaved(settings)` | ❌ Not implemented | Add without saving |
| `SaveHostname(hostname)` | ❌ Not implemented | Set hostname |
| `ReloadConnections()` | ❌ Not implemented | Reload from disk |

#### org.freedesktop.NetworkManager.Connection.Active

| Property | crnetctl Status | Notes |
|----------|----------------|-------|
| `Connection` property | ⚠️ Partial | Connection path |
| `SpecificObject` property | ❌ Not implemented | AP/modem path |
| `Id` property | ✅ Implemented | Connection name |
| `Uuid` property | ✅ Implemented | Connection UUID |
| `Type` property | ✅ Implemented | Connection type |
| `Devices` property | ⚠️ Partial | Device paths |
| `State` property | ✅ Implemented | Connection state |
| `StateFlags` property | ❌ Not implemented | Detailed flags |
| `Default` property | ❌ Not implemented | Is default route |
| `Ip4Config` property | ⚠️ Partial | IPv4 config path |
| `Ip6Config` property | ⚠️ Partial | IPv6 config path |
| `Dhcp4Config` property | ❌ Not implemented | DHCP config path |
| `Dhcp6Config` property | ❌ Not implemented | DHCP config path |
| `Vpn` property | ⚠️ Partial | VPN flag |

### 2.3 Signals Required

| Signal | crnetctl Status | Used By |
|--------|----------------|---------|
| `StateChanged` | ✅ Implemented | All clients |
| `DeviceAdded` | ✅ Implemented | GUI applets |
| `DeviceRemoved` | ✅ Implemented | GUI applets |
| `PropertiesChanged` | ✅ Implemented | All clients |
| `CheckPermissions` | ❌ Not implemented | PolicyKit |

---

## 3. crnetctl Current Implementation Status

### 3.1 Native CR D-Bus API (org.crrouter.NetworkControl)

Fully implemented with these interfaces:
- `org.crrouter.NetworkControl` - Main network control
- `org.crrouter.NetworkControl.WiFi` - WiFi operations
- `org.crrouter.NetworkControl.VPN` - VPN management
- `org.crrouter.NetworkControl.Device` - Device control
- `org.crrouter.NetworkControl.Connection` - Connection management
- `org.crrouter.NetworkControl.DHCP` - DHCP configuration
- `org.crrouter.NetworkControl.DNS` - DNS configuration
- `org.crrouter.NetworkControl.Routing` - Routing tables
- `org.crrouter.NetworkControl.Privilege` - Privilege tokens

### 3.2 NetworkManager Compatibility Layer

Current implementation in `src/dbus/mod.rs`:
- Basic `org.freedesktop.NetworkManager` interface
- Device listing and state
- Connection activation/deactivation (basic)
- State and connectivity properties
- D-Bus signals (StateChanged, DeviceAdded, DeviceRemoved)

### 3.3 libnm-Compatible Library (libcr_compat)

Rust library providing libnm-equivalent API:
- `CRClient` - Equivalent to `NMClient`
- `CRDevice` - Equivalent to `NMDevice`
- `CRConnection` - Equivalent to `NMConnection`
- `CRActiveConnection` - Equivalent to `NMActiveConnection`
- `CRAccessPoint` - Equivalent to `NMAccessPoint`
- `CRIPConfig` - Equivalent to `NMIPConfig`

---

## 4. Implementation Plan for Full Compatibility

### 4.1 Phase 1: Core D-Bus API (Priority: High)

**Goal:** Support basic GUI applets (network-manager-applet, nm-tray)

| Task | Effort | Files |
|------|--------|-------|
| Implement Settings interface | 3 days | `src/nm_dbus/settings.rs` |
| Add ListConnections() method | 1 day | `src/nm_dbus/settings.rs` |
| Add GetConnectionByUuid() | 1 day | `src/nm_dbus/settings.rs` |
| Add AddConnection() method | 2 days | `src/nm_dbus/settings.rs` |
| Implement IP4Config object | 2 days | `src/nm_dbus/ip_config.rs` |
| Implement IP6Config object | 1 day | `src/nm_dbus/ip_config.rs` |
| Implement DHCP4Config object | 1 day | `src/nm_dbus/dhcp_config.rs` |
| Add ActiveConnection full props | 2 days | `src/nm_dbus/active_connection.rs` |
| Device StateReason property | 1 day | `src/nm_dbus/device.rs` |

### 4.2 Phase 2: WiFi Support (Priority: High)

**Goal:** Full WiFi GUI support

| Task | Effort | Files |
|------|--------|-------|
| AccessPoint D-Bus objects | 2 days | `src/nm_dbus/access_point.rs` |
| Device.Wireless interface | 2 days | `src/nm_dbus/device_wireless.rs` |
| WiFi security settings | 2 days | `src/nm_dbus/wifi_security.rs` |
| WPA-Enterprise support | 3 days | `src/nm_dbus/wifi_security.rs` |
| Hidden network support | 1 day | `src/nm_dbus/wifi.rs` |

### 4.3 Phase 3: VPN Plugin Support (Priority: Medium)

**Goal:** Support nm-openvpn, nm-wireguard plugins

| Task | Effort | Files |
|------|--------|-------|
| VPN service interface | 2 days | `src/nm_dbus/vpn_service.rs` |
| VPN connection interface | 2 days | `src/nm_dbus/vpn_connection.rs` |
| OpenVPN settings mapping | 2 days | `src/nm_dbus/vpn_openvpn.rs` |
| WireGuard settings mapping | 2 days | `src/nm_dbus/vpn_wireguard.rs` |
| IPsec settings mapping | 3 days | `src/nm_dbus/vpn_ipsec.rs` |

### 4.4 Phase 4: Advanced Features (Priority: Low)

**Goal:** Full desktop environment integration

| Task | Effort | Files |
|------|--------|-------|
| PolicyKit integration | 3 days | `src/nm_dbus/polkit.rs` |
| SecretAgent interface | 3 days | `src/nm_dbus/secret_agent.rs` |
| Checkpoint/rollback | 2 days | `src/nm_dbus/checkpoint.rs` |
| DNS manager interface | 2 days | `src/nm_dbus/dns_manager.rs` |
| Metered connection support | 1 day | `src/nm_dbus/metered.rs` |

---

## 5. Package Compatibility Matrix

### 5.1 Current Support Level

| Package | Status | Blocking Issues |
|---------|--------|-----------------|
| `wpasupplicant` | ✅ Works | None (uses wpa_cli) |
| `python3-networkmanager` | ⚠️ Partial | Missing Settings interface |
| `nm-connection-editor` | ❌ Not working | Missing Settings, libnm required |
| `network-manager-applet` | ❌ Not working | Missing Settings, secrets |
| `plasma-nm` | ❌ Not working | Missing multiple interfaces |
| `network-manager-openvpn` | ⚠️ Partial | VPN plugin interface needed |
| `chrony` | ✅ Works | Independent |
| `resolvconf` | ✅ Works | Can be configured for netctld |

### 5.2 After Phase 1 Completion

| Package | Expected Status |
|---------|-----------------|
| `python3-networkmanager` | ✅ Full support |
| `nm-tray` | ✅ Full support |
| Simple applets | ✅ Working |
| `network-manager-applet` | ⚠️ Basic support |
| `nm-connection-editor` | ⚠️ Basic support |

### 5.3 After Full Implementation

| Package | Expected Status |
|---------|-----------------|
| All GUI tools | ✅ Full support |
| All VPN plugins | ✅ Full support |
| GNOME integration | ✅ Full support |
| KDE integration | ✅ Full support |

---

## 6. Alternative Approach: D-Bus Adapter Daemon

Instead of implementing full NM compatibility in netctld, create a separate adapter:

```
┌─────────────────────┐
│  nm-applet/GUI      │
│  (uses libnm)       │
└──────────┬──────────┘
           │ D-Bus: org.freedesktop.NetworkManager
           ▼
┌─────────────────────┐
│  nm-compat-adapter  │  ← Translates NM D-Bus → CR D-Bus
│  (Rust daemon)      │
└──────────┬──────────┘
           │ D-Bus: org.crrouter.NetworkControl
           ▼
┌─────────────────────┐
│     netctld         │
│  (native service)   │
└─────────────────────┘
```

**Advantages:**
- Clean separation of concerns
- Native CR API remains simple
- Can be disabled if not needed
- Easier testing and debugging

**Disadvantages:**
- Additional daemon to run
- Slight latency overhead
- More moving parts

---

## 7. Recommended Implementation Order

1. **Immediate (This Week):**
   - Complete Settings interface with ListConnections()
   - Add GetConnectionByUuid()
   - Fix D-Bus name ownership issue

2. **Short Term (2-4 Weeks):**
   - Full Device properties
   - IP4Config and IP6Config objects
   - ActiveConnection full implementation
   - python3-networkmanager compatibility

3. **Medium Term (1-2 Months):**
   - GUI applet support
   - VPN plugin compatibility
   - WiFi advanced features

4. **Long Term (3+ Months):**
   - Full desktop environment integration
   - PolicyKit support
   - SecretAgent interface

---

## 8. Testing Strategy

### 8.1 D-Bus Interface Testing

```bash
# Test basic NM compatibility
busctl introspect org.freedesktop.NetworkManager /org/freedesktop/NetworkManager

# Test device listing
dbus-send --system --print-reply \
  --dest=org.freedesktop.NetworkManager \
  /org/freedesktop/NetworkManager \
  org.freedesktop.NetworkManager.GetDevices

# Test state query
dbus-send --system --print-reply \
  --dest=org.freedesktop.NetworkManager \
  /org/freedesktop/NetworkManager \
  org.freedesktop.DBus.Properties.Get \
  string:'org.freedesktop.NetworkManager' string:'State'
```

### 8.2 Python Client Testing

```python
#!/usr/bin/env python3
import dbus

bus = dbus.SystemBus()
nm = bus.get_object('org.freedesktop.NetworkManager',
                    '/org/freedesktop/NetworkManager')
nm_iface = dbus.Interface(nm, 'org.freedesktop.NetworkManager')

# Test basic operations
print(f"Version: {nm_iface.Version()}")
print(f"State: {nm_iface.state()}")
print(f"Devices: {nm_iface.GetDevices()}")
```

### 8.3 GUI Application Testing

1. Install `nm-tray` (lightweight, good for testing)
2. Install `network-manager-applet` (full featured)
3. Verify connection listing, WiFi scanning, connection activation

---

## 9. Known Limitations

1. **libnm Dependency:** Some applications link directly against libnm.so instead of using D-Bus. These cannot be supported without a source code change or LD_PRELOAD hack.

2. **PolicyKit:** Full desktop integration requires PolicyKit authentication. This is complex to implement correctly.

3. **Secrets:** NetworkManager has sophisticated secret handling (gnome-keyring integration). May need simplified approach initially.

4. **Connection Files:** NM uses `/etc/NetworkManager/system-connections/`. Applications may expect files there.

---

## 10. Conclusion

crnetctl has a solid foundation for NetworkManager compatibility with:
- Native D-Bus service (org.crrouter.NetworkControl)
- Basic NM compatibility layer (org.freedesktop.NetworkManager)
- libnm-compatible Rust library (libcr_compat)

To achieve full compatibility with dependent packages:
1. Implement the Settings interface for connection management
2. Add full Device and WiFi object support
3. Implement VPN plugin interfaces
4. Add PolicyKit for desktop integration

Estimated effort for basic GUI support: 2-3 weeks
Estimated effort for full compatibility: 2-3 months
