//! netctl - Network Control CLI Tool
//!
//! Standalone network management tool similar to nmcli
//! Can operate independently or alongside crrouterd

use clap::{Parser, Subcommand};
use netctl::*;
use netctl::validation;
use serde_json;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "netctl")]
#[command(about = "Network control tool - manage interfaces, WiFi, AP, DHCP, DNS", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format: text, json
    #[arg(short = 'o', long, default_value = "text")]
    output: String,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Device management (list interfaces and devices)
    #[command(subcommand)]
    Device(DeviceCommands),

    /// Interface control (up/down, IP configuration)
    #[command(subcommand)]
    Interface(InterfaceCommands),

    /// WiFi operations (scan, connect, AP mode)
    #[command(subcommand)]
    Wifi(WifiCommands),

    /// Access Point management
    #[command(subcommand)]
    Ap(ApCommands),

    /// DHCP server management
    #[command(subcommand)]
    Dhcp(DhcpCommands),

    /// DNS server management
    #[command(subcommand)]
    Dns(DnsCommands),

    /// Routing management
    #[command(subcommand)]
    Route(RouteCommands),

    /// VPN management (WireGuard, OpenVPN, IPsec)
    #[command(subcommand)]
    Vpn(VpnCommands),

    /// Network monitoring
    #[command(subcommand)]
    Monitor(MonitorCommands),

    /// Debug and diagnostics
    #[command(subcommand)]
    Debug(DebugCommands),
}

#[derive(Subcommand)]
enum DeviceCommands {
    /// List all network devices
    List,
    /// Show device details
    Show { interface: String },
    /// Get device status
    Status { interface: String },
}

#[derive(Subcommand)]
enum InterfaceCommands {
    /// List all interfaces
    List,
    /// Show interface details
    Show { interface: String },
    /// Bring interface up
    Up { interface: String },
    /// Bring interface down
    Down { interface: String },
    /// Set IP address
    SetIp {
        interface: String,
        address: String,
        #[arg(short, long, default_value = "24")]
        prefix: u8,
    },
    /// Add IP address
    AddIp {
        interface: String,
        address: String,
        #[arg(short, long, default_value = "24")]
        prefix: u8,
    },
    /// Delete IP address
    DelIp {
        interface: String,
        address: String,
        #[arg(short, long, default_value = "24")]
        prefix: u8,
    },
    /// Flush all IP addresses
    FlushIp { interface: String },
    /// Set MAC address
    SetMac { interface: String, mac: String },
    /// Set MTU
    SetMtu { interface: String, mtu: u32 },
    /// Rename interface
    Rename { old_name: String, new_name: String },
}

#[derive(Subcommand)]
enum WifiCommands {
    /// List WiFi interfaces
    List,
    /// Scan for networks
    Scan { interface: String },
    /// Show WiFi device info
    Info { interface: String },
    /// Get regulatory domain
    GetReg,
    /// Set regulatory domain
    SetReg { country: String },
    /// Get TX power
    GetTxpower { interface: String },
    /// Set TX power
    SetTxpower { interface: String, power: String },
}

#[derive(Subcommand)]
enum ApCommands {
    /// Start Access Point
    Start {
        interface: String,
        #[arg(short, long)]
        ssid: String,
        #[arg(short, long)]
        password: Option<String>,
        #[arg(short, long, default_value = "6")]
        channel: u8,
        #[arg(short, long, default_value = "2.4GHz")]
        band: String,
        #[arg(long, default_value = "US")]
        country: String,
        #[arg(long, default_value = "10.255.24.1/24")]
        ip: String,
    },
    /// Stop Access Point
    Stop,
    /// Get AP status
    Status,
    /// Restart Access Point
    Restart,
}

#[derive(Subcommand)]
enum DhcpCommands {
    /// Start DHCP server
    Start {
        interface: String,
        #[arg(long)]
        range_start: String,
        #[arg(long)]
        range_end: String,
        #[arg(long)]
        gateway: String,
        #[arg(long)]
        dns: Vec<String>,
    },
    /// Stop DHCP server
    Stop,
    /// Get DHCP status
    Status,
    /// Show active leases
    Leases,
}

#[derive(Subcommand)]
enum DnsCommands {
    /// Start DNS server
    Start {
        #[arg(long)]
        forwarders: Vec<String>,
    },
    /// Stop DNS server
    Stop,
    /// Get DNS status
    Status,
    /// Flush DNS cache
    Flush,
}

#[derive(Subcommand)]
enum RouteCommands {
    /// Show routing table
    Show,
    /// Add default gateway
    AddDefault {
        gateway: String,
        #[arg(short, long)]
        interface: Option<String>,
    },
    /// Delete default gateway
    DelDefault,
}

#[derive(Subcommand)]
enum VpnCommands {
    /// List all VPN connections
    List,
    /// Show VPN connection details
    Show { name: String },
    /// Connect to a VPN
    Connect { name: String },
    /// Disconnect from a VPN
    Disconnect { name: String },
    /// Import VPN configuration file
    Import {
        /// VPN type: wireguard, openvpn, ipsec
        #[arg(short, long)]
        vpn_type: String,
        /// Configuration file path
        config_file: PathBuf,
        /// Connection name
        #[arg(short, long)]
        name: String,
    },
    /// Export VPN configuration
    Export {
        /// Connection name
        name: String,
        /// Output file path
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Create a new VPN connection from a config file
    Create {
        /// Configuration file in TOML format
        config_file: PathBuf,
    },
    /// Delete a VPN connection
    Delete { name: String },
    /// Get VPN connection status
    Status { name: String },
    /// Get VPN connection statistics
    Stats { name: String },
    /// List available VPN backends
    Backends,
}

#[derive(Subcommand)]
enum MonitorCommands {
    /// Show bandwidth usage
    Bandwidth { interface: String },
    /// Show interface statistics
    Stats { interface: String },
}

#[derive(Subcommand)]
enum DebugCommands {
    /// Ping a host
    Ping {
        host: String,
        #[arg(short, long, default_value = "4")]
        count: u32,
    },
    /// Start packet capture
    Tcpdump {
        interface: String,
        #[arg(short, long)]
        filter: Option<String>,
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Device(ref cmd) => handle_device(cmd, &cli).await,
        Commands::Interface(ref cmd) => handle_interface(cmd, &cli).await,
        Commands::Wifi(ref cmd) => handle_wifi(cmd, &cli).await,
        Commands::Ap(ref cmd) => handle_ap(cmd, &cli).await,
        Commands::Dhcp(ref cmd) => handle_dhcp(cmd, &cli).await,
        Commands::Dns(ref cmd) => handle_dns(cmd, &cli).await,
        Commands::Route(ref cmd) => handle_route(cmd, &cli).await,
        Commands::Vpn(ref cmd) => handle_vpn(cmd, &cli).await,
        Commands::Monitor(ref cmd) => handle_monitor(cmd, &cli).await,
        Commands::Debug(ref cmd) => handle_debug(cmd, &cli).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

async fn handle_device(cmd: &DeviceCommands, cli: &Cli) -> NetctlResult<()> {
    let iface_ctrl = interface::InterfaceController::new();

    match cmd {
        DeviceCommands::List => {
            let interfaces = iface_ctrl.list().await?;
            if cli.output == "json" {
                println!("{}", serde_json::to_string_pretty(&interfaces)
                    .map_err(|e| NetctlError::ParseError(format!("JSON serialization error: {}", e)))?);
            } else {
                println!("DEVICE");
                for iface in interfaces {
                    println!("{}", iface);
                }
            }
        }
        DeviceCommands::Show { interface } | DeviceCommands::Status { interface } => {
            let info = iface_ctrl.get_info(&interface).await?;
            if cli.output == "json" {
                println!("{}", serde_json::to_string_pretty(&info)
                    .map_err(|e| NetctlError::ParseError(format!("JSON serialization error: {}", e)))?);
            } else {
                println!("Device: {}", info.name);
                if let Some(mac) = &info.mac_address {
                    println!("  MAC: {}", mac);
                }
                if let Some(mtu) = info.mtu {
                    println!("  MTU: {}", mtu);
                }
                if let Some(state) = &info.state {
                    println!("  State: {}", state);
                }
                if !info.addresses.is_empty() {
                    println!("  Addresses:");
                    for addr in &info.addresses {
                        println!("    {}/{} ({})", addr.address, addr.prefix_len, addr.family);
                    }
                }
            }
        }
    }
    Ok(())
}

async fn handle_interface(cmd: &InterfaceCommands, cli: &Cli) -> NetctlResult<()> {
    let iface_ctrl = interface::InterfaceController::new();

    match cmd {
        InterfaceCommands::List => {
            let interfaces = iface_ctrl.list().await?;
            if cli.output == "json" {
                println!("{}", serde_json::to_string_pretty(&interfaces)
                    .map_err(|e| NetctlError::ParseError(format!("JSON serialization error: {}", e)))?);
            } else {
                println!("INTERFACE");
                for iface in interfaces {
                    if let Ok(info) = iface_ctrl.get_info(&iface).await {
                        let state = info.state.unwrap_or_else(|| "unknown".to_string());
                        println!("{:15} {}", iface, state);
                    }
                }
            }
        }
        InterfaceCommands::Show { interface } => {
            let info = iface_ctrl.get_info(&interface).await?;
            if cli.output == "json" {
                println!("{}", serde_json::to_string_pretty(&info)
                    .map_err(|e| NetctlError::ParseError(format!("JSON serialization error: {}", e)))?);
            } else {
                println!("{}: <{}>", info.name, info.flags.join(","));
                if let Some(mac) = &info.mac_address {
                    println!("    link/ether {}", mac);
                }
                for addr in &info.addresses {
                    println!("    {}/{}", addr.address, addr.prefix_len);
                }
                if let Some(stats) = &info.stats {
                    println!("    RX: {} bytes, {} packets", stats.rx_bytes, stats.rx_packets);
                    println!("    TX: {} bytes, {} packets", stats.tx_bytes, stats.tx_packets);
                }
            }
        }
        InterfaceCommands::Up { interface } => {
            iface_ctrl.up(&interface).await?;
            println!("Interface {} is up", interface);
        }
        InterfaceCommands::Down { interface } => {
            iface_ctrl.down(&interface).await?;
            println!("Interface {} is down", interface);
        }
        InterfaceCommands::SetIp { interface, address, prefix } => {
            iface_ctrl.set_ip(&interface, &address, *prefix).await?;
            println!("Set {}/{} on {}", address, prefix, interface);
        }
        InterfaceCommands::AddIp { interface, address, prefix } => {
            iface_ctrl.add_ip(&interface, &address, *prefix).await?;
            println!("Added {}/{} to {}", address, prefix, interface);
        }
        InterfaceCommands::DelIp { interface, address, prefix } => {
            iface_ctrl.del_ip(&interface, &address, *prefix).await?;
            println!("Deleted {}/{} from {}", address, prefix, interface);
        }
        InterfaceCommands::FlushIp { interface } => {
            iface_ctrl.flush_addrs(&interface).await?;
            println!("Flushed all addresses from {}", interface);
        }
        InterfaceCommands::SetMac { interface, mac } => {
            iface_ctrl.set_mac(&interface, &mac).await?;
            println!("Set MAC {} on {}", mac, interface);
        }
        InterfaceCommands::SetMtu { interface, mtu } => {
            iface_ctrl.set_mtu(&interface, *mtu).await?;
            println!("Set MTU {} on {}", mtu, interface);
        }
        InterfaceCommands::Rename { old_name, new_name } => {
            iface_ctrl.rename(&old_name, &new_name).await?;
            println!("Renamed {} to {}", old_name, new_name);
        }
    }
    Ok(())
}

async fn handle_wifi(cmd: &WifiCommands, cli: &Cli) -> NetctlResult<()> {
    let wifi_ctrl = wifi::WifiController::new();

    match cmd {
        WifiCommands::List => {
            let iface_ctrl = interface::InterfaceController::new();
            let interfaces = iface_ctrl.list().await?;
            let wifi_ifaces: Vec<_> = interfaces.into_iter()
                .filter(|i| i.starts_with("wlan") || i.starts_with("wlp"))
                .collect();

            if cli.output == "json" {
                println!("{}", serde_json::to_string_pretty(&wifi_ifaces)
                    .map_err(|e| NetctlError::ParseError(format!("JSON serialization error: {}", e)))?);
            } else {
                println!("WIFI INTERFACE");
                for iface in wifi_ifaces {
                    println!("{}", iface);
                }
            }
        }
        WifiCommands::Scan { interface } => {
            println!("Scanning on {}...", interface);
            let results = wifi_ctrl.scan(&interface).await?;
            if cli.output == "json" {
                println!("{}", serde_json::to_string_pretty(&results)
                    .map_err(|e| NetctlError::ParseError(format!("JSON serialization error: {}", e)))?);
            } else {
                println!("SSID                             BSSID              FREQ    SIGNAL");
                for result in results {
                    let ssid = result.ssid.unwrap_or_else(|| "".to_string());
                    let freq = result.frequency.map(|f| f.to_string()).unwrap_or_else(|| "".to_string());
                    let signal = result.signal.unwrap_or_else(|| "".to_string());
                    println!("{:32} {:17} {:7} {}", ssid, result.bssid, freq, signal);
                }
            }
        }
        WifiCommands::Info { interface } => {
            let info = wifi_ctrl.get_dev_info(&interface).await?;
            if cli.output == "json" {
                println!("{}", serde_json::to_string_pretty(&info)
                    .map_err(|e| NetctlError::ParseError(format!("JSON serialization error: {}", e)))?);
            } else {
                println!("Interface: {}", info.interface);
                if let Some(phy) = &info.phy {
                    println!("  PHY: {}", phy);
                }
                if let Some(ch) = info.channel {
                    println!("  Channel: {}", ch);
                }
                if let Some(freq) = info.frequency {
                    println!("  Frequency: {} MHz", freq);
                }
                if let Some(pwr) = &info.txpower {
                    println!("  TX Power: {}", pwr);
                }
            }
        }
        WifiCommands::GetReg => {
            let reg = wifi_ctrl.get_reg_domain().await?;
            if cli.output == "json" {
                println!("{}", serde_json::to_string_pretty(&reg)
                    .map_err(|e| NetctlError::ParseError(format!("JSON serialization error: {}", e)))?);
            } else {
                if let Some(country) = &reg.country {
                    println!("Country: {}", country);
                }
                if let Some(dfs) = &reg.dfs_region {
                    println!("DFS Region: {}", dfs);
                }
            }
        }
        WifiCommands::SetReg { country } => {
            wifi_ctrl.set_reg_domain(&country).await?;
            println!("Set regulatory domain to {}", country);
        }
        WifiCommands::GetTxpower { interface } => {
            let power = wifi_ctrl.get_txpower(&interface).await?;
            println!("{}", power);
        }
        WifiCommands::SetTxpower { interface, power } => {
            wifi_ctrl.set_txpower(&interface, &power).await?;
            println!("Set TX power to {} on {}", power, interface);
        }
    }
    Ok(())
}

async fn handle_ap(cmd: &ApCommands, _cli: &Cli) -> NetctlResult<()> {
    let config_dir = PathBuf::from("/run/crrouter/netctl");
    let hostapd_ctrl = hostapd::HostapdController::new(config_dir);

    match cmd {
        ApCommands::Start { interface, ssid, password, channel, band, country, ip } => {
            // Parse IP address and prefix
            let parts: Vec<&str> = ip.split('/').collect();
            if parts.len() != 2 {
                return Err(NetctlError::InvalidParameter(
                    "IP must be in format: address/prefix (e.g., 10.255.24.1/24)".to_string()
                ));
            }
            let ip_addr = parts[0];
            let prefix: u8 = parts[1].parse()
                .map_err(|_| NetctlError::InvalidParameter("Invalid prefix length".to_string()))?;

            // Set up interface before starting AP
            let iface_ctrl = interface::InterfaceController::new();

            // Bring interface up
            iface_ctrl.up(&interface).await?;

            // Flush existing IPs and add new one
            iface_ctrl.flush_addrs(&interface).await?;
            iface_ctrl.add_ip(&interface, ip_addr, prefix).await?;

            println!("Interface {} configured with IP {}", interface, ip);

            let config = hostapd::AccessPointConfig {
                interface: interface.clone(),
                ssid: ssid.clone(),
                password: password.clone(),
                channel: *channel,
                band: band.clone(),
                country_code: country.clone(),
                ..Default::default()
            };

            hostapd_ctrl.start(&config).await?;
            println!("Access Point started");
        }
        ApCommands::Stop => {
            hostapd_ctrl.stop().await?;
            println!("Access Point stopped");
        }
        ApCommands::Status => {
            let running = hostapd_ctrl.is_running().await?;
            println!("Access Point: {}", if running { "running" } else { "stopped" });
        }
        ApCommands::Restart => {
            println!("Restarting Access Point...");
            // Would need to read existing config
            println!("Not implemented - use stop then start");
        }
    }
    Ok(())
}

async fn handle_dhcp(cmd: &DhcpCommands, _cli: &Cli) -> NetctlResult<()> {
    let config_path = PathBuf::from("/run/crrouter/netctl/dora.yaml");
    let dhcp_ctrl = dhcp::DhcpController::new(config_path);

    match cmd {
        DhcpCommands::Start { interface, range_start, range_end, gateway, dns } => {
            let config = dhcp::DhcpConfig {
                interface: interface.clone(),
                range_start: range_start.clone(),
                range_end: range_end.clone(),
                gateway: gateway.clone(),
                dns_servers: dns.clone(),
                ..Default::default()
            };

            dhcp_ctrl.write_config(&config).await?;
            println!("DHCP server configuration written");
            println!("Note: Start dora manually: sudo /usr/local/bin/dora -c /run/crrouter/netctl/dora.yaml");
        }
        DhcpCommands::Stop | DhcpCommands::Status | DhcpCommands::Leases => {
            println!("Not fully implemented yet");
        }
    }
    Ok(())
}

async fn handle_dns(_cmd: &DnsCommands, _cli: &Cli) -> NetctlResult<()> {
    println!("DNS commands not fully implemented yet");
    Ok(())
}

async fn handle_route(cmd: &RouteCommands, _cli: &Cli) -> NetctlResult<()> {
    let route_ctrl = routing::RoutingController::new();

    match cmd {
        RouteCommands::Show => {
            println!("Route table:");
            let output = tokio::process::Command::new("ip")
                .args(["route", "show"])
                .output()
                .await
                .map_err(|e| NetctlError::CommandFailed {
                    cmd: "ip route show".to_string(),
                    code: None,
                    stderr: e.to_string(),
                })?;
            let stdout = String::from_utf8(output.stdout)
                .unwrap_or_else(|e| String::from_utf8_lossy(&e.into_bytes()).to_string());
            println!("{}", stdout);
        }
        RouteCommands::AddDefault { gateway, interface } => {
            route_ctrl.add_default_gateway(&gateway, interface.as_deref()).await?;
            println!("Added default gateway {}", gateway);
        }
        RouteCommands::DelDefault => {
            println!("Not implemented yet");
        }
    }
    Ok(())
}

async fn handle_monitor(cmd: &MonitorCommands, cli: &Cli) -> NetctlResult<()> {
    let iface_ctrl = interface::InterfaceController::new();

    match cmd {
        MonitorCommands::Bandwidth { interface } | MonitorCommands::Stats { interface } => {
            let info = iface_ctrl.get_info(&interface).await?;
            if let Some(stats) = &info.stats {
                if cli.output == "json" {
                    println!("{}", serde_json::to_string_pretty(&stats)
                        .map_err(|e| NetctlError::ParseError(format!("JSON serialization error: {}", e)))?);
                } else {
                    println!("Statistics for {}:", interface);
                    println!("  RX: {} bytes, {} packets, {} errors, {} dropped",
                             stats.rx_bytes, stats.rx_packets, stats.rx_errors, stats.rx_dropped);
                    println!("  TX: {} bytes, {} packets, {} errors, {} dropped",
                             stats.tx_bytes, stats.tx_packets, stats.tx_errors, stats.tx_dropped);
                }
            } else {
                println!("No statistics available for {}", interface);
            }
        }
    }
    Ok(())
}

async fn handle_debug(cmd: &DebugCommands, _cli: &Cli) -> NetctlResult<()> {
    match cmd {
        DebugCommands::Ping { host, count } => {
            // Validate hostname to prevent command injection
            validation::validate_hostname(host)?;

            println!("Pinging {} {} times...", host, count);
            let count_str = count.to_string();
            let output = tokio::process::Command::new("ping")
                .args(["-c", &count_str, host])
                .output()
                .await
                .map_err(|e| NetctlError::CommandFailed {
                    cmd: format!("ping -c {} {}", count, host),
                    code: None,
                    stderr: e.to_string(),
                })?;
            let stdout = String::from_utf8(output.stdout)
                .unwrap_or_else(|e| String::from_utf8_lossy(&e.into_bytes()).to_string());
            println!("{}", stdout);
        }
        DebugCommands::Tcpdump { interface, filter, output } => {
            // Validate interface name to prevent command injection
            validation::validate_interface_name(interface)?;

            println!("Starting packet capture on {}...", interface);
            let mut args = vec!["-i", interface.as_str()];
            if let Some(ref f) = filter {
                args.push(f.as_str());
            }
            if let Some(ref o) = output {
                args.extend_from_slice(&["-w", o.as_str()]);
            }

            let cmd_str = format!("tcpdump {}", args.join(" "));
            let status = tokio::process::Command::new("tcpdump")
                .args(&args)
                .status()
                .await
                .map_err(|e| NetctlError::CommandFailed {
                    cmd: cmd_str.clone(),
                    code: None,
                    stderr: e.to_string(),
                })?;

            if !status.success() {
                return Err(NetctlError::CommandFailed {
                    cmd: cmd_str,
                    code: status.code(),
                    stderr: "tcpdump failed".to_string(),
                });
            }
        }
    }
    Ok(())
}

async fn handle_vpn(cmd: &VpnCommands, cli: &Cli) -> NetctlResult<()> {
    use netctl::vpn::{VpnManager, wireguard, openvpn, ipsec};
    #[cfg(feature = "vpn-tor")]
    use netctl::vpn::arti;

    // Initialize VPN manager with backends
    let config_dir = std::env::var("NETCTL_CONFIG_DIR")
        .unwrap_or_else(|_| "/etc/netctl".to_string());
    let mut manager = VpnManager::new(PathBuf::from(config_dir));

    // Register VPN backends
    manager.register_backend("wireguard", wireguard::create_backend);
    manager.register_backend("openvpn", openvpn::create_backend);
    manager.register_backend("ipsec", ipsec::create_backend);

    #[cfg(feature = "vpn-tor")]
    manager.register_backend("arti", arti::create_backend);

    match cmd {
        VpnCommands::List => {
            let connections = manager.list_connections().await;
            if cli.output == "json" {
                println!("{}", serde_json::to_string_pretty(&connections)?);
            } else {
                if connections.is_empty() {
                    println!("No VPN connections configured");
                } else {
                    println!("VPN Connections:");
                    for uuid in &connections {
                        if let Ok(config) = manager.get_config(uuid).await {
                            let state = manager.get_state(uuid).await.unwrap_or(netctl::vpn::VpnState::Disconnected);
                            println!("  {} - {} ({:?})", config.name, uuid, state);
                        }
                    }
                }
            }
        }

        VpnCommands::Show { name } => {
            // Find connection by name
            let connections = manager.list_connections().await;
            let mut found = None;
            for uuid in &connections {
                if let Ok(config) = manager.get_config(uuid).await {
                    if config.name == *name {
                        found = Some(uuid.clone());
                        break;
                    }
                }
            }

            if let Some(uuid) = found {
                let config = manager.get_config(&uuid).await?;
                let state = manager.get_state(&uuid).await?;
                let status = manager.get_status(&uuid).await?;

                if cli.output == "json" {
                    let output = serde_json::json!({
                        "config": config,
                        "state": format!("{:?}", state),
                        "status": status,
                    });
                    println!("{}", serde_json::to_string_pretty(&output)?);
                } else {
                    println!("VPN Connection: {}", config.name);
                    println!("  UUID: {}", config.uuid);
                    println!("  Type: {}", config.conn_type);
                    println!("  State: {:?}", state);
                    println!("  Auto-connect: {}", config.autoconnect);
                    println!("\nConfiguration:");
                    for (key, value) in &config.settings {
                        if !key.contains("key") && !key.contains("password") && !key.contains("psk") {
                            println!("    {}: {}", key, value);
                        } else {
                            println!("    {}: ********", key);
                        }
                    }
                    println!("\nStatus:");
                    println!("{}", serde_json::to_string_pretty(&status)?);
                }
            } else {
                return Err(NetctlError::NotFound(format!("VPN connection '{}' not found", name)));
            }
        }

        VpnCommands::Connect { name } => {
            // Find connection by name
            let connections = manager.list_connections().await;
            let mut found = None;
            for uuid in &connections {
                if let Ok(config) = manager.get_config(uuid).await {
                    if config.name == *name {
                        found = Some(uuid.clone());
                        break;
                    }
                }
            }

            if let Some(uuid) = found {
                println!("Connecting to VPN: {}", name);
                let interface = manager.connect(&uuid).await?;
                println!("Connected! Interface: {}", interface);
            } else {
                return Err(NetctlError::NotFound(format!("VPN connection '{}' not found", name)));
            }
        }

        VpnCommands::Disconnect { name } => {
            // Find connection by name
            let connections = manager.list_connections().await;
            let mut found = None;
            for uuid in &connections {
                if let Ok(config) = manager.get_config(uuid).await {
                    if config.name == *name {
                        found = Some(uuid.clone());
                        break;
                    }
                }
            }

            if let Some(uuid) = found {
                println!("Disconnecting VPN: {}", name);
                manager.disconnect(&uuid).await?;
                println!("Disconnected!");
            } else {
                return Err(NetctlError::NotFound(format!("VPN connection '{}' not found", name)));
            }
        }

        VpnCommands::Import { vpn_type, config_file, name } => {
            println!("Importing {} configuration from {:?}", vpn_type, config_file);
            let uuid = manager.import_config(vpn_type, config_file, name.clone()).await?;
            println!("Imported successfully! Connection UUID: {}", uuid);
        }

        VpnCommands::Export { name, output } => {
            // Find connection by name
            let connections = manager.list_connections().await;
            let mut found = None;
            for uuid in &connections {
                if let Ok(config) = manager.get_config(uuid).await {
                    if config.name == *name {
                        found = Some(uuid.clone());
                        break;
                    }
                }
            }

            if let Some(uuid) = found {
                println!("Exporting VPN configuration to {:?}", output);
                manager.export_config(&uuid, output).await?;
                println!("Exported successfully!");
            } else {
                return Err(NetctlError::NotFound(format!("VPN connection '{}' not found", name)));
            }
        }

        VpnCommands::Create { config_file } => {
            println!("Creating VPN connection from {:?}", config_file);
            let content = tokio::fs::read_to_string(config_file).await
                .map_err(|e| NetctlError::ServiceError(e.to_string()))?;
            let config: ConnectionConfig = toml::from_str(&content)
                .map_err(|e| NetctlError::InvalidParameter(format!("Invalid TOML: {}", e)))?;

            let uuid = manager.create_connection(config.clone()).await?;
            println!("Created VPN connection: {} ({})", config.name, uuid);
        }

        VpnCommands::Delete { name } => {
            // Find connection by name
            let connections = manager.list_connections().await;
            let mut found = None;
            for uuid in &connections {
                if let Ok(config) = manager.get_config(uuid).await {
                    if config.name == *name {
                        found = Some(uuid.clone());
                        break;
                    }
                }
            }

            if let Some(uuid) = found {
                println!("Deleting VPN connection: {}", name);
                manager.delete_connection(&uuid).await?;
                println!("Deleted!");
            } else {
                return Err(NetctlError::NotFound(format!("VPN connection '{}' not found", name)));
            }
        }

        VpnCommands::Status { name } => {
            // Find connection by name
            let connections = manager.list_connections().await;
            let mut found = None;
            for uuid in &connections {
                if let Ok(config) = manager.get_config(uuid).await {
                    if config.name == *name {
                        found = Some(uuid.clone());
                        break;
                    }
                }
            }

            if let Some(uuid) = found {
                let state = manager.get_state(&uuid).await?;
                let status = manager.get_status(&uuid).await?;

                if cli.output == "json" {
                    let output = serde_json::json!({
                        "name": name,
                        "uuid": uuid,
                        "state": format!("{:?}", state),
                        "status": status,
                    });
                    println!("{}", serde_json::to_string_pretty(&output)?);
                } else {
                    println!("VPN Connection: {}", name);
                    println!("State: {:?}", state);
                    println!("\nStatus:");
                    println!("{}", serde_json::to_string_pretty(&status)?);
                }
            } else {
                return Err(NetctlError::NotFound(format!("VPN connection '{}' not found", name)));
            }
        }

        VpnCommands::Stats { name } => {
            // Find connection by name
            let connections = manager.list_connections().await;
            let mut found = None;
            for uuid in &connections {
                if let Ok(config) = manager.get_config(uuid).await {
                    if config.name == *name {
                        found = Some(uuid.clone());
                        break;
                    }
                }
            }

            if let Some(uuid) = found {
                let stats = manager.get_stats(&uuid).await?;

                if cli.output == "json" {
                    println!("{}", serde_json::to_string_pretty(&stats)?);
                } else {
                    println!("VPN Statistics: {}", name);
                    println!("  Bytes sent: {}", stats.bytes_sent);
                    println!("  Bytes received: {}", stats.bytes_received);
                    println!("  Packets sent: {}", stats.packets_sent);
                    println!("  Packets received: {}", stats.packets_received);
                    if let Some(connected_since) = stats.connected_since {
                        println!("  Connected since: {:?}", connected_since);
                    }
                    if let Some(last_handshake) = stats.last_handshake {
                        println!("  Last handshake: {:?}", last_handshake);
                    }
                    if let Some(peer_endpoint) = stats.peer_endpoint {
                        println!("  Peer endpoint: {}", peer_endpoint);
                    }
                }
            } else {
                return Err(NetctlError::NotFound(format!("VPN connection '{}' not found", name)));
            }
        }

        VpnCommands::Backends => {
            let backends = manager.available_backends();
            if cli.output == "json" {
                println!("{}", serde_json::to_string_pretty(&backends)?);
            } else {
                println!("Available VPN Backends:");
                for backend in &backends {
                    println!("  - {}", backend);
                }
            }
        }
    }

    Ok(())
}
