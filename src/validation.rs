//! Input validation and sanitization
//!
//! Security module to prevent command injection and other input-based attacks

use crate::error::{NetctlError, NetctlResult};
use std::net::IpAddr;
use std::path::{Path, PathBuf};

/// Maximum length for interface names (Linux kernel limit is 15)
const MAX_INTERFACE_NAME_LEN: usize = 15;

/// Maximum length for configuration values
const MAX_CONFIG_VALUE_LEN: usize = 255;

/// Maximum length for error messages shown to users
const MAX_ERROR_MESSAGE_LEN: usize = 500;

/// Validate interface name to prevent command injection
///
/// Interface names must be alphanumeric with optional dashes and underscores,
/// and no longer than 15 characters (Linux kernel limit)
pub fn validate_interface_name(name: &str) -> NetctlResult<()> {
    if name.is_empty() {
        return Err(NetctlError::InvalidParameter(
            "Interface name cannot be empty".to_string()
        ));
    }

    if name.len() > MAX_INTERFACE_NAME_LEN {
        return Err(NetctlError::InvalidParameter(
            format!("Interface name too long (max {} characters)", MAX_INTERFACE_NAME_LEN)
        ));
    }

    // Only allow alphanumeric, dash, underscore
    // This prevents shell metacharacters and command injection
    for c in name.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' && c != '_' {
            return Err(NetctlError::InvalidParameter(
                format!("Invalid interface name '{}': contains invalid character '{}'", name, c)
            ));
        }
    }

    // Don't allow names starting with dash (could be interpreted as option)
    if name.starts_with('-') {
        return Err(NetctlError::InvalidParameter(
            "Interface name cannot start with dash".to_string()
        ));
    }

    Ok(())
}

/// Validate IP address
///
/// Uses Rust's built-in IP address parser to ensure valid format
pub fn validate_ip_address(addr: &str) -> NetctlResult<IpAddr> {
    addr.parse::<IpAddr>()
        .map_err(|_| NetctlError::InvalidParameter(
            format!("Invalid IP address: {}", addr)
        ))
}

/// Validate MAC address format
///
/// Accepts standard MAC format: XX:XX:XX:XX:XX:XX (hex digits)
pub fn validate_mac_address(mac: &str) -> NetctlResult<()> {
    if mac.len() != 17 {
        return Err(NetctlError::InvalidParameter(
            "MAC address must be in format XX:XX:XX:XX:XX:XX".to_string()
        ));
    }

    let parts: Vec<&str> = mac.split(':').collect();
    if parts.len() != 6 {
        return Err(NetctlError::InvalidParameter(
            "MAC address must have 6 octets separated by colons".to_string()
        ));
    }

    for part in parts {
        if part.len() != 2 {
            return Err(NetctlError::InvalidParameter(
                "Each MAC address octet must be 2 hex digits".to_string()
            ));
        }

        if !part.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(NetctlError::InvalidParameter(
                format!("Invalid hex digit in MAC address: {}", part)
            ));
        }
    }

    Ok(())
}

/// Validate prefix length for IPv4 or IPv6
pub fn validate_prefix_len(prefix: u8, is_ipv6: bool) -> NetctlResult<()> {
    let max = if is_ipv6 { 128 } else { 32 };
    if prefix > max {
        return Err(NetctlError::InvalidParameter(
            format!("Prefix length {} exceeds maximum {}", prefix, max)
        ));
    }
    Ok(())
}

/// Validate MTU value
pub fn validate_mtu(mtu: u32) -> NetctlResult<()> {
    // Ethernet minimum is 68, maximum is typically 9000 (jumbo frames)
    // Most common maximum is 1500
    if mtu < 68 {
        return Err(NetctlError::InvalidParameter(
            "MTU must be at least 68 bytes".to_string()
        ));
    }
    if mtu > 9000 {
        return Err(NetctlError::InvalidParameter(
            "MTU cannot exceed 9000 bytes".to_string()
        ));
    }
    Ok(())
}

/// Sanitize configuration values to prevent injection
///
/// Removes or rejects values containing dangerous characters
pub fn sanitize_config_value(value: &str) -> NetctlResult<String> {
    // Check for control characters
    if value.chars().any(|c| c.is_control() && c != '\t') {
        return Err(NetctlError::InvalidParameter(
            "Configuration value contains invalid control characters".to_string()
        ));
    }

    // Check for null bytes
    if value.contains('\0') {
        return Err(NetctlError::InvalidParameter(
            "Configuration value contains null byte".to_string()
        ));
    }

    // Limit length
    if value.len() > MAX_CONFIG_VALUE_LEN {
        return Err(NetctlError::InvalidParameter(
            format!("Configuration value too long (max {} characters)", MAX_CONFIG_VALUE_LEN)
        ));
    }

    Ok(value.to_string())
}

/// Validate WiFi SSID
///
/// SSIDs can be 0-32 bytes (can include non-ASCII, but we'll be conservative)
pub fn validate_ssid(ssid: &str) -> NetctlResult<()> {
    if ssid.is_empty() {
        return Err(NetctlError::InvalidParameter(
            "SSID cannot be empty".to_string()
        ));
    }

    if ssid.len() > 32 {
        return Err(NetctlError::InvalidParameter(
            "SSID cannot exceed 32 characters".to_string()
        ));
    }

    // Check for newlines and other control characters that could break config
    if ssid.chars().any(|c| c.is_control()) {
        return Err(NetctlError::InvalidParameter(
            "SSID contains invalid control characters".to_string()
        ));
    }

    Ok(())
}

/// Validate WiFi password (WPA2/WPA3)
///
/// WPA2 requirements: 8-63 ASCII characters
pub fn validate_wifi_password(password: &str) -> NetctlResult<()> {
    if password.len() < 8 {
        return Err(NetctlError::InvalidParameter(
            "WiFi password must be at least 8 characters".to_string()
        ));
    }

    if password.len() > 63 {
        return Err(NetctlError::InvalidParameter(
            "WiFi password cannot exceed 63 characters".to_string()
        ));
    }

    // WPA2 requires ASCII
    if !password.is_ascii() {
        return Err(NetctlError::InvalidParameter(
            "WiFi password must contain only ASCII characters".to_string()
        ));
    }

    // Check for control characters
    if password.chars().any(|c| c.is_control()) {
        return Err(NetctlError::InvalidParameter(
            "WiFi password contains invalid control characters".to_string()
        ));
    }

    Ok(())
}

/// Validate country code (ISO 3166-1 alpha-2)
pub fn validate_country_code(code: &str) -> NetctlResult<()> {
    // List of common country codes - in production, use a complete list
    const VALID_CODES: &[&str] = &[
        "US", "GB", "DE", "FR", "CA", "AU", "JP", "CN", "IN", "BR",
        "RU", "IT", "ES", "KR", "MX", "NL", "SE", "CH", "NO", "DK",
        "FI", "BE", "AT", "PL", "CZ", "PT", "GR", "IE", "NZ", "SG",
    ];

    let code_upper = code.to_uppercase();

    if code_upper.len() != 2 {
        return Err(NetctlError::InvalidParameter(
            "Country code must be 2 characters".to_string()
        ));
    }

    if !code_upper.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(NetctlError::InvalidParameter(
            "Country code must contain only letters".to_string()
        ));
    }

    if !VALID_CODES.contains(&code_upper.as_str()) {
        return Err(NetctlError::InvalidParameter(
            format!("Unsupported country code: {}", code)
        ));
    }

    Ok(())
}

/// Validate WiFi channel for a given band and country
pub fn validate_wifi_channel(channel: u8, band: &str) -> NetctlResult<()> {
    let valid_channels: &[u8] = match band {
        "2.4GHz" => &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
        "5GHz" => &[36, 40, 44, 48, 52, 56, 60, 64, 100, 104, 108, 112,
                    116, 120, 124, 128, 132, 136, 140, 149, 153, 157, 161, 165],
        _ => return Err(NetctlError::InvalidParameter(
            format!("Invalid band: {}", band)
        )),
    };

    if !valid_channels.contains(&channel) {
        return Err(NetctlError::InvalidParameter(
            format!("Invalid channel {} for band {}", channel, band)
        ));
    }

    Ok(())
}

/// Validate and canonicalize a path, ensuring it's within allowed directory
pub fn validate_config_path(path: &Path, allowed_base: &Path) -> NetctlResult<PathBuf> {
    // Get canonical (absolute, symlink-resolved) paths
    let canonical = path.canonicalize()
        .or_else(|_| {
            // If path doesn't exist yet, canonicalize parent
            if let Some(parent) = path.parent() {
                let parent_canonical = parent.canonicalize()
                    .map_err(|_| NetctlError::InvalidParameter(
                        "Invalid config path: parent directory does not exist".to_string()
                    ))?;
                if let Some(filename) = path.file_name() {
                    Ok(parent_canonical.join(filename))
                } else {
                    Err(NetctlError::InvalidParameter("Invalid config path".to_string()))
                }
            } else {
                Err(NetctlError::InvalidParameter("Invalid config path".to_string()))
            }
        })?;

    let base_canonical = allowed_base.canonicalize()
        .map_err(|_| NetctlError::ConfigError(
            "Config directory not found".to_string()
        ))?;

    if !canonical.starts_with(&base_canonical) {
        return Err(NetctlError::InvalidParameter(
            "Config path outside allowed directory".to_string()
        ));
    }

    // Check for symlinks in the path
    if path.exists() {
        let metadata = std::fs::symlink_metadata(path)
            .map_err(|_| NetctlError::InvalidParameter("Cannot read path metadata".to_string()))?;

        if metadata.file_type().is_symlink() {
            return Err(NetctlError::InvalidParameter(
                "Config path cannot be a symlink".to_string()
            ));
        }
    }

    Ok(canonical)
}

/// Sanitize error messages to prevent information disclosure
pub fn sanitize_error_message(stderr: &str) -> String {
    let mut sanitized = stderr.to_string();

    // Redact potential sensitive patterns (simplified for now)
    // In production, use regex to redact IP addresses, passwords, etc.
    // For now, just limit the length to prevent information disclosure

    // Limit length
    if sanitized.len() > MAX_ERROR_MESSAGE_LEN {
        sanitized.truncate(MAX_ERROR_MESSAGE_LEN);
        sanitized.push_str("... (truncated)");
    }

    sanitized
}

/// Validate hostname for ping/debug commands
pub fn validate_hostname(host: &str) -> NetctlResult<()> {
    if host.is_empty() {
        return Err(NetctlError::InvalidParameter(
            "Hostname cannot be empty".to_string()
        ));
    }

    if host.len() > 253 {
        return Err(NetctlError::InvalidParameter(
            "Hostname too long".to_string()
        ));
    }

    // Try parsing as IP address first
    if host.parse::<IpAddr>().is_ok() {
        return Ok(());
    }

    // Otherwise validate as hostname
    // Hostnames can contain alphanumeric, dash, and dots
    for c in host.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' && c != '.' {
            return Err(NetctlError::InvalidParameter(
                format!("Invalid hostname character: {}", c)
            ));
        }
    }

    // No leading/trailing dashes or dots
    if host.starts_with('-') || host.starts_with('.') ||
       host.ends_with('-') || host.ends_with('.') {
        return Err(NetctlError::InvalidParameter(
            "Invalid hostname format".to_string()
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface_name_validation() {
        // Valid names
        assert!(validate_interface_name("eth0").is_ok());
        assert!(validate_interface_name("wlan0").is_ok());
        assert!(validate_interface_name("br-lan").is_ok());
        assert!(validate_interface_name("veth_test").is_ok());

        // Invalid names - command injection attempts
        assert!(validate_interface_name("eth0; rm -rf /").is_err());
        assert!(validate_interface_name("wlan0`curl evil.com`").is_err());
        assert!(validate_interface_name("eth0 && echo pwned").is_err());
        assert!(validate_interface_name("wlan0|ls").is_err());
        assert!(validate_interface_name("eth0$evil").is_err());
        assert!(validate_interface_name("wlan0\nmalicious").is_err());

        // Invalid - too long
        assert!(validate_interface_name("verylonginterfacename").is_err());

        // Invalid - starts with dash
        assert!(validate_interface_name("-eth0").is_err());

        // Invalid - empty
        assert!(validate_interface_name("").is_err());
    }

    #[test]
    fn test_ip_validation() {
        // Valid IPv4
        assert!(validate_ip_address("192.168.1.1").is_ok());
        assert!(validate_ip_address("10.0.0.1").is_ok());

        // Valid IPv6
        assert!(validate_ip_address("::1").is_ok());
        assert!(validate_ip_address("fe80::1").is_ok());

        // Invalid
        assert!(validate_ip_address("256.1.1.1").is_err());
        assert!(validate_ip_address("192.168.1.1; rm -rf /").is_err());
        assert!(validate_ip_address("not_an_ip").is_err());
    }

    #[test]
    fn test_mac_validation() {
        // Valid MAC
        assert!(validate_mac_address("00:11:22:33:44:55").is_ok());
        assert!(validate_mac_address("AA:BB:CC:DD:EE:FF").is_ok());

        // Invalid format
        assert!(validate_mac_address("00:11:22:33:44").is_err());
        assert!(validate_mac_address("00-11-22-33-44-55").is_err());
        assert!(validate_mac_address("invalid").is_err());
        assert!(validate_mac_address("00:11:22:33:44:GG").is_err());
    }

    #[test]
    fn test_ssid_validation() {
        assert!(validate_ssid("MyNetwork").is_ok());
        assert!(validate_ssid("Test-WiFi_123").is_ok());

        // Empty SSID
        assert!(validate_ssid("").is_err());

        // Too long
        assert!(validate_ssid("ThisIsAVeryLongSSIDThatExceedsTheMaximumLength").is_err());

        // Control characters
        assert!(validate_ssid("SSID\nwith\nnewlines").is_err());
    }

    #[test]
    fn test_wifi_password_validation() {
        // Valid passwords
        assert!(validate_wifi_password("password123").is_ok());
        assert!(validate_wifi_password("SecureP@ss123").is_ok());

        // Too short
        assert!(validate_wifi_password("short").is_err());

        // Too long
        let long_pass = "a".repeat(64);
        assert!(validate_wifi_password(&long_pass).is_err());

        // Non-ASCII
        assert!(validate_wifi_password("pässwörd").is_err());

        // Control characters
        assert!(validate_wifi_password("pass\nword").is_err());
    }

    #[test]
    fn test_country_code_validation() {
        // Valid codes
        assert!(validate_country_code("US").is_ok());
        assert!(validate_country_code("us").is_ok());  // Should work with lowercase
        assert!(validate_country_code("GB").is_ok());

        // Invalid
        assert!(validate_country_code("USA").is_err());  // Too long
        assert!(validate_country_code("X").is_err());    // Too short
        assert!(validate_country_code("99").is_err());   // Not letters
        assert!(validate_country_code("XX").is_err());   // Not in list
    }

    #[test]
    fn test_wifi_channel_validation() {
        // Valid 2.4GHz channels
        assert!(validate_wifi_channel(1, "2.4GHz").is_ok());
        assert!(validate_wifi_channel(11, "2.4GHz").is_ok());

        // Invalid channel for 2.4GHz
        assert!(validate_wifi_channel(36, "2.4GHz").is_err());

        // Valid 5GHz channels
        assert!(validate_wifi_channel(36, "5GHz").is_ok());
        assert!(validate_wifi_channel(165, "5GHz").is_ok());

        // Invalid channel for 5GHz
        assert!(validate_wifi_channel(1, "5GHz").is_err());

        // Invalid band
        assert!(validate_wifi_channel(1, "invalid").is_err());
    }

    #[test]
    fn test_hostname_validation() {
        // Valid hostnames
        assert!(validate_hostname("example.com").is_ok());
        assert!(validate_hostname("sub.example.com").is_ok());
        assert!(validate_hostname("192.168.1.1").is_ok());
        assert!(validate_hostname("host-name").is_ok());

        // Invalid
        assert!(validate_hostname("").is_err());
        assert!(validate_hostname("-invalid").is_err());
        assert!(validate_hostname("invalid-").is_err());
        assert!(validate_hostname(".invalid").is_err());
        assert!(validate_hostname("invalid.").is_err());
        assert!(validate_hostname("host name").is_err());  // Space
        assert!(validate_hostname("host;name").is_err());  // Semicolon
    }
}
