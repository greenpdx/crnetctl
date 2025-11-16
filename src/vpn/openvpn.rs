use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;
use tokio::process::{Child, Command};
use tracing::{debug, info, warn};

use crate::plugin::ConnectionConfig;
use crate::error::{NetctlError, NetctlResult};
use super::backend::{VpnBackend, VpnState, VpnStats};
use super::common;

/// OpenVPN backend implementation
pub struct OpenVpnBackend {
    process: Option<Child>,
    interface_name: Option<String>,
    connected_since: Option<SystemTime>,
    #[allow(dead_code)]
    config_path: Option<std::path::PathBuf>,
}

impl OpenVpnBackend {
    /// Create a new OpenVPN backend instance
    pub fn new() -> Self {
        Self {
            process: None,
            interface_name: None,
            connected_since: None,
            config_path: None,
        }
    }

    /// Build OpenVPN command arguments from configuration
    fn build_command_args(&self, config: &ConnectionConfig) -> NetctlResult<Vec<String>> {
        let settings = &config.settings;
        let mut args = Vec::new();

        // Use config file if specified
        if let Some(config_file) = settings.get("config_file").and_then(|v| v.as_str()) {
            args.push("--config".to_string());
            args.push(config_file.to_string());
        } else {
            // Build command line from individual settings
            if let Some(remote) = settings.get("remote").and_then(|v| v.as_str()) {
                args.push("--remote".to_string());
                args.push(remote.to_string());
            } else {
                return Err(NetctlError::InvalidParameter(
                    "Either 'config_file' or 'remote' must be specified".to_string()
                ));
            }

            if let Some(port) = settings.get("port").and_then(|v| v.as_u64()) {
                args.push("--port".to_string());
                args.push(port.to_string());
            }

            if let Some(proto) = settings.get("proto").and_then(|v| v.as_str()) {
                args.push("--proto".to_string());
                args.push(proto.to_string());
            }

            if let Some(dev_type) = settings.get("dev_type").and_then(|v| v.as_str()) {
                args.push("--dev-type".to_string());
                args.push(dev_type.to_string());
            }

            if let Some(dev) = settings.get("dev").and_then(|v| v.as_str()) {
                args.push("--dev".to_string());
                args.push(dev.to_string());
            }

            // Certificate files
            if let Some(ca) = settings.get("ca").and_then(|v| v.as_str()) {
                args.push("--ca".to_string());
                args.push(ca.to_string());
            }

            if let Some(cert) = settings.get("cert").and_then(|v| v.as_str()) {
                args.push("--cert".to_string());
                args.push(cert.to_string());
            }

            if let Some(key) = settings.get("key").and_then(|v| v.as_str()) {
                args.push("--key".to_string());
                args.push(key.to_string());
            }

            // TLS auth
            if let Some(tls_auth) = settings.get("tls_auth").and_then(|v| v.as_str()) {
                args.push("--tls-auth".to_string());
                args.push(tls_auth.to_string());

                if let Some(key_direction) = settings.get("key_direction").and_then(|v| v.as_u64()) {
                    args.push(key_direction.to_string());
                }
            }

            // Cipher
            if let Some(cipher) = settings.get("cipher").and_then(|v| v.as_str()) {
                args.push("--cipher".to_string());
                args.push(cipher.to_string());
            }

            // Authentication
            if let Some(auth) = settings.get("auth").and_then(|v| v.as_str()) {
                args.push("--auth".to_string());
                args.push(auth.to_string());
            }

            // Compression
            if let Some(comp_lzo) = settings.get("comp_lzo").and_then(|v| v.as_bool()) {
                if comp_lzo {
                    args.push("--comp-lzo".to_string());
                }
            }

            // Username/password authentication
            if let Some(auth_user_pass) = settings.get("auth_user_pass").and_then(|v| v.as_str()) {
                args.push("--auth-user-pass".to_string());
                args.push(auth_user_pass.to_string());
            }
        }

        // Common options for client mode
        args.push("--client".to_string());
        args.push("--nobind".to_string());
        args.push("--persist-key".to_string());
        args.push("--persist-tun".to_string());

        // Verbose output for debugging
        if settings.get("verbose").and_then(|v| v.as_bool()).unwrap_or(false) {
            args.push("--verb".to_string());
            args.push("3".to_string());
        }

        Ok(args)
    }

    /// Build OpenVPN configuration file content
    fn build_config_content(&self, config: &ConnectionConfig) -> NetctlResult<String> {
        let settings = &config.settings;
        let mut cfg = String::new();

        // Client mode
        cfg.push_str("client\n");
        cfg.push_str("nobind\n");
        cfg.push_str("persist-key\n");
        cfg.push_str("persist-tun\n");

        // Remote server
        if let Some(remote) = settings.get("remote").and_then(|v| v.as_str()) {
            let port = settings.get("port").and_then(|v| v.as_u64()).unwrap_or(1194);
            let proto = settings.get("proto").and_then(|v| v.as_str()).unwrap_or("udp");
            cfg.push_str(&format!("remote {} {} {}\n", remote, port, proto));
        }

        // Device type
        let dev_type = settings.get("dev_type").and_then(|v| v.as_str()).unwrap_or("tun");
        cfg.push_str(&format!("dev {}\n", dev_type));

        // Certificates
        if let Some(ca) = settings.get("ca").and_then(|v| v.as_str()) {
            cfg.push_str(&format!("ca {}\n", ca));
        }

        if let Some(cert) = settings.get("cert").and_then(|v| v.as_str()) {
            cfg.push_str(&format!("cert {}\n", cert));
        }

        if let Some(key) = settings.get("key").and_then(|v| v.as_str()) {
            cfg.push_str(&format!("key {}\n", key));
        }

        // TLS auth
        if let Some(tls_auth) = settings.get("tls_auth").and_then(|v| v.as_str()) {
            let key_direction = settings.get("key_direction").and_then(|v| v.as_u64()).unwrap_or(1);
            cfg.push_str(&format!("tls-auth {} {}\n", tls_auth, key_direction));
        }

        // Cipher
        if let Some(cipher) = settings.get("cipher").and_then(|v| v.as_str()) {
            cfg.push_str(&format!("cipher {}\n", cipher));
        }

        // Authentication
        if let Some(auth) = settings.get("auth").and_then(|v| v.as_str()) {
            cfg.push_str(&format!("auth {}\n", auth));
        }

        // Compression
        if settings.get("comp_lzo").and_then(|v| v.as_bool()).unwrap_or(false) {
            cfg.push_str("comp-lzo\n");
        }

        // Username/password
        if let Some(auth_user_pass) = settings.get("auth_user_pass").and_then(|v| v.as_str()) {
            cfg.push_str(&format!("auth-user-pass {}\n", auth_user_pass));
        }

        Ok(cfg)
    }

    /// Parse OpenVPN configuration file
    async fn parse_ovpn_config(path: &Path) -> NetctlResult<HashMap<String, Value>> {
        let content = common::read_config_file(path).await?;
        let mut settings = HashMap::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let key = parts[0].to_lowercase().replace("-", "_");

            match parts.len() {
                1 => {
                    // Boolean option
                    settings.insert(key, json!(true));
                }
                2 => {
                    // Key-value option
                    settings.insert(key, json!(parts[1]));
                }
                3 => {
                    // Key with two values (e.g., remote host port)
                    if key == "remote" {
                        settings.insert("remote".to_string(), json!(parts[1]));
                        if let Ok(port) = parts[2].parse::<u64>() {
                            settings.insert("port".to_string(), json!(port));
                        }
                    } else {
                        settings.insert(key, json!(vec![parts[1], parts[2]]));
                    }
                }
                4 => {
                    // Remote with protocol
                    if key == "remote" {
                        settings.insert("remote".to_string(), json!(parts[1]));
                        if let Ok(port) = parts[2].parse::<u64>() {
                            settings.insert("port".to_string(), json!(port));
                        }
                        settings.insert("proto".to_string(), json!(parts[3]));
                    }
                }
                _ => {}
            }
        }

        Ok(settings)
    }

    /// Get the process ID of the OpenVPN process
    fn get_pid(&self) -> Option<u32> {
        self.process.as_ref().and_then(|p| p.id())
    }
}

#[async_trait]
impl VpnBackend for OpenVpnBackend {
    fn name(&self) -> &str {
        "openvpn"
    }

    async fn version(&self) -> NetctlResult<String> {
        common::get_binary_version("openvpn").await
    }

    async fn is_available(&self) -> bool {
        common::check_binary_available("openvpn").await
    }

    async fn validate_config(&self, config: &ConnectionConfig) -> NetctlResult<()> {
        let settings = &config.settings;

        // Check required fields
        if !settings.contains_key("config_file") && !settings.contains_key("remote") {
            return Err(NetctlError::InvalidParameter(
                "Either 'config_file' or 'remote' must be specified".to_string()
            ));
        }

        // Validate config file exists if specified
        if let Some(config_file) = settings.get("config_file").and_then(|v| v.as_str()) {
            let path = Path::new(config_file);
            if !path.exists() {
                return Err(NetctlError::InvalidParameter(
                    format!("Config file not found: {}", config_file)
                ));
            }
        }

        // Validate device type
        if let Some(dev_type) = settings.get("dev_type").and_then(|v| v.as_str()) {
            if !["tun", "tap"].contains(&dev_type) {
                return Err(NetctlError::InvalidParameter(
                    "dev_type must be 'tun' or 'tap'".to_string()
                ));
            }
        }

        // Validate protocol
        if let Some(proto) = settings.get("proto").and_then(|v| v.as_str()) {
            if !["udp", "tcp", "udp4", "udp6", "tcp4", "tcp6"].contains(&proto) {
                return Err(NetctlError::InvalidParameter(
                    format!("Invalid protocol: {}", proto)
                ));
            }
        }

        Ok(())
    }

    async fn connect(&mut self, config: &ConnectionConfig) -> NetctlResult<String> {
        info!("Connecting OpenVPN: {}", config.name);

        // Build command arguments
        let args = self.build_command_args(config)?;

        // Start OpenVPN process
        let child = Command::new("openvpn")
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| NetctlError::ServiceError(format!("Failed to start OpenVPN: {}", e)))?;

        let pid = child.id().ok_or_else(||
            NetctlError::ServiceError("Failed to get OpenVPN process ID".to_string())
        )?;

        // Generate interface name (OpenVPN typically creates tun0, tun1, etc.)
        let interface_name = config.settings
            .get("dev")
            .and_then(|v| v.as_str())
            .unwrap_or("tun0")
            .to_string();

        self.process = Some(child);
        self.interface_name = Some(interface_name.clone());
        self.connected_since = Some(SystemTime::now());

        info!("OpenVPN connected: {} (PID: {}, interface: {})", config.name, pid, interface_name);

        // Give OpenVPN a moment to bring up the interface
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        Ok(interface_name)
    }

    async fn disconnect(&mut self) -> NetctlResult<()> {
        if let Some(mut process) = self.process.take() {
            info!("Disconnecting OpenVPN");

            // Try graceful shutdown first
            if let Err(e) = process.kill().await {
                warn!("Failed to kill OpenVPN process: {}", e);
            }

            // Wait for process to exit
            match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                process.wait()
            ).await {
                Ok(Ok(status)) => {
                    debug!("OpenVPN process exited with status: {}", status);
                }
                Ok(Err(e)) => {
                    warn!("Error waiting for OpenVPN process: {}", e);
                }
                Err(_) => {
                    warn!("Timeout waiting for OpenVPN process to exit");
                }
            }

            self.interface_name = None;
            self.connected_since = None;

            info!("OpenVPN disconnected");
        }

        Ok(())
    }

    async fn state(&self) -> VpnState {
        if let Some(pid) = self.get_pid() {
            // Check if process is still running by checking if PID exists
            let pid_exists = std::path::Path::new(&format!("/proc/{}", pid)).exists();

            if !pid_exists {
                VpnState::Failed("Process has exited".to_string())
            } else {
                // Process still running
                if let Some(interface_name) = &self.interface_name {
                    if common::interface_exists(interface_name).await {
                        VpnState::Connected
                    } else {
                        VpnState::Connecting
                    }
                } else {
                    VpnState::Connecting
                }
            }
        } else {
            VpnState::Disconnected
        }
    }

    async fn stats(&self) -> NetctlResult<VpnStats> {
        let mut stats = VpnStats::default();
        stats.connected_since = self.connected_since;

        if let Some(interface_name) = &self.interface_name {
            // Get interface statistics
            if let Ok((rx_bytes, tx_bytes)) = common::get_interface_stats(interface_name).await {
                stats.bytes_received = rx_bytes;
                stats.bytes_sent = tx_bytes;
            }
        }

        Ok(stats)
    }

    fn interface_name(&self) -> Option<String> {
        self.interface_name.clone()
    }

    async fn status_json(&self) -> NetctlResult<Value> {
        let state = self.state().await;
        let stats = self.stats().await.unwrap_or_default();

        Ok(json!({
            "backend": "openvpn",
            "state": format!("{:?}", state),
            "interface": self.interface_name,
            "pid": self.get_pid(),
            "connected_since": stats.connected_since.map(|t| format!("{:?}", t)),
            "bytes_sent": stats.bytes_sent,
            "bytes_received": stats.bytes_received,
        }))
    }

    async fn import_config(&self, path: &Path) -> NetctlResult<HashMap<String, Value>> {
        info!("Importing OpenVPN configuration from: {:?}", path);

        // For .ovpn files, we typically just reference them directly
        let mut settings = Self::parse_ovpn_config(path).await?;

        // Also store the config file path for direct use
        settings.insert("config_file".to_string(), json!(path.to_string_lossy()));

        Ok(settings)
    }

    async fn export_config(&self, config: &ConnectionConfig, path: &Path) -> NetctlResult<()> {
        info!("Exporting OpenVPN configuration to: {:?}", path);

        // If there's a source config file, copy it
        if let Some(config_file) = config.settings.get("config_file").and_then(|v| v.as_str()) {
            let source = Path::new(config_file);
            tokio::fs::copy(source, path).await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to copy config file: {}", e)))?;
        } else {
            // Otherwise, build config from settings
            let config_content = self.build_config_content(config)?;
            common::write_secure_config(path, &config_content, 0o600).await?;
        }

        Ok(())
    }
}

impl Drop for OpenVpnBackend {
    fn drop(&mut self) {
        if let Some(ref mut process) = self.process {
            // Attempt to kill the process on drop
            // Note: This is synchronous and may not complete
            if let Some(pid) = process.id() {
                let _ = std::process::Command::new("kill")
                    .arg("-TERM")
                    .arg(pid.to_string())
                    .output();
            }
        }
    }
}

/// Factory function to create an OpenVPN backend
pub fn create_backend() -> Box<dyn VpnBackend> {
    Box::new(OpenVpnBackend::new())
}
