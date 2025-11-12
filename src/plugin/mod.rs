//! Plugin system for netctl
//!
//! Provides extensible plugin architecture for network functionality:
//! - VPN protocols (OpenVPN, WireGuard, IPSec, PPTP, etc.)
//! - TUN/TAP devices
//! - Virtual interfaces (bridges, bonds, VLANs)
//! - Mobile broadband (3G/4G/5G)
//! - Bluetooth networking
//! - Custom network protocols

pub mod traits;
pub mod manager;
pub mod loader;
pub mod config;

// Built-in plugins
pub mod openvpn;
pub mod wireguard;
pub mod tuntap;
pub mod vlan;
pub mod bridge;

pub use traits::{NetworkPlugin, PluginCapability, PluginMetadata, PluginState, ConnectionConfig, ConnectionStats};
pub use manager::PluginManager;
pub use config::{PluginConfig, PluginConfigManager};
pub use loader::PluginLoader;
