//! VPN Module for LnxNetCtl
//!
//! This module provides a unified interface for managing VPN connections across
//! different VPN technologies including WireGuard, OpenVPN, IPsec/FreeSWAN, and Tor (via Arti).
//!
//! # Architecture
//!
//! The VPN module uses a driver-based architecture:
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │       VPN Manager (Unified API)     │
//! └──────────────┬──────────────────────┘
//!                │
//!    ┌───────────┼───────────┬──────────┐
//!    │           │           │          │
//!    ▼           ▼           ▼          ▼
//! ┌──────┐   ┌──────┐   ┌──────┐   ┌──────┐
//! │  WG  │   │ OVPN │   │IPsec │   │ Arti │  <- Backend Drivers
//! └──────┘   └──────┘   └──────┘   └──────┘
//! ```
//!
//! Each backend driver implements the `VpnBackend` trait, providing a common
//! interface for connection management, statistics, and configuration.
//!
//! # Usage
//!
//! ```rust,no_run
//! use lnxnetctl::vpn::{VpnManager, wireguard, openvpn, ipsec, arti};
//!
//! let mut manager = VpnManager::new("/etc/netctl".into());
//! manager.register_backend("wireguard", wireguard::create_backend);
//! manager.register_backend("openvpn", openvpn::create_backend);
//! manager.register_backend("ipsec", ipsec::create_backend);
//! manager.register_backend("arti", arti::create_backend);
//!
//! // Create and connect to a VPN
//! let uuid = manager.create_connection(config).await?;
//! manager.connect(&uuid).await?;
//! ```

pub mod backend;
pub mod common;
pub mod manager;
pub mod wireguard;
pub mod openvpn;
pub mod ipsec;
pub mod arti;

pub use backend::{VpnBackend, VpnBackendFactory, VpnState, VpnStats};
pub use manager::VpnManager;
