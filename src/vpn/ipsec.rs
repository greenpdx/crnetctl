use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;
use tokio::process::Command;
use tracing::{debug, info, warn};

use crate::plugin::ConnectionConfig;
use crate::error::{NetctlError, NetctlResult};
use super::backend::{VpnBackend, VpnState, VpnStats};
use super::common;

/// IPsec/IKEv2 backend implementation using strongSwan
/// This driver supports IKEv2/IPsec connections and is compatible with
/// FreeSWAN, Libreswan, and strongSwan configurations.
pub struct IPsecBackend {
    connection_name: Option<String>,
    interface_name: Option<String>,
    connected_since: Option<SystemTime>,
    config_base_path: std::path::PathBuf,
}

impl IPsecBackend {
    /// Create a new IPsec backend instance
    pub fn new() -> Self {
        Self {
            connection_name: None,
            interface_name: None,
            connected_since: None,
            config_base_path: std::path::PathBuf::from("/etc/ipsec.d"),
        }
    }

    /// Build IPsec connection configuration
    fn build_ipsec_conf(&self, config: &ConnectionConfig) -> NetctlResult<String> {
        let settings = &config.settings;
        let conn_name = config.name.replace(" ", "_");
        let mut conf = String::new();

        conf.push_str(&format!("conn {}\n", conn_name));

        // Connection type
        let conn_type = settings.get("type").and_then(|v| v.as_str()).unwrap_or("tunnel");
        conf.push_str(&format!("\ttype={}\n", conn_type));

        // Auto start
        let auto = settings.get("auto").and_then(|v| v.as_str()).unwrap_or("start");
        conf.push_str(&format!("\tauto={}\n", auto));

        // Key exchange version (ikev1, ikev2)
        let keyexchange = settings.get("keyexchange").and_then(|v| v.as_str()).unwrap_or("ikev2");
        conf.push_str(&format!("\tkeyexchange={}\n", keyexchange));

        // Left (local) configuration
        conf.push_str("\t# Local configuration\n");

        if let Some(leftid) = settings.get("leftid").and_then(|v| v.as_str()) {
            conf.push_str(&format!("\tleftid={}\n", leftid));
        }

        if let Some(leftcert) = settings.get("leftcert").and_then(|v| v.as_str()) {
            conf.push_str(&format!("\tleftcert={}\n", leftcert));
        }

        if let Some(leftauth) = settings.get("leftauth").and_then(|v| v.as_str()) {
            conf.push_str(&format!("\tleftauth={}\n", leftauth));
        } else {
            conf.push_str("\tleftauth=pubkey\n");
        }

        if let Some(leftsourceip) = settings.get("leftsourceip").and_then(|v| v.as_str()) {
            conf.push_str(&format!("\tleftsourceip={}\n", leftsourceip));
        }

        // Right (remote) configuration
        conf.push_str("\t# Remote configuration\n");

        if let Some(right) = settings.get("right").and_then(|v| v.as_str()) {
            conf.push_str(&format!("\tright={}\n", right));
        } else {
            return Err(NetctlError::InvalidParameter("'right' (remote gateway) is required".to_string()));
        }

        if let Some(rightid) = settings.get("rightid").and_then(|v| v.as_str()) {
            conf.push_str(&format!("\trightid={}\n", rightid));
        }

        if let Some(rightauth) = settings.get("rightauth").and_then(|v| v.as_str()) {
            conf.push_str(&format!("\trightauth={}\n", rightauth));
        } else {
            conf.push_str("\trightauth=pubkey\n");
        }

        if let Some(rightsubnet) = settings.get("rightsubnet").and_then(|v| v.as_str()) {
            conf.push_str(&format!("\trightsubnet={}\n", rightsubnet));
        }

        // IKE and ESP proposals
        if let Some(ike) = settings.get("ike").and_then(|v| v.as_str()) {
            conf.push_str(&format!("\tike={}\n", ike));
        }

        if let Some(esp) = settings.get("esp").and_then(|v| v.as_str()) {
            conf.push_str(&format!("\tesp={}\n", esp));
        }

        // DPD (Dead Peer Detection)
        if settings.get("dpdaction").is_some() {
            let dpdaction = settings.get("dpdaction").and_then(|v| v.as_str()).unwrap_or("restart");
            conf.push_str(&format!("\tdpdaction={}\n", dpdaction));

            let dpddelay = settings.get("dpddelay").and_then(|v| v.as_u64()).unwrap_or(30);
            conf.push_str(&format!("\tdpddelay={}s\n", dpddelay));

            let dpdtimeout = settings.get("dpdtimeout").and_then(|v| v.as_u64()).unwrap_or(150);
            conf.push_str(&format!("\tdpdtimeout={}s\n", dpdtimeout));
        }

        // Mark connections
        if let Some(mark) = settings.get("mark").and_then(|v| v.as_str()) {
            conf.push_str(&format!("\tmark={}\n", mark));
        }

        // Lifetime
        if let Some(ikelifetime) = settings.get("ikelifetime").and_then(|v| v.as_str()) {
            conf.push_str(&format!("\tikelifetime={}\n", ikelifetime));
        }

        if let Some(lifetime) = settings.get("lifetime").and_then(|v| v.as_str()) {
            conf.push_str(&format!("\tlifetime={}\n", lifetime));
        }

        // Rekey settings
        if let Some(rekey) = settings.get("rekey").and_then(|v| v.as_bool()) {
            conf.push_str(&format!("\trekey={}\n", if rekey { "yes" } else { "no" }));
        }

        // Additional options
        if let Some(closeaction) = settings.get("closeaction").and_then(|v| v.as_str()) {
            conf.push_str(&format!("\tcloseaction={}\n", closeaction));
        }

        Ok(conf)
    }

    /// Build IPsec secrets file entry
    fn build_ipsec_secrets(&self, config: &ConnectionConfig) -> NetctlResult<String> {
        let settings = &config.settings;
        let mut secrets = String::new();

        // PSK (Pre-Shared Key) authentication
        if let Some(psk) = settings.get("psk").and_then(|v| v.as_str()) {
            let leftid = settings.get("leftid").and_then(|v| v.as_str()).unwrap_or("%any");
            let rightid = settings.get("rightid").and_then(|v| v.as_str()).unwrap_or("%any");
            secrets.push_str(&format!("{} {} : PSK \"{}\"\n", leftid, rightid, psk));
        }

        // RSA private key
        if let Some(rsa_key) = settings.get("rsa_key").and_then(|v| v.as_str()) {
            secrets.push_str(&format!(": RSA {}\n", rsa_key));
        }

        // EAP authentication
        if let Some(eap_identity) = settings.get("eap_identity").and_then(|v| v.as_str()) {
            if let Some(eap_password) = settings.get("eap_password").and_then(|v| v.as_str()) {
                secrets.push_str(&format!("{} : EAP \"{}\"\n", eap_identity, eap_password));
            }
        }

        // XAUTH authentication
        if let Some(xauth_user) = settings.get("xauth_user").and_then(|v| v.as_str()) {
            if let Some(xauth_pass) = settings.get("xauth_pass").and_then(|v| v.as_str()) {
                secrets.push_str(&format!("{} : XAUTH \"{}\"\n", xauth_user, xauth_pass));
            }
        }

        Ok(secrets)
    }

    /// Parse IPsec configuration file
    async fn parse_ipsec_conf(path: &Path) -> NetctlResult<HashMap<String, Value>> {
        let content = common::read_config_file(path).await?;
        let mut settings = HashMap::new();
        let mut in_conn = false;

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Connection section
            if let Some(conn_name) = line.strip_prefix("conn ") {
                in_conn = true;
                let conn_name = conn_name.trim();
                if conn_name != "%default" {
                    settings.insert("connection_name".to_string(), json!(conn_name));
                }
                continue;
            }

            if !in_conn {
                continue;
            }

            // Parse key=value
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                settings.insert(key, json!(value));
            }
        }

        Ok(settings)
    }

    /// Get connection name
    fn get_connection_name(&self, config: &ConnectionConfig) -> String {
        config.name.replace(" ", "_")
    }

    /// Check if a connection is active
    async fn is_connection_active(&self, conn_name: &str) -> bool {
        match Command::new("ipsec")
            .args(&["status", conn_name])
            .output()
            .await
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.contains("ESTABLISHED") || stdout.contains("INSTALLED")
            }
            Err(_) => false,
        }
    }
}

#[async_trait]
impl VpnBackend for IPsecBackend {
    fn name(&self) -> &str {
        "ipsec"
    }

    async fn version(&self) -> NetctlResult<String> {
        // Try strongSwan first, then Libreswan
        if common::check_binary_available("ipsec").await {
            let output = Command::new("ipsec")
                .arg("--version")
                .output()
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to get IPsec version: {}", e)))?;

            let version_output = String::from_utf8_lossy(&output.stdout);
            Ok(version_output.lines().next().unwrap_or("unknown").to_string())
        } else {
            Err(NetctlError::NotSupported("IPsec not available".to_string()))
        }
    }

    async fn is_available(&self) -> bool {
        common::check_binary_available("ipsec").await
    }

    async fn validate_config(&self, config: &ConnectionConfig) -> NetctlResult<()> {
        let settings = &config.settings;

        // Check required fields
        if !settings.contains_key("right") {
            return Err(NetctlError::InvalidParameter(
                "'right' (remote gateway address) is required".to_string()
            ));
        }

        // Validate authentication method
        if !settings.contains_key("psk") &&
           !settings.contains_key("leftcert") &&
           !settings.contains_key("eap_identity") &&
           !settings.contains_key("xauth_user") {
            return Err(NetctlError::InvalidParameter(
                "At least one authentication method is required (psk, leftcert, eap_identity, or xauth_user)".to_string()
            ));
        }

        // Validate key exchange
        if let Some(keyexchange) = settings.get("keyexchange").and_then(|v| v.as_str()) {
            if !["ikev1", "ikev2", "ike"].contains(&keyexchange) {
                return Err(NetctlError::InvalidParameter(
                    format!("Invalid keyexchange: {}", keyexchange)
                ));
            }
        }

        // Validate remote address
        if let Some(right) = settings.get("right").and_then(|v| v.as_str()) {
            if !common::is_valid_ip(right) && right != "%any" {
                // Could be a hostname, which is valid
                if right.is_empty() {
                    return Err(NetctlError::InvalidParameter(
                        "Invalid 'right' address".to_string()
                    ));
                }
            }
        }

        Ok(())
    }

    async fn connect(&mut self, config: &ConnectionConfig) -> NetctlResult<String> {
        let conn_name = self.get_connection_name(config);
        info!("Connecting IPsec VPN: {}", conn_name);

        // Build configuration files
        let conf_content = self.build_ipsec_conf(config)?;
        let secrets_content = self.build_ipsec_secrets(config)?;

        // Write configuration files
        let conf_path = self.config_base_path.join(format!("connections/{}.conf", conn_name));
        let secrets_path = self.config_base_path.join(format!("secrets/{}.secrets", conn_name));

        let conf_parent = conf_path.parent()
            .ok_or_else(|| NetctlError::InvalidParameter("Invalid config path".to_string()))?;
        let secrets_parent = secrets_path.parent()
            .ok_or_else(|| NetctlError::InvalidParameter("Invalid secrets path".to_string()))?;

        common::ensure_directory_exists(conf_parent).await?;
        common::ensure_directory_exists(secrets_parent).await?;

        common::write_secure_config(&conf_path, &conf_content, 0o644).await?;
        common::write_secure_config(&secrets_path, &secrets_content, 0o600).await?;

        // Reload IPsec configuration
        let output = Command::new("ipsec")
            .arg("reload")
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to reload IPsec: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(NetctlError::ServiceError(
                format!("IPsec reload failed: {}", error_msg)
            ));
        }

        // Start the connection
        let output = Command::new("ipsec")
            .args(&["up", &conn_name])
            .output()
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to start IPsec connection: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(NetctlError::ServiceError(
                format!("IPsec connection failed: {}", error_msg)
            ));
        }

        // Determine interface name (usually something like ipsec0 or the actual interface)
        let interface_name = format!("ipsec-{}", conn_name);

        self.connection_name = Some(conn_name);
        self.interface_name = Some(interface_name.clone());
        self.connected_since = Some(SystemTime::now());

        info!("IPsec VPN connected: {}", config.name);
        Ok(interface_name)
    }

    async fn disconnect(&mut self) -> NetctlResult<()> {
        if let Some(conn_name) = &self.connection_name {
            info!("Disconnecting IPsec VPN: {}", conn_name);

            let output = Command::new("ipsec")
                .args(&["down", conn_name])
                .output()
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to stop IPsec connection: {}", e)))?;

            if !output.status.success() {
                warn!("IPsec down failed: {}", String::from_utf8_lossy(&output.stderr));
            }

            // Clean up configuration files
            let conf_path = self.config_base_path.join(format!("connections/{}.conf", conn_name));
            let secrets_path = self.config_base_path.join(format!("secrets/{}.secrets", conn_name));

            common::delete_config_file(&conf_path).await.ok();
            common::delete_config_file(&secrets_path).await.ok();

            self.connection_name = None;
            self.interface_name = None;
            self.connected_since = None;

            info!("IPsec VPN disconnected");
        }

        Ok(())
    }

    async fn state(&self) -> VpnState {
        if let Some(conn_name) = &self.connection_name {
            if self.is_connection_active(conn_name).await {
                VpnState::Connected
            } else {
                VpnState::Disconnected
            }
        } else {
            VpnState::Disconnected
        }
    }

    async fn stats(&self) -> NetctlResult<VpnStats> {
        let mut stats = VpnStats::default();
        stats.connected_since = self.connected_since;

        // IPsec doesn't have a dedicated interface, stats are tracked differently
        // We could parse 'ipsec statusall' output for detailed statistics
        if let Some(conn_name) = &self.connection_name {
            if let Ok(output) = Command::new("ipsec")
                .args(&["statusall", conn_name])
                .output()
                .await
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Parse bytes in/out from output
                // Format is typically: "bytes_i (XXX bytes_o), rekeying in"
                for line in stdout.lines() {
                    if line.contains("bytes_i") {
                        // This is a simplified parser; actual parsing would be more complex
                        debug!("IPsec stats line: {}", line);
                    }
                }
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

        let mut status = json!({
            "backend": "ipsec",
            "state": format!("{:?}", state),
            "connection_name": self.connection_name,
            "connected_since": stats.connected_since.map(|t| format!("{:?}", t)),
            "bytes_sent": stats.bytes_sent,
            "bytes_received": stats.bytes_received,
        });

        // Get detailed status from ipsec status command
        if let Some(conn_name) = &self.connection_name {
            if let Ok(output) = Command::new("ipsec")
                .args(&["status", conn_name])
                .output()
                .await
            {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    status["details"] = json!(stdout);
                }
            }
        }

        Ok(status)
    }

    async fn import_config(&self, path: &Path) -> NetctlResult<HashMap<String, Value>> {
        info!("Importing IPsec configuration from: {:?}", path);
        Self::parse_ipsec_conf(path).await
    }

    async fn export_config(&self, config: &ConnectionConfig, path: &Path) -> NetctlResult<()> {
        info!("Exporting IPsec configuration to: {:?}", path);
        let conf_content = self.build_ipsec_conf(config)?;
        common::write_secure_config(path, &conf_content, 0o644).await
    }
}

/// Factory function to create an IPsec backend
pub fn create_backend() -> Box<dyn VpnBackend> {
    Box::new(IPsecBackend::new())
}
