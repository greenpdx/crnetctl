//! DNS server management via unbound

use crate::error::NetctlResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    pub interface: Option<String>,
    pub forwarders: Vec<String>,
    pub dnssec: bool,
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            interface: None,
            forwarders: vec!["1.1.1.1".to_string(), "8.8.8.8".to_string()],
            dnssec: true,
        }
    }
}

pub struct DnsController;

impl DnsController {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DnsController {
    fn default() -> Self {
        Self::new()
    }
}
