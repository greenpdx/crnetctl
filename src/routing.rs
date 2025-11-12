//! Routing table management

use crate::error::{NetctlError, NetctlResult};
use std::process::Command;

pub struct RoutingController;

impl RoutingController {
    pub fn new() -> Self {
        Self
    }

    pub fn add_default_gateway(&self, gateway: &str, interface: Option<&str>) -> NetctlResult<()> {
        let mut args = vec!["route", "add", "default", "via", gateway];
        if let Some(iface) = interface {
            args.extend_from_slice(&["dev", iface]);
        }

        let output = Command::new("ip").args(&args).output()?;

        if !output.status.success() {
            return Err(NetctlError::CommandFailed {
                cmd: format!("ip {}", args.join(" ")),
                code: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        Ok(())
    }
}

impl Default for RoutingController {
    fn default() -> Self {
        Self::new()
    }
}
