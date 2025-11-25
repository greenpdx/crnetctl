//! CRIPConfig - IP configuration (libnm NMIPConfig equivalent)

use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};

/// IP configuration (equivalent to NMIPConfig)
///
/// Represents IPv4 or IPv6 configuration for a device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRIPConfig {
    /// Whether this is IPv6 (false = IPv4)
    is_ipv6: bool,
    /// Interface name
    interface: String,
    /// IP addresses
    addresses: Vec<CRIPAddress>,
    /// Gateway
    gateway: Option<String>,
    /// Routes
    routes: Vec<CRIPRoute>,
    /// Nameservers
    nameservers: Vec<String>,
    /// DNS domains
    domains: Vec<String>,
    /// DNS searches
    searches: Vec<String>,
    /// DNS options
    dns_options: Vec<String>,
    /// DNS priority
    dns_priority: i32,
    /// WINS servers (IPv4 only)
    wins_servers: Vec<String>,
}

impl CRIPConfig {
    /// Creates a new IP configuration
    pub fn new(is_ipv6: bool, interface: &str) -> Self {
        Self {
            is_ipv6,
            interface: interface.to_string(),
            addresses: Vec::new(),
            gateway: None,
            routes: Vec::new(),
            nameservers: Vec::new(),
            domains: Vec::new(),
            searches: Vec::new(),
            dns_options: Vec::new(),
            dns_priority: 0,
            wins_servers: Vec::new(),
        }
    }

    /// Creates IP config from an interface (reads current system config)
    pub(crate) fn from_interface(interface: &str, is_ipv6: bool) -> Self {
        // In a real implementation, this would read from the system
        // For now, return a basic configuration
        let mut config = Self::new(is_ipv6, interface);

        // Mock data - in reality would read from /sys/class/net/*/
        if !is_ipv6 {
            config.addresses.push(CRIPAddress {
                address: "192.168.1.100".to_string(),
                prefix: 24,
            });
            config.gateway = Some("192.168.1.1".to_string());
            config.nameservers.push("8.8.8.8".to_string());
            config.nameservers.push("8.8.4.4".to_string());
        } else {
            config.addresses.push(CRIPAddress {
                address: "fe80::1".to_string(),
                prefix: 64,
            });
        }

        config
    }

    /// Gets the interface (equivalent to nm_ip_config_get_iface)
    pub fn get_iface(&self) -> &str {
        &self.interface
    }

    /// Gets whether this is IPv6 configuration
    pub fn is_ipv6(&self) -> bool {
        self.is_ipv6
    }

    /// Gets the addresses (equivalent to nm_ip_config_get_addresses)
    pub fn get_addresses(&self) -> &[CRIPAddress] {
        &self.addresses
    }

    /// Gets the gateway (equivalent to nm_ip_config_get_gateway)
    pub fn get_gateway(&self) -> Option<&str> {
        self.gateway.as_deref()
    }

    /// Gets the routes (equivalent to nm_ip_config_get_routes)
    pub fn get_routes(&self) -> &[CRIPRoute] {
        &self.routes
    }

    /// Gets the nameservers (equivalent to nm_ip_config_get_nameservers)
    pub fn get_nameservers(&self) -> &[String] {
        &self.nameservers
    }

    /// Gets the DNS domains (equivalent to nm_ip_config_get_domains)
    pub fn get_domains(&self) -> &[String] {
        &self.domains
    }

    /// Gets the DNS searches (equivalent to nm_ip_config_get_searches)
    pub fn get_searches(&self) -> &[String] {
        &self.searches
    }

    /// Gets the DNS options (equivalent to nm_ip_config_get_dns_options)
    pub fn get_dns_options(&self) -> &[String] {
        &self.dns_options
    }

    /// Gets the DNS priority (equivalent to nm_ip_config_get_dns_priority)
    pub fn get_dns_priority(&self) -> i32 {
        self.dns_priority
    }

    /// Gets the WINS servers (IPv4 only) (equivalent to nm_ip_config_get_wins_servers)
    pub fn get_wins_servers(&self) -> &[String] {
        &self.wins_servers
    }

    /// Adds an address
    pub fn add_address(&mut self, address: CRIPAddress) {
        self.addresses.push(address);
    }

    /// Adds a route
    pub fn add_route(&mut self, route: CRIPRoute) {
        self.routes.push(route);
    }

    /// Adds a nameserver
    pub fn add_nameserver(&mut self, nameserver: String) {
        self.nameservers.push(nameserver);
    }

    /// Sets the gateway
    pub fn set_gateway(&mut self, gateway: String) {
        self.gateway = Some(gateway);
    }
}

/// IP address with prefix (equivalent to NMIPAddress)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRIPAddress {
    /// IP address as string
    pub address: String,
    /// Prefix length (CIDR notation)
    pub prefix: u32,
}

impl CRIPAddress {
    /// Creates a new IP address
    pub fn new(address: String, prefix: u32) -> Self {
        Self { address, prefix }
    }

    /// Creates from an IpAddr
    pub fn from_ip_addr(addr: IpAddr, prefix: u32) -> Self {
        Self {
            address: addr.to_string(),
            prefix,
        }
    }

    /// Gets the address (equivalent to nm_ip_address_get_address)
    pub fn get_address(&self) -> &str {
        &self.address
    }

    /// Gets the prefix (equivalent to nm_ip_address_get_prefix)
    pub fn get_prefix(&self) -> u32 {
        self.prefix
    }

    /// Parses the address as an IpAddr
    pub fn parse(&self) -> Option<IpAddr> {
        self.address.parse().ok()
    }

    /// Checks if this is an IPv6 address
    pub fn is_ipv6(&self) -> bool {
        self.parse().map(|ip| ip.is_ipv6()).unwrap_or(false)
    }

    /// Checks if this is an IPv4 address
    pub fn is_ipv4(&self) -> bool {
        self.parse().map(|ip| ip.is_ipv4()).unwrap_or(false)
    }

    /// Gets the netmask (IPv4 only)
    pub fn get_netmask(&self) -> Option<Ipv4Addr> {
        if !self.is_ipv4() || self.prefix > 32 {
            return None;
        }

        let mask = if self.prefix == 0 {
            0u32
        } else {
            !0u32 << (32 - self.prefix)
        };

        Some(Ipv4Addr::from(mask))
    }

    /// Formats as CIDR notation (e.g., "192.168.1.1/24")
    pub fn to_cidr(&self) -> String {
        format!("{}/{}", self.address, self.prefix)
    }
}

/// IP route (equivalent to NMIPRoute)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CRIPRoute {
    /// Destination network
    pub dest: String,
    /// Prefix length
    pub prefix: u32,
    /// Next hop (gateway)
    pub next_hop: Option<String>,
    /// Metric
    pub metric: i64,
}

impl CRIPRoute {
    /// Creates a new route
    pub fn new(dest: String, prefix: u32) -> Self {
        Self {
            dest,
            prefix,
            next_hop: None,
            metric: -1,
        }
    }

    /// Gets the destination (equivalent to nm_ip_route_get_dest)
    pub fn get_dest(&self) -> &str {
        &self.dest
    }

    /// Gets the prefix (equivalent to nm_ip_route_get_prefix)
    pub fn get_prefix(&self) -> u32 {
        self.prefix
    }

    /// Gets the next hop (equivalent to nm_ip_route_get_next_hop)
    pub fn get_next_hop(&self) -> Option<&str> {
        self.next_hop.as_deref()
    }

    /// Gets the metric (equivalent to nm_ip_route_get_metric)
    pub fn get_metric(&self) -> i64 {
        self.metric
    }

    /// Sets the next hop (equivalent to nm_ip_route_set_next_hop)
    pub fn set_next_hop(&mut self, next_hop: String) {
        self.next_hop = Some(next_hop);
    }

    /// Sets the metric (equivalent to nm_ip_route_set_metric)
    pub fn set_metric(&mut self, metric: i64) {
        self.metric = metric;
    }

    /// Checks if this is a default route
    pub fn is_default(&self) -> bool {
        (self.dest == "0.0.0.0" || self.dest == "::") && self.prefix == 0
    }

    /// Formats as string (e.g., "192.168.1.0/24 via 192.168.1.1")
    pub fn to_string_format(&self) -> String {
        let mut s = format!("{}/{}", self.dest, self.prefix);
        if let Some(ref nh) = self.next_hop {
            s.push_str(&format!(" via {}", nh));
        }
        if self.metric >= 0 {
            s.push_str(&format!(" metric {}", self.metric));
        }
        s
    }
}
