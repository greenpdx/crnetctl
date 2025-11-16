//! libnm-compatible API layer for libnetctl
//!
//! This module provides a NetworkManager libnm-compatible API using CR prefix.
//! It wraps the existing libnetctl functionality to match libnm's API structure.
//!
//! Main components:
//! - CRClient: Main entry point (equivalent to NMClient)
//! - CRDevice: Network device representation (equivalent to NMDevice)
//! - CRConnection: Connection configuration (equivalent to NMConnection)
//! - CRActiveConnection: Active connection state (equivalent to NMActiveConnection)
//! - CRAccessPoint: WiFi access point (equivalent to NMAccessPoint)
//! - CRIPConfig: IP configuration (equivalent to NMIPConfig)

mod client;
mod device;
mod connection;
mod active_connection;
mod access_point;
mod ip_config;
mod settings;
mod enums;

// Re-export public API
pub use client::CRClient;
pub use device::{CRDevice, CRDeviceType, CRDeviceState, CRDeviceCapabilities};
pub use connection::{CRConnection, CRRemoteConnection};
pub use active_connection::CRActiveConnection;
pub use access_point::CRAccessPoint;
pub use ip_config::{CRIPConfig, CRIPAddress, CRIPRoute};
pub use settings::{
    CRSetting,
    CRSettingConnection,
    CRSettingWired,
    CRSettingWireless,
    CRSettingIP4Config,
    CRSettingIP6Config,
};
pub use enums::{CRState, CRConnectivityState, CRActiveConnectionState};
