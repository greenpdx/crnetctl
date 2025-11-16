use crate::error::{NetctlError, NetctlResult};
use std::path::Path;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Common VPN utility functions shared across all backends

/// Check if a binary is available in the system PATH
pub async fn check_binary_available(binary: &str) -> bool {
    match Command::new("which")
        .arg(binary)
        .output()
        .await
    {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// Get the version of a binary by running it with --version
pub async fn get_binary_version(binary: &str) -> NetctlResult<String> {
    let output = Command::new(binary)
        .arg("--version")
        .output()
        .await
        .map_err(|e| NetctlError::ServiceError(format!("Failed to get {} version: {}", binary, e)))?;

    if !output.status.success() {
        return Err(NetctlError::ServiceError(format!(
            "{} --version failed: {}",
            binary,
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let version_output = String::from_utf8_lossy(&output.stdout);
    Ok(version_output.lines().next().unwrap_or("unknown").to_string())
}

/// Ensure a directory exists, creating it if necessary
pub async fn ensure_directory_exists(path: &Path) -> NetctlResult<()> {
    if !path.exists() {
        tokio::fs::create_dir_all(path)
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to create directory {:?}: {}", path, e)))?;
        info!("Created directory: {:?}", path);
    }
    Ok(())
}

/// Write configuration to a file securely (with appropriate permissions)
pub async fn write_secure_config(path: &Path, content: &str, permissions: u32) -> NetctlResult<()> {
    use std::os::unix::fs::PermissionsExt;

    tokio::fs::write(path, content)
        .await
        .map_err(|e| NetctlError::ServiceError(format!("Failed to write config to {:?}: {}", path, e)))?;

    let perms = std::fs::Permissions::from_mode(permissions);
    tokio::fs::set_permissions(path, perms)
        .await
        .map_err(|e| NetctlError::ServiceError(format!("Failed to set permissions on {:?}: {}", path, e)))?;

    debug!("Wrote config to {:?} with permissions {:o}", path, permissions);
    Ok(())
}

/// Read a configuration file
pub async fn read_config_file(path: &Path) -> NetctlResult<String> {
    tokio::fs::read_to_string(path)
        .await
        .map_err(|e| NetctlError::ServiceError(format!("Failed to read config from {:?}: {}", path, e)))
}

/// Delete a configuration file if it exists
pub async fn delete_config_file(path: &Path) -> NetctlResult<()> {
    if path.exists() {
        tokio::fs::remove_file(path)
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to delete {:?}: {}", path, e)))?;
        debug!("Deleted config file: {:?}", path);
    }
    Ok(())
}

/// Kill a process by PID
pub async fn kill_process(pid: u32) -> NetctlResult<()> {
    let output = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .output()
        .await
        .map_err(|e| NetctlError::ServiceError(format!("Failed to kill process {}: {}", pid, e)))?;

    if !output.status.success() {
        warn!("Failed to kill process {}: {}", pid, String::from_utf8_lossy(&output.stderr));

        // Try SIGKILL as fallback
        let output = Command::new("kill")
            .arg("-KILL")
            .arg(pid.to_string())
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to force kill process {}: {}", pid, e)))?;

        if !output.status.success() {
            return Err(NetctlError::ServiceError(format!("Failed to kill process {}", pid)));
        }
    }

    debug!("Killed process {}", pid);
    Ok(())
}

/// Check if an interface exists
pub async fn interface_exists(interface: &str) -> bool {
    Path::new(&format!("/sys/class/net/{}", interface)).exists()
}

/// Get interface statistics from /sys/class/net
pub async fn get_interface_stats(interface: &str) -> NetctlResult<(u64, u64)> {
    let base_path = format!("/sys/class/net/{}/statistics", interface);

    let rx_bytes = tokio::fs::read_to_string(format!("{}/rx_bytes", base_path))
        .await
        .map_err(|e| NetctlError::ServiceError(format!("Failed to read rx_bytes: {}", e)))?
        .trim()
        .parse::<u64>()
        .unwrap_or(0);

    let tx_bytes = tokio::fs::read_to_string(format!("{}/tx_bytes", base_path))
        .await
        .map_err(|e| NetctlError::ServiceError(format!("Failed to read tx_bytes: {}", e)))?
        .trim()
        .parse::<u64>()
        .unwrap_or(0);

    Ok((rx_bytes, tx_bytes))
}

/// Parse key=value configuration format (common in VPN configs)
pub fn parse_key_value_config(content: &str) -> std::collections::HashMap<String, String> {
    let mut config = std::collections::HashMap::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        // Parse key=value or key value
        if let Some((key, value)) = line.split_once('=').or_else(|| line.split_once(' ')) {
            config.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    config
}

/// Validate an IPv4 address
pub fn is_valid_ipv4(addr: &str) -> bool {
    addr.parse::<std::net::Ipv4Addr>().is_ok()
}

/// Validate an IPv6 address
pub fn is_valid_ipv6(addr: &str) -> bool {
    addr.parse::<std::net::Ipv6Addr>().is_ok()
}

/// Validate an IP address (v4 or v6)
pub fn is_valid_ip(addr: &str) -> bool {
    addr.parse::<std::net::IpAddr>().is_ok()
}

/// Validate a CIDR notation (e.g., "10.0.0.1/24")
pub fn is_valid_cidr(cidr: &str) -> bool {
    if let Some((ip, prefix)) = cidr.split_once('/') {
        if let Ok(prefix_len) = prefix.parse::<u8>() {
            if is_valid_ipv4(ip) {
                return prefix_len <= 32;
            } else if is_valid_ipv6(ip) {
                return prefix_len <= 128;
            }
        }
    }
    false
}
