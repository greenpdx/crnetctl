//! Network interface control
//!
//! Low-level interface management using ip command and sysfs

use crate::error::{NetctlError, NetctlResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command;
use tokio::io::{AsyncReadExt};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceInfo {
    pub name: String,
    pub index: Option<u32>,
    pub mac_address: Option<String>,
    pub mtu: Option<u32>,
    pub state: Option<String>,
    pub flags: Vec<String>,
    pub addresses: Vec<IpAddress>,
    pub stats: Option<InterfaceStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpAddress {
    pub address: String,
    pub family: String,      // "inet" or "inet6"
    pub prefix_len: u8,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceStats {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_dropped: u64,
    pub tx_dropped: u64,
}

/// Interface controller
pub struct InterfaceController {
    // Future: could hold state or config
}

impl InterfaceController {
    pub fn new() -> Self {
        Self {}
    }

    /// List all network interfaces
    pub async fn list(&self) -> NetctlResult<Vec<String>> {
        let net_path = Path::new("/sys/class/net");

        if !net_path.exists() {
            return Err(NetctlError::NotSupported(
                "/sys/class/net not available".to_string()
            ));
        }

        let mut entries = fs::read_dir(net_path).await?;
        let mut interfaces = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                interfaces.push(name.to_string());
            }
        }

        interfaces.sort();
        Ok(interfaces)
    }

    /// Get detailed interface information
    pub async fn get_info(&self, interface: &str) -> NetctlResult<InterfaceInfo> {
        // Verify interface exists
        let sys_path = format!("/sys/class/net/{}", interface);
        if !Path::new(&sys_path).exists() {
            return Err(NetctlError::InterfaceNotFound(interface.to_string()));
        }

        let mut info = InterfaceInfo {
            name: interface.to_string(),
            index: None,
            mac_address: None,
            mtu: None,
            state: None,
            flags: Vec::new(),
            addresses: Vec::new(),
            stats: None,
        };

        // Read from sysfs
        info.index = self.read_sysfs_u32(interface, "ifindex").await;
        info.mac_address = self.read_sysfs_string(interface, "address").await;
        info.mtu = self.read_sysfs_u32(interface, "mtu").await;
        info.state = self.read_sysfs_string(interface, "operstate").await;
        info.stats = self.read_stats(interface).await;

        // Get IP addresses
        info.addresses = self.get_addresses(interface).await?;

        // Get flags from ip command
        info.flags = self.get_flags(interface).await?;

        Ok(info)
    }

    /// Bring interface up
    pub async fn up(&self, interface: &str) -> NetctlResult<()> {
        self.run_ip(&["link", "set", "dev", interface, "up"]).await
    }

    /// Bring interface down
    pub async fn down(&self, interface: &str) -> NetctlResult<()> {
        self.run_ip(&["link", "set", "dev", interface, "down"]).await
    }

    /// Set IP address
    pub async fn set_ip(&self, interface: &str, address: &str, prefix_len: u8) -> NetctlResult<()> {
        let addr = format!("{}/{}", address, prefix_len);
        self.run_ip(&["addr", "add", &addr, "dev", interface]).await
    }

    /// Add secondary IP address
    pub async fn add_ip(&self, interface: &str, address: &str, prefix_len: u8) -> NetctlResult<()> {
        self.set_ip(interface, address, prefix_len).await
    }

    /// Delete IP address
    pub async fn del_ip(&self, interface: &str, address: &str, prefix_len: u8) -> NetctlResult<()> {
        let addr = format!("{}/{}", address, prefix_len);
        self.run_ip(&["addr", "del", &addr, "dev", interface]).await
    }

    /// Flush all IP addresses
    pub async fn flush_addrs(&self, interface: &str) -> NetctlResult<()> {
        self.run_ip(&["addr", "flush", "dev", interface]).await
    }

    /// Set MAC address
    pub async fn set_mac(&self, interface: &str, mac: &str) -> NetctlResult<()> {
        // Must bring interface down first
        self.down(interface).await?;
        self.run_ip(&["link", "set", "dev", interface, "address", mac]).await?;
        self.up(interface).await?;
        Ok(())
    }

    /// Set MTU
    pub async fn set_mtu(&self, interface: &str, mtu: u32) -> NetctlResult<()> {
        let mtu_str = mtu.to_string();
        self.run_ip(&["link", "set", "dev", interface, "mtu", &mtu_str]).await
    }

    /// Set transmit queue length
    pub async fn set_txqueuelen(&self, interface: &str, len: u32) -> NetctlResult<()> {
        let len_str = len.to_string();
        self.run_ip(&["link", "set", "dev", interface, "txqueuelen", &len_str]).await
    }

    /// Set promiscuous mode
    pub async fn set_promisc(&self, interface: &str, enable: bool) -> NetctlResult<()> {
        let mode = if enable { "on" } else { "off" };
        self.run_ip(&["link", "set", "dev", interface, "promisc", mode]).await
    }

    /// Set multicast
    pub async fn set_multicast(&self, interface: &str, enable: bool) -> NetctlResult<()> {
        let mode = if enable { "on" } else { "off" };
        self.run_ip(&["link", "set", "dev", interface, "multicast", mode]).await
    }

    /// Set all multicast
    pub async fn set_allmulticast(&self, interface: &str, enable: bool) -> NetctlResult<()> {
        let mode = if enable { "on" } else { "off" };
        self.run_ip(&["link", "set", "dev", interface, "allmulticast", mode]).await
    }

    /// Rename interface
    pub async fn rename(&self, old_name: &str, new_name: &str) -> NetctlResult<()> {
        // Must be down to rename
        self.down(old_name).await?;
        self.run_ip(&["link", "set", "dev", old_name, "name", new_name]).await?;
        self.up(new_name).await?;
        Ok(())
    }

    // === Helper functions ===

    async fn run_ip(&self, args: &[&str]) -> NetctlResult<()> {
        let output = Command::new("ip")
            .args(args)
            .output()
            .await
            .map_err(|e| NetctlError::CommandFailed {
                cmd: format!("ip {}", args.join(" ")),
                code: None,
                stderr: e.to_string(),
            })?;

        if !output.status.success() {
            return Err(NetctlError::CommandFailed {
                cmd: format!("ip {}", args.join(" ")),
                code: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(())
    }

    async fn read_sysfs_string(&self, interface: &str, file: &str) -> Option<String> {
        let path = format!("/sys/class/net/{}/{}", interface, file);
        fs::read_to_string(path).await.ok().map(|s| s.trim().to_string())
    }

    async fn read_sysfs_u32(&self, interface: &str, file: &str) -> Option<u32> {
        self.read_sysfs_string(interface, file).await?.parse().ok()
    }

    async fn read_sysfs_u64(&self, interface: &str, file: &str) -> Option<u64> {
        self.read_sysfs_string(interface, file).await?.parse().ok()
    }

    async fn read_stats(&self, interface: &str) -> Option<InterfaceStats> {
        Some(InterfaceStats {
            rx_bytes: self.read_sysfs_u64(interface, "statistics/rx_bytes").await?,
            tx_bytes: self.read_sysfs_u64(interface, "statistics/tx_bytes").await?,
            rx_packets: self.read_sysfs_u64(interface, "statistics/rx_packets").await?,
            tx_packets: self.read_sysfs_u64(interface, "statistics/tx_packets").await?,
            rx_errors: self.read_sysfs_u64(interface, "statistics/rx_errors").await.unwrap_or(0),
            tx_errors: self.read_sysfs_u64(interface, "statistics/tx_errors").await.unwrap_or(0),
            rx_dropped: self.read_sysfs_u64(interface, "statistics/rx_dropped").await.unwrap_or(0),
            tx_dropped: self.read_sysfs_u64(interface, "statistics/tx_dropped").await.unwrap_or(0),
        })
    }

    async fn get_addresses(&self, interface: &str) -> NetctlResult<Vec<IpAddress>> {
        let output = Command::new("ip")
            .args(["-json", "addr", "show", interface])
            .output()
            .await
            .map_err(|e| NetctlError::CommandFailed {
                cmd: format!("ip -json addr show {}", interface),
                code: None,
                stderr: e.to_string(),
            })?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| NetctlError::ParseError(e.to_string()))?;

        let mut addresses = Vec::new();

        if let Some(arr) = json.as_array() {
            if let Some(iface) = arr.first() {
                if let Some(addr_info) = iface.get("addr_info").and_then(|v| v.as_array()) {
                    for addr in addr_info {
                        if let (Some(local), Some(family), Some(prefixlen)) = (
                            addr.get("local").and_then(|v| v.as_str()),
                            addr.get("family").and_then(|v| v.as_str()),
                            addr.get("prefixlen").and_then(|v| v.as_u64()),
                        ) {
                            addresses.push(IpAddress {
                                address: local.to_string(),
                                family: family.to_string(),
                                prefix_len: prefixlen as u8,
                                scope: addr.get("scope").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            });
                        }
                    }
                }
            }
        }

        Ok(addresses)
    }

    async fn get_flags(&self, interface: &str) -> NetctlResult<Vec<String>> {
        let output = Command::new("ip")
            .args(["-json", "link", "show", interface])
            .output()
            .await
            .map_err(|e| NetctlError::CommandFailed {
                cmd: format!("ip -json link show {}", interface),
                code: None,
                stderr: e.to_string(),
            })?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| NetctlError::ParseError(e.to_string()))?;

        let mut flags = Vec::new();

        if let Some(arr) = json.as_array() {
            if let Some(iface) = arr.first() {
                if let Some(flag_arr) = iface.get("flags").and_then(|v| v.as_array()) {
                    for flag in flag_arr {
                        if let Some(f) = flag.as_str() {
                            flags.push(f.to_string());
                        }
                    }
                }
            }
        }

        Ok(flags)
    }
}

impl Default for InterfaceController {
    fn default() -> Self {
        Self::new()
    }
}
