//! CRAccessPoint - WiFi access point (libnm NMAccessPoint equivalent)

use crate::wifi::ScanResult;
use serde::{Deserialize, Serialize};

/// WiFi access point (equivalent to NMAccessPoint)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRAccessPoint {
    /// SSID (network name)
    ssid: Vec<u8>,
    /// BSSID (MAC address of AP)
    bssid: String,
    /// Frequency in MHz
    frequency: u32,
    /// Channel
    channel: u32,
    /// Signal strength (0-100)
    strength: u8,
    /// Flags
    flags: u32,
    /// WPA flags
    wpa_flags: u32,
    /// RSN (WPA2) flags
    rsn_flags: u32,
    /// Operating mode
    mode: CRAccessPointMode,
    /// Maximum bitrate in Kb/s
    max_bitrate: u32,
    /// Last seen timestamp
    last_seen: i64,
}

/// Access point operating mode (equivalent to NM80211Mode)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CRAccessPointMode {
    /// Mode is unknown
    Unknown = 0,
    /// For both devices and access point objects, indicates the object is part of an Ad-Hoc 802.11 network
    Adhoc = 1,
    /// The device or access point is in infrastructure mode
    Infra = 2,
    /// The device is an access point/hotspot
    Ap = 3,
    /// The device is a 802.11s mesh point
    Mesh = 4,
}

impl CRAccessPoint {
    /// Creates an access point from a scan result
    pub(crate) fn from_scan_result(result: ScanResult) -> Self {
        // Parse signal strength from signal string (e.g., "-50.00 dBm")
        let strength = result.signal
            .and_then(|s| s.split_whitespace().next().map(|v| v.to_string()))
            .and_then(|s| s.parse::<f32>().ok())
            .map(|dbm| {
                // Convert dBm to 0-100 scale
                // Typical WiFi range is -90 dBm (weak) to -30 dBm (strong)
                let percent = ((dbm + 90.0) / 60.0 * 100.0).max(0.0).min(100.0);
                percent as u8
            })
            .unwrap_or(0);

        // Parse frequency to calculate channel
        let frequency = result.frequency.unwrap_or(0);
        let channel = if frequency >= 2412 && frequency <= 2484 {
            // 2.4 GHz: channel = (freq - 2407) / 5
            (frequency - 2407) / 5
        } else if frequency >= 5170 && frequency <= 5825 {
            // 5 GHz: channel = (freq - 5000) / 5
            (frequency - 5000) / 5
        } else {
            0
        };

        // Check capabilities for security info
        let has_wpa = result.capabilities.iter().any(|c| c.contains("WPA"));
        let has_wpa2 = result.capabilities.iter().any(|c| c.contains("WPA2") || c.contains("RSN"));
        let has_wpa3 = result.capabilities.iter().any(|c| c.contains("WPA3"));
        let has_wep = result.capabilities.iter().any(|c| c.contains("WEP") || c.contains("Privacy"));

        Self {
            ssid: result.ssid.unwrap_or_default().into_bytes(),
            bssid: result.bssid,
            frequency,
            channel,
            strength,
            flags: if has_wpa || has_wep { 1 } else { 0 },
            wpa_flags: if has_wpa && !has_wpa2 { 0x344 } else { 0 },
            rsn_flags: if has_wpa2 || has_wpa3 { 0x344 } else { 0 },
            mode: CRAccessPointMode::Infra,
            max_bitrate: 54000, // Default to 54 Mbps
            last_seen: chrono::Utc::now().timestamp(),
        }
    }

    /// Gets the SSID (equivalent to nm_access_point_get_ssid)
    pub fn get_ssid(&self) -> &[u8] {
        &self.ssid
    }

    /// Gets the SSID as a string
    pub fn get_ssid_string(&self) -> String {
        String::from_utf8_lossy(&self.ssid).to_string()
    }

    /// Gets the BSSID (equivalent to nm_access_point_get_bssid)
    pub fn get_bssid(&self) -> &str {
        &self.bssid
    }

    /// Gets the frequency in MHz (equivalent to nm_access_point_get_frequency)
    pub fn get_frequency(&self) -> u32 {
        self.frequency
    }

    /// Gets the channel (equivalent to nm_access_point_get_channel)
    pub fn get_channel(&self) -> u32 {
        self.channel
    }

    /// Gets the signal strength (0-100) (equivalent to nm_access_point_get_strength)
    pub fn get_strength(&self) -> u8 {
        self.strength
    }

    /// Gets the flags (equivalent to nm_access_point_get_flags)
    pub fn get_flags(&self) -> u32 {
        self.flags
    }

    /// Gets the WPA flags (equivalent to nm_access_point_get_wpa_flags)
    pub fn get_wpa_flags(&self) -> u32 {
        self.wpa_flags
    }

    /// Gets the RSN flags (equivalent to nm_access_point_get_rsn_flags)
    pub fn get_rsn_flags(&self) -> u32 {
        self.rsn_flags
    }

    /// Gets the operating mode (equivalent to nm_access_point_get_mode)
    pub fn get_mode(&self) -> CRAccessPointMode {
        self.mode
    }

    /// Gets the maximum bitrate in Kb/s (equivalent to nm_access_point_get_max_bitrate)
    pub fn get_max_bitrate(&self) -> u32 {
        self.max_bitrate
    }

    /// Gets the last seen timestamp (equivalent to nm_access_point_get_last_seen)
    pub fn get_last_seen(&self) -> i64 {
        self.last_seen
    }

    /// Gets the path (D-Bus path) (equivalent to nm_access_point_get_path)
    pub fn get_path(&self) -> String {
        format!("/org/freedesktop/NetworkManager/AccessPoint/{}",
                self.bssid.replace(":", ""))
    }

    /// Checks if the access point has security
    pub fn is_secured(&self) -> bool {
        self.wpa_flags != 0 || self.rsn_flags != 0 || self.flags != 0
    }

    /// Gets security type as a string
    pub fn get_security_type(&self) -> String {
        if self.rsn_flags != 0 {
            if self.rsn_flags & 0x100 != 0 {
                "WPA3".to_string()
            } else {
                "WPA2".to_string()
            }
        } else if self.wpa_flags != 0 {
            "WPA".to_string()
        } else if self.flags != 0 {
            "WEP".to_string()
        } else {
            "Open".to_string()
        }
    }
}
