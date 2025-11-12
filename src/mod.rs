//! Network Control Library (netctl)
//!
//! Shared library for network device control, usable both as a crrouterd plugin
//! and as a standalone CLI tool.
//!
//! Modules:
//! - interface: Low-level interface control (up/down, IP, MAC, MTU)
//! - wifi: WiFi device control and capabilities
//! - hostapd: WiFi Access Point management via hostapd
//! - dhcp: DHCP server management via dora
//! - dns: DNS server management via unbound
//! - routing: Routing table and policy management
//! - monitor: Network monitoring and statistics
//! - debug: Debugging and diagnostic tools
//! - config: Configuration file management
//! - error: Error types

pub mod error;
pub mod config;
pub mod interface;
pub mod wifi;
pub mod hostapd;
pub mod dhcp;
pub mod dns;
pub mod routing;
pub mod monitor;
pub mod debug;

// Re-export commonly used types
pub use error::{NetctlError, NetctlResult};
pub use config::NetctlConfig;
