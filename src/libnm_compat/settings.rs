//! Settings types (libnm NMSetting* equivalents)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Base setting type (equivalent to NMSetting)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRSetting {
    pub name: String,
    pub properties: HashMap<String, String>,
}

/// Connection setting (equivalent to NMSettingConnection)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRSettingConnection {
    /// Connection ID
    pub id: String,
    /// Connection UUID
    pub uuid: String,
    /// Connection type (e.g., "802-3-ethernet", "802-11-wireless")
    pub connection_type: String,
    /// Interface name
    pub interface_name: Option<String>,
    /// Whether to auto-connect
    pub autoconnect: bool,
    /// Timestamp of last connection
    pub timestamp: u64,
}

/// Wired Ethernet setting (equivalent to NMSettingWired)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRSettingWired {
    /// Port type
    pub port: Option<String>,
    /// Speed in Mb/s
    pub speed: Option<u32>,
    /// Duplex mode
    pub duplex: Option<String>,
    /// Auto-negotiate
    pub auto_negotiate: bool,
    /// MAC address
    pub mac_address: Option<String>,
    /// Cloned MAC address
    pub cloned_mac_address: Option<String>,
    /// MAC address blacklist
    pub mac_address_blacklist: Vec<String>,
    /// MTU
    pub mtu: Option<u32>,
    /// S390 subchannels
    pub s390_subchannels: Vec<String>,
    /// S390 network type
    pub s390_nettype: Option<String>,
    /// S390 options
    pub s390_options: HashMap<String, String>,
    /// Wake-on-LAN
    pub wake_on_lan: u32,
    /// Wake-on-LAN password
    pub wake_on_lan_password: Option<String>,
}

impl Default for CRSettingWired {
    fn default() -> Self {
        Self {
            port: None,
            speed: None,
            duplex: None,
            auto_negotiate: true,
            mac_address: None,
            cloned_mac_address: None,
            mac_address_blacklist: Vec::new(),
            mtu: None,
            s390_subchannels: Vec::new(),
            s390_nettype: None,
            s390_options: HashMap::new(),
            wake_on_lan: 1, // Default enabled
            wake_on_lan_password: None,
        }
    }
}

/// Wireless setting (equivalent to NMSettingWireless)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRSettingWireless {
    /// SSID
    pub ssid: Vec<u8>,
    /// Operating mode (infrastructure, adhoc, ap, mesh)
    pub mode: String,
    /// WiFi band (a, bg)
    pub band: Option<String>,
    /// WiFi channel
    pub channel: Option<u32>,
    /// BSSID (specific access point)
    pub bssid: Option<String>,
    /// TX rate limit
    pub rate: Option<u32>,
    /// TX power
    pub tx_power: Option<u32>,
    /// MAC address
    pub mac_address: Option<String>,
    /// Cloned MAC address
    pub cloned_mac_address: Option<String>,
    /// MAC address blacklist
    pub mac_address_blacklist: Vec<String>,
    /// MAC address randomization
    pub mac_address_randomization: u32,
    /// MTU
    pub mtu: Option<u32>,
    /// Hidden network
    pub hidden: bool,
    /// Power save mode
    pub powersave: u32,
}

impl Default for CRSettingWireless {
    fn default() -> Self {
        Self {
            ssid: Vec::new(),
            mode: "infrastructure".to_string(),
            band: None,
            channel: None,
            bssid: None,
            rate: None,
            tx_power: None,
            mac_address: None,
            cloned_mac_address: None,
            mac_address_blacklist: Vec::new(),
            mac_address_randomization: 0,
            mtu: None,
            hidden: false,
            powersave: 0,
        }
    }
}

/// Wireless security setting (equivalent to NMSettingWirelessSecurity)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRSettingWirelessSecurity {
    /// Key management (none, ieee8021x, wpa-none, wpa-psk, wpa-eap, sae, owe, wpa-psk-sha256, wpa-eap-suite-b-192)
    pub key_mgmt: String,
    /// WEP TX key index
    pub wep_tx_keyidx: u32,
    /// Auth algorithm (open, shared, leap)
    pub auth_alg: Option<String>,
    /// Protocols (wpa, rsn)
    pub proto: Vec<String>,
    /// Pairwise ciphers (tkip, ccmp)
    pub pairwise: Vec<String>,
    /// Group ciphers (wep40, wep104, tkip, ccmp)
    pub group: Vec<String>,
    /// Proactive Key Caching
    pub pmf: u32,
    /// LEAP username
    pub leap_username: Option<String>,
    /// WEP key (for open/shared auth)
    pub wep_key0: Option<String>,
    /// WEP key type (1=hex/ASCII, 2=passphrase)
    pub wep_key_type: u32,
    /// WPA PSK (password)
    pub psk: Option<String>,
    /// WPA PSK flags
    pub psk_flags: u32,
}

impl Default for CRSettingWirelessSecurity {
    fn default() -> Self {
        Self {
            key_mgmt: "wpa-psk".to_string(),
            wep_tx_keyidx: 0,
            auth_alg: None,
            proto: vec!["rsn".to_string()],
            pairwise: vec!["ccmp".to_string()],
            group: vec!["ccmp".to_string()],
            pmf: 0,
            leap_username: None,
            wep_key0: None,
            wep_key_type: 1,
            psk: None,
            psk_flags: 0,
        }
    }
}

/// IPv4 configuration setting (equivalent to NMSettingIP4Config)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRSettingIP4Config {
    /// Method (auto, link-local, manual, shared, disabled)
    pub method: String,
    /// DNS servers
    pub dns: Vec<String>,
    /// DNS search domains
    pub dns_search: Vec<String>,
    /// DNS priority
    pub dns_priority: i32,
    /// Static addresses
    pub addresses: Vec<CRIPAddress>,
    /// Gateway
    pub gateway: Option<String>,
    /// Static routes
    pub routes: Vec<CRIPRoute>,
    /// Route metric
    pub route_metric: i64,
    /// Ignore auto routes
    pub ignore_auto_routes: bool,
    /// Ignore auto DNS
    pub ignore_auto_dns: bool,
    /// DHCP client ID
    pub dhcp_client_id: Option<String>,
    /// DHCP timeout
    pub dhcp_timeout: u32,
    /// DHCP send hostname
    pub dhcp_send_hostname: bool,
    /// DHCP hostname
    pub dhcp_hostname: Option<String>,
    /// Never default route
    pub never_default: bool,
    /// May fail (allow continuing even if IP config fails)
    pub may_fail: bool,
}

impl Default for CRSettingIP4Config {
    fn default() -> Self {
        Self {
            method: "auto".to_string(),
            dns: Vec::new(),
            dns_search: Vec::new(),
            dns_priority: 0,
            addresses: Vec::new(),
            gateway: None,
            routes: Vec::new(),
            route_metric: -1,
            ignore_auto_routes: false,
            ignore_auto_dns: false,
            dhcp_client_id: None,
            dhcp_timeout: 0,
            dhcp_send_hostname: true,
            dhcp_hostname: None,
            never_default: false,
            may_fail: true,
        }
    }
}

/// IPv6 configuration setting (equivalent to NMSettingIP6Config)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRSettingIP6Config {
    /// Method (auto, dhcp, link-local, manual, shared, ignore, disabled)
    pub method: String,
    /// DNS servers
    pub dns: Vec<String>,
    /// DNS search domains
    pub dns_search: Vec<String>,
    /// DNS priority
    pub dns_priority: i32,
    /// Static addresses
    pub addresses: Vec<CRIPAddress>,
    /// Gateway
    pub gateway: Option<String>,
    /// Static routes
    pub routes: Vec<CRIPRoute>,
    /// Route metric
    pub route_metric: i64,
    /// Ignore auto routes
    pub ignore_auto_routes: bool,
    /// Ignore auto DNS
    pub ignore_auto_dns: bool,
    /// Never default route
    pub never_default: bool,
    /// May fail
    pub may_fail: bool,
    /// IPv6 privacy extensions
    pub ip6_privacy: i32,
    /// Address generation mode
    pub addr_gen_mode: i32,
    /// DHCP send hostname
    pub dhcp_send_hostname: bool,
    /// DHCP hostname
    pub dhcp_hostname: Option<String>,
    /// Token (for stable privacy addressing)
    pub token: Option<String>,
}

impl Default for CRSettingIP6Config {
    fn default() -> Self {
        Self {
            method: "auto".to_string(),
            dns: Vec::new(),
            dns_search: Vec::new(),
            dns_priority: 0,
            addresses: Vec::new(),
            gateway: None,
            routes: Vec::new(),
            route_metric: -1,
            ignore_auto_routes: false,
            ignore_auto_dns: false,
            never_default: false,
            may_fail: true,
            ip6_privacy: 0,
            addr_gen_mode: 1,
            dhcp_send_hostname: true,
            dhcp_hostname: None,
            token: None,
        }
    }
}

/// IP address representation for settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRIPAddress {
    /// IP address
    pub address: String,
    /// Prefix length
    pub prefix: u32,
}

/// IP route representation for settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRIPRoute {
    /// Destination network
    pub dest: String,
    /// Prefix length
    pub prefix: u32,
    /// Next hop
    pub next_hop: Option<String>,
    /// Metric
    pub metric: i64,
}
