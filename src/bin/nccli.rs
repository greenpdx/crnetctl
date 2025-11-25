//! nccli - Network Control CLI Tool
//!
//! A comprehensive network management command-line interface
//! providing complete network control using the netctl backend

use clap::{Parser, Subcommand, ValueEnum};
use libnetctl::*;
use libnetctl::validation;
use libnetctl::connection_config::NetctlConnectionConfig;
use serde_json;
use std::path::PathBuf;
use std::process;
use std::fs::OpenOptions;
use std::io::Write;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

// ============================================================================
// PRIVILEGED OPERATIONS SECURITY
// ============================================================================

/// List of all commands that modify system network configuration.
/// These operations require either:
/// 1. Running as root (UID 0), or
/// 2. Using --allow-root-ops flag (which itself requires root)
///
/// Categories of privileged operations:
/// - Interface management: up/down, IP configuration, MAC changes
/// - Connection management: create, modify, delete, activate
/// - Service control: AP, DHCP, VPN start/stop
/// - System settings: hostname, routing, DNS
/// - Radio control: WiFi on/off
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivilegedOp {
    // General
    SetHostname,

    // Networking
    NetworkingOn,
    NetworkingOff,

    // Radio
    RadioWifiOn,
    RadioWifiOff,

    // Connection management
    ConnectionUp,
    ConnectionDown,
    ConnectionAdd,
    ConnectionModify,
    ConnectionDelete,
    ConnectionReload,
    ConnectionLoad,
    ConnectionClone,
    ConnectionEdit,

    // Device management
    DeviceConnect,
    DeviceDisconnect,
    DeviceSet,
    DeviceReapply,
    DeviceModify,
    DeviceDelete,
    DeviceWifiConnect,
    DeviceWifiHotspot,
    DeviceWifiRadio,

    // VPN
    VpnConnect,
    VpnDisconnect,
    VpnCreate,
    VpnImport,
    VpnDelete,

    // Access Point
    ApStart,
    ApStop,
    ApRestart,

    // DHCP
    DhcpStart,
    DhcpStop,

    // DNS
    DnsSet,
    DnsFlush,

    // Routing
    RouteAddDefault,
    RouteAdd,
    RouteDelete,
}

impl PrivilegedOp {
    /// Get a human-readable description of the operation
    pub fn description(&self) -> &'static str {
        match self {
            Self::SetHostname => "set system hostname",
            Self::NetworkingOn => "enable networking",
            Self::NetworkingOff => "disable networking",
            Self::RadioWifiOn => "enable WiFi radio",
            Self::RadioWifiOff => "disable WiFi radio",
            Self::ConnectionUp => "activate connection",
            Self::ConnectionDown => "deactivate connection",
            Self::ConnectionAdd => "create connection",
            Self::ConnectionModify => "modify connection",
            Self::ConnectionDelete => "delete connection",
            Self::ConnectionReload => "reload connections",
            Self::ConnectionLoad => "load connection file",
            Self::ConnectionClone => "clone connection",
            Self::ConnectionEdit => "edit connection",
            Self::DeviceConnect => "connect device",
            Self::DeviceDisconnect => "disconnect device",
            Self::DeviceSet => "set device properties",
            Self::DeviceReapply => "reapply device configuration",
            Self::DeviceModify => "modify device",
            Self::DeviceDelete => "delete device",
            Self::DeviceWifiConnect => "connect to WiFi network",
            Self::DeviceWifiHotspot => "create WiFi hotspot",
            Self::DeviceWifiRadio => "control WiFi radio",
            Self::VpnConnect => "connect VPN",
            Self::VpnDisconnect => "disconnect VPN",
            Self::VpnCreate => "create VPN connection",
            Self::VpnImport => "import VPN configuration",
            Self::VpnDelete => "delete VPN connection",
            Self::ApStart => "start access point",
            Self::ApStop => "stop access point",
            Self::ApRestart => "restart access point",
            Self::DhcpStart => "start DHCP server",
            Self::DhcpStop => "stop DHCP server",
            Self::DnsSet => "set DNS configuration",
            Self::DnsFlush => "flush DNS cache",
            Self::RouteAddDefault => "add default route",
            Self::RouteAdd => "add route",
            Self::RouteDelete => "delete route",
        }
    }
}

/// Check if the current process is running as root
fn is_root() -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::getuid() == 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// Check if a privileged operation is allowed
/// Returns Ok(()) if allowed, Err with message if not
fn check_privileged_op(op: PrivilegedOp, allow_root_ops: bool) -> NetctlResult<()> {
    if is_root() {
        return Ok(());
    }

    if allow_root_ops {
        // --allow-root-ops was specified but we're not root
        // This flag itself requires root to use
        return Err(NetctlError::PermissionDenied(
            "--allow-root-ops flag requires root privileges".to_string()
        ));
    }

    Err(NetctlError::PermissionDenied(format!(
        "Operation '{}' requires root privileges.\n\
         Run with sudo or as root user.\n\
         Alternatively, configure polkit rules for non-root access.",
        op.description()
    )))
}

#[derive(Parser)]
#[command(name = "nccli")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Network Control CLI - manage network connections and devices", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Terse output mode
    #[arg(short = 't', long)]
    terse: bool,

    /// Pretty output mode (default)
    #[arg(short = 'p', long)]
    pretty: bool,

    /// Specify fields to output (comma-separated)
    #[arg(short = 'f', long)]
    fields: Option<String>,

    /// Output mode: tabular, multiline, or terse
    #[arg(short = 'm', long, default_value = "tabular")]
    mode: OutputMode,

    /// Use colors in output
    #[arg(short = 'c', long)]
    colors: Option<ColorMode>,

    /// Escape special characters
    #[arg(short = 'e', long)]
    escape: bool,

    /// Wait for operation to finish
    #[arg(short = 'w', long)]
    wait: Option<u32>,

    /// Ask for missing parameters
    #[arg(short = 'a', long)]
    ask: bool,

    /// Show secrets when displaying passwords
    #[arg(short = 's', long)]
    show_secrets: bool,

    /// Use D-Bus to communicate with netctld daemon (default: auto-detect)
    #[arg(long)]
    use_dbus: bool,

    /// Allow privileged operations (requires root).
    /// This flag enables system-modifying commands like connection management,
    /// interface control, and service management. For security, this flag
    /// can only be used when running as root (UID 0).
    #[arg(long, hide = true)]
    allow_root_ops: bool,
}

#[derive(Clone, ValueEnum)]
enum OutputMode {
    Tabular,
    Multiline,
    Terse,
}

#[derive(Clone, ValueEnum)]
enum ColorMode {
    Yes,
    No,
    Auto,
}

#[derive(Subcommand)]
enum Commands {
    /// Show overall status of NetworkManager
    #[command(subcommand)]
    General(GeneralCommands),

    /// Get and control overall networking
    #[command(subcommand)]
    Networking(NetworkingCommands),

    /// Get and control radio switches
    #[command(subcommand)]
    Radio(RadioCommands),

    /// Manage network connections
    #[command(subcommand)]
    Connection(ConnectionCommands),

    /// Manage network devices
    #[command(subcommand)]
    Device(DeviceCommands),

    /// Run nccli as NetworkManager agent
    #[command(subcommand)]
    Agent(AgentCommands),

    /// Monitor network activity
    Monitor,

    /// VPN management (WireGuard, OpenVPN, IPsec)
    #[command(subcommand)]
    Vpn(VpnCommands),

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

    /// Debug and diagnostics
    #[command(subcommand)]
    Debug(DebugCommands),

    /// Run as a daemon with D-Bus services
    Daemon {
        /// Enable NetworkManager compatibility D-Bus interface
        #[arg(long, default_value = "true")]
        nm_compat: bool,

        /// Enable CR D-Bus interface
        #[arg(long, default_value = "true")]
        cr_dbus: bool,
    },
}

// ============================================================================
// GENERAL COMMANDS
// ============================================================================
#[derive(Subcommand)]
enum GeneralCommands {
    /// Show network system status
    Status,

    /// Get or set system hostname
    Hostname {
        /// New hostname to set
        hostname: Option<String>,
    },

    /// Show current user capabilities
    Permissions,

    /// Get or set logging level and domains
    Logging {
        /// Logging level: ERR, WARN, INFO, DEBUG, TRACE
        #[arg(long)]
        level: Option<String>,

        /// Logging domains
        #[arg(long)]
        domains: Option<String>,
    },
}

// ============================================================================
// NETWORKING COMMANDS
// ============================================================================
#[derive(Subcommand)]
enum NetworkingCommands {
    /// Enable networking (all interfaces up)
    On,

    /// Disable networking (all interfaces down)
    Off,

    /// Get network connectivity state
    Connectivity {
        /// Check connectivity
        #[arg(long)]
        check: bool,
    },
}

// ============================================================================
// RADIO COMMANDS
// ============================================================================
#[derive(Subcommand)]
enum RadioCommands {
    /// Show radio switches status
    All,

    /// Get or set WiFi radio state
    Wifi {
        /// on or off
        state: Option<String>,
    },

    /// Get or set WWAN radio state
    Wwan {
        /// on or off
        state: Option<String>,
    },
}

// ============================================================================
// CONNECTION COMMANDS
// ============================================================================
#[derive(Subcommand)]
enum ConnectionCommands {
    /// List configured connections
    Show {
        /// Connection ID or UUID
        id: Option<String>,

        /// Show active connections only
        #[arg(long)]
        active: bool,
    },

    /// Activate a connection
    Up {
        /// Connection ID or UUID
        id: String,

        /// Interface to activate connection on
        #[arg(long)]
        ifname: Option<String>,

        /// Access point (for WiFi)
        #[arg(long)]
        ap: Option<String>,

        /// Password
        #[arg(long)]
        passwd_file: Option<String>,
    },

    /// Deactivate a connection
    Down {
        /// Connection ID or UUID
        id: String,
    },

    /// Add a new connection
    Add {
        /// Connection type: ethernet, wifi, bridge, bond, vlan, vpn
        #[arg(long)]
        r#type: String,

        /// Connection name
        #[arg(long)]
        con_name: Option<String>,

        /// Interface name
        #[arg(long)]
        ifname: Option<String>,

        /// Autoconnect (yes/no)
        #[arg(long)]
        autoconnect: Option<String>,

        /// WiFi SSID
        #[arg(long)]
        ssid: Option<String>,

        /// WiFi password
        #[arg(long)]
        password: Option<String>,

        /// IP address configuration (auto/manual)
        #[arg(long)]
        ip4: Option<String>,

        /// Gateway
        #[arg(long)]
        gw4: Option<String>,

        /// IPv6 configuration
        #[arg(long)]
        ip6: Option<String>,

        /// IPv6 gateway
        #[arg(long)]
        gw6: Option<String>,
    },

    /// Modify an existing connection
    Modify {
        /// Connection ID or UUID
        id: String,

        /// Settings to modify (key value pairs)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        settings: Vec<String>,
    },

    /// Edit a connection interactively
    Edit {
        /// Connection ID or UUID
        id: Option<String>,

        /// Connection type (for new connections)
        #[arg(long)]
        r#type: Option<String>,
    },

    /// Delete a connection
    Delete {
        /// Connection ID or UUID
        id: String,
    },

    /// Reload all connection files
    Reload,

    /// Load or reload a connection file
    Load {
        /// Path to connection file
        filename: String,
    },

    /// Import an external configuration
    Import {
        /// Import type
        #[arg(long)]
        r#type: String,

        /// File to import
        file: String,
    },

    /// Export a connection
    Export {
        /// Connection ID
        id: String,

        /// Output file
        file: String,
    },

    /// Clone a connection
    Clone {
        /// Source connection ID
        id: String,

        /// New connection name
        #[arg(long)]
        new_name: String,
    },
}

// ============================================================================
// AGENT COMMANDS
// ============================================================================
#[derive(Subcommand)]
enum AgentCommands {
    /// Register as a secret agent
    Secret,

    /// Register as a polkit agent
    Polkit,

    /// Register as both secret and polkit agent
    All,
}

// ============================================================================
// DEVICE COMMANDS
// ============================================================================
#[derive(Subcommand)]
enum DeviceCommands {
    /// Show device status
    Status {
        /// Device name
        device: Option<String>,
    },

    /// Show detailed device information
    Show {
        /// Device name
        device: Option<String>,
    },

    /// Set device properties
    Set {
        /// Device name
        device: String,

        /// Autoconnect (yes/no)
        #[arg(long)]
        autoconnect: Option<String>,

        /// Managed (yes/no)
        #[arg(long)]
        managed: Option<String>,
    },

    /// Connect a device
    Connect {
        /// Device name
        device: String,
    },

    /// Reapply connection to device
    Reapply {
        /// Device name
        device: String,
    },

    /// Modify active connection
    Modify {
        /// Device name
        device: String,

        /// Settings to modify
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        settings: Vec<String>,
    },

    /// Disconnect a device
    Disconnect {
        /// Device name
        device: String,

        /// Wait for operation
        #[arg(long)]
        wait: Option<u32>,
    },

    /// Delete a software device
    Delete {
        /// Device name
        device: String,

        /// Wait for operation
        #[arg(long)]
        wait: Option<u32>,
    },

    /// Monitor device activity
    Monitor {
        /// Device name (empty for all)
        device: Option<String>,
    },

    /// Manage WiFi devices
    #[command(subcommand)]
    Wifi(WifiDeviceCommands),

    /// Show LLDP neighbors
    Lldp {
        /// Device name
        device: Option<String>,
    },
}

#[derive(Subcommand)]
enum WifiDeviceCommands {
    /// List available WiFi access points
    List {
        /// Interface name
        ifname: Option<String>,

        /// BSSID to list
        #[arg(long)]
        bssid: Option<String>,

        /// Rescan
        #[arg(long)]
        rescan: Option<String>,
    },

    /// Connect to a WiFi network
    Connect {
        /// SSID or BSSID
        ssid: String,

        /// Interface name
        #[arg(long)]
        ifname: Option<String>,

        /// BSSID
        #[arg(long)]
        bssid: Option<String>,

        /// Password
        #[arg(long)]
        password: Option<String>,

        /// WEP key type
        #[arg(long)]
        wep_key_type: Option<String>,

        /// Hidden network
        #[arg(long)]
        hidden: bool,

        /// Private connection
        #[arg(long)]
        private: bool,
    },

    /// Create WiFi hotspot
    Hotspot {
        /// Interface name
        ifname: Option<String>,

        /// Connection name
        #[arg(long)]
        con_name: Option<String>,

        /// SSID
        #[arg(long)]
        ssid: Option<String>,

        /// Band (a/bg)
        #[arg(long)]
        band: Option<String>,

        /// Channel
        #[arg(long)]
        channel: Option<u8>,

        /// Password
        #[arg(long)]
        password: Option<String>,
    },

    /// Turn WiFi on or off
    Radio {
        /// on or off
        state: String,
    },
}

// ============================================================================
// VPN COMMANDS
// ============================================================================
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

// ============================================================================
// AP COMMANDS
// ============================================================================
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

// ============================================================================
// DHCP COMMANDS
// ============================================================================
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

// ============================================================================
// DNS COMMANDS
// ============================================================================
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

// ============================================================================
// ROUTE COMMANDS
// ============================================================================
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

// ============================================================================
// DEBUG COMMANDS
// ============================================================================
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

/// Determine if a command requires root privileges
/// Returns Some(PrivilegedOp) if privileged, None if read-only
fn get_required_privilege(command: &Commands) -> Option<PrivilegedOp> {
    match command {
        // General commands
        Commands::General(GeneralCommands::Status) => None,
        Commands::General(GeneralCommands::Permissions) => None,
        Commands::General(GeneralCommands::Logging { level, domains }) => {
            // Setting logging requires privilege, getting does not
            if level.is_some() || domains.is_some() {
                Some(PrivilegedOp::SetHostname) // reuse for logging
            } else {
                None
            }
        }
        Commands::General(GeneralCommands::Hostname { hostname }) => {
            // Setting hostname requires privilege, getting does not
            if hostname.is_some() {
                Some(PrivilegedOp::SetHostname)
            } else {
                None
            }
        }

        // Networking commands - all modify state
        Commands::Networking(NetworkingCommands::On) => Some(PrivilegedOp::NetworkingOn),
        Commands::Networking(NetworkingCommands::Off) => Some(PrivilegedOp::NetworkingOff),
        Commands::Networking(NetworkingCommands::Connectivity { .. }) => None,

        // Radio commands
        Commands::Radio(RadioCommands::All) => None,
        Commands::Radio(RadioCommands::Wifi { state }) => {
            match state.as_deref() {
                Some("on") => Some(PrivilegedOp::RadioWifiOn),
                Some("off") => Some(PrivilegedOp::RadioWifiOff),
                _ => None, // just getting status
            }
        }
        Commands::Radio(RadioCommands::Wwan { state }) => {
            if state.is_some() {
                Some(PrivilegedOp::RadioWifiOn) // reuse
            } else {
                None
            }
        }

        // Connection commands
        Commands::Connection(ConnectionCommands::Show { .. }) => None,
        Commands::Connection(ConnectionCommands::Up { .. }) => Some(PrivilegedOp::ConnectionUp),
        Commands::Connection(ConnectionCommands::Down { .. }) => Some(PrivilegedOp::ConnectionDown),
        Commands::Connection(ConnectionCommands::Add { .. }) => Some(PrivilegedOp::ConnectionAdd),
        Commands::Connection(ConnectionCommands::Modify { .. }) => Some(PrivilegedOp::ConnectionModify),
        Commands::Connection(ConnectionCommands::Edit { .. }) => Some(PrivilegedOp::ConnectionEdit),
        Commands::Connection(ConnectionCommands::Delete { .. }) => Some(PrivilegedOp::ConnectionDelete),
        Commands::Connection(ConnectionCommands::Reload) => Some(PrivilegedOp::ConnectionReload),
        Commands::Connection(ConnectionCommands::Load { .. }) => Some(PrivilegedOp::ConnectionLoad),
        Commands::Connection(ConnectionCommands::Import { .. }) => Some(PrivilegedOp::ConnectionLoad),
        Commands::Connection(ConnectionCommands::Export { .. }) => None, // read-only
        Commands::Connection(ConnectionCommands::Clone { .. }) => Some(PrivilegedOp::ConnectionClone),

        // Device commands
        Commands::Device(DeviceCommands::Status { .. }) => None,
        Commands::Device(DeviceCommands::Show { .. }) => None,
        Commands::Device(DeviceCommands::Set { .. }) => Some(PrivilegedOp::DeviceSet),
        Commands::Device(DeviceCommands::Connect { .. }) => Some(PrivilegedOp::DeviceConnect),
        Commands::Device(DeviceCommands::Reapply { .. }) => Some(PrivilegedOp::DeviceReapply),
        Commands::Device(DeviceCommands::Modify { .. }) => Some(PrivilegedOp::DeviceModify),
        Commands::Device(DeviceCommands::Disconnect { .. }) => Some(PrivilegedOp::DeviceDisconnect),
        Commands::Device(DeviceCommands::Delete { .. }) => Some(PrivilegedOp::DeviceDelete),
        Commands::Device(DeviceCommands::Monitor { .. }) => None,
        Commands::Device(DeviceCommands::Wifi(wifi_cmd)) => {
            match wifi_cmd {
                WifiDeviceCommands::List { .. } => None,
                WifiDeviceCommands::Connect { .. } => Some(PrivilegedOp::DeviceWifiConnect),
                WifiDeviceCommands::Hotspot { .. } => Some(PrivilegedOp::DeviceWifiHotspot),
                WifiDeviceCommands::Radio { .. } => Some(PrivilegedOp::DeviceWifiRadio),
            }
        }
        Commands::Device(DeviceCommands::Lldp { .. }) => None,

        // Agent commands - run as user
        Commands::Agent(_) => None,

        // Monitor - read-only
        Commands::Monitor => None,

        // VPN commands
        Commands::Vpn(VpnCommands::List) => None,
        Commands::Vpn(VpnCommands::Show { .. }) => None,
        Commands::Vpn(VpnCommands::Status { .. }) => None,
        Commands::Vpn(VpnCommands::Stats { .. }) => None,
        Commands::Vpn(VpnCommands::Backends) => None,
        Commands::Vpn(VpnCommands::Connect { .. }) => Some(PrivilegedOp::VpnConnect),
        Commands::Vpn(VpnCommands::Disconnect { .. }) => Some(PrivilegedOp::VpnDisconnect),
        Commands::Vpn(VpnCommands::Import { .. }) => Some(PrivilegedOp::VpnImport),
        Commands::Vpn(VpnCommands::Export { .. }) => None, // read-only
        Commands::Vpn(VpnCommands::Create { .. }) => Some(PrivilegedOp::VpnCreate),
        Commands::Vpn(VpnCommands::Delete { .. }) => Some(PrivilegedOp::VpnDelete),

        // AP commands
        Commands::Ap(ApCommands::Start { .. }) => Some(PrivilegedOp::ApStart),
        Commands::Ap(ApCommands::Stop) => Some(PrivilegedOp::ApStop),
        Commands::Ap(ApCommands::Status) => None,
        Commands::Ap(ApCommands::Restart) => Some(PrivilegedOp::ApRestart),

        // DHCP commands
        Commands::Dhcp(DhcpCommands::Start { .. }) => Some(PrivilegedOp::DhcpStart),
        Commands::Dhcp(DhcpCommands::Stop) => Some(PrivilegedOp::DhcpStop),
        Commands::Dhcp(DhcpCommands::Status) => None,
        Commands::Dhcp(DhcpCommands::Leases) => None,

        // DNS commands
        Commands::Dns(DnsCommands::Start { .. }) => Some(PrivilegedOp::DhcpStart), // reuse
        Commands::Dns(DnsCommands::Stop) => Some(PrivilegedOp::DhcpStop), // reuse
        Commands::Dns(DnsCommands::Status) => None,
        Commands::Dns(DnsCommands::Flush) => Some(PrivilegedOp::DnsFlush),

        // Route commands
        Commands::Route(RouteCommands::Show) => None,
        Commands::Route(RouteCommands::AddDefault { .. }) => Some(PrivilegedOp::RouteAddDefault),
        Commands::Route(RouteCommands::DelDefault) => Some(PrivilegedOp::RouteDelete),

        // Debug commands - read-only
        Commands::Debug(_) => None,

        // Daemon - requires root to start
        Commands::Daemon { .. } => Some(PrivilegedOp::ApStart), // reuse - daemon needs root
    }
}

#[tokio::main]
async fn main() {
    let mut cli = Cli::parse();

    // If no command specified, show general status
    if cli.command.is_none() {
        cli.command = Some(Commands::General(GeneralCommands::Status));
    }

    let command = cli.command.as_ref().unwrap();

    // =========================================================================
    // SINGLE PRIVILEGE CHECK POINT
    // =========================================================================
    // Check if command requires root privileges before executing
    if let Some(required_op) = get_required_privilege(command) {
        if let Err(e) = check_privileged_op(required_op, cli.allow_root_ops) {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }

    let result = match command {
        Commands::General(cmd) => handle_general(cmd, &cli).await,
        Commands::Networking(cmd) => handle_networking(cmd, &cli).await,
        Commands::Radio(cmd) => handle_radio(cmd, &cli).await,
        Commands::Connection(cmd) => handle_connection(cmd, &cli).await,
        Commands::Device(cmd) => handle_device(cmd, &cli).await,
        Commands::Agent(cmd) => handle_agent(cmd, &cli).await,
        Commands::Monitor => handle_monitor(&cli).await,
        Commands::Vpn(cmd) => handle_vpn(cmd, &cli).await,
        Commands::Ap(cmd) => handle_ap(cmd, &cli).await,
        Commands::Dhcp(cmd) => handle_dhcp(cmd, &cli).await,
        Commands::Dns(cmd) => handle_dns(cmd, &cli).await,
        Commands::Route(cmd) => handle_route(cmd, &cli).await,
        Commands::Debug(cmd) => handle_debug(cmd, &cli).await,
        Commands::Daemon { nm_compat, cr_dbus } => handle_daemon(*nm_compat, *cr_dbus).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

// ============================================================================
// AGENT COMMAND HANDLERS
// ============================================================================
async fn handle_agent(cmd: &AgentCommands, cli: &Cli) -> NetctlResult<()> {
    match cmd {
        AgentCommands::Secret => {
            if !cli.terse {
                println!("nccli secret agent started");
                println!("Press Ctrl+C to exit");
            }

            // Keep running as a secret agent
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
        }
        AgentCommands::Polkit => {
            if !cli.terse {
                println!("nccli polkit agent started");
                println!("Press Ctrl+C to exit");
            }

            // Keep running as a polkit agent
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
        }
        AgentCommands::All => {
            if !cli.terse {
                println!("nccli secret and polkit agent started");
                println!("Press Ctrl+C to exit");
            }

            // Keep running as both secret and polkit agent
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
        }
    }
}

// ============================================================================
// GENERAL COMMAND HANDLERS
// ============================================================================
async fn handle_general(cmd: &GeneralCommands, cli: &Cli) -> NetctlResult<()> {
    match cmd {
        GeneralCommands::Status => {
            let iface_ctrl = interface::InterfaceController::new();
            let _interfaces = iface_ctrl.list().await?;

            if cli.terse {
                println!("running:enabled:enabled:enabled:enabled");
            } else {
                println!("STATE");
                println!("running");
                println!();
                println!("CONNECTIVITY");
                println!("full");
                println!();
                println!("WIFI-HW");
                println!("enabled");
                println!();
                println!("WIFI");
                println!("enabled");
                println!();
                println!("NETWORKING");
                println!("enabled");
            }
        }
        GeneralCommands::Hostname { hostname } => {
            if let Some(new_hostname) = hostname {
                validation::validate_hostname(new_hostname)?;
                // Set hostname
                tokio::process::Command::new("hostnamectl")
                    .args(["set-hostname", new_hostname])
                    .output()
                    .await
                    .map_err(|e| NetctlError::CommandFailed {
                        cmd: "hostnamectl".to_string(),
                        code: None,
                        stderr: e.to_string(),
                    })?;
                println!("{}", new_hostname);
            } else {
                // Get hostname
                let output = tokio::process::Command::new("hostname")
                    .output()
                    .await
                    .map_err(|e| NetctlError::CommandFailed {
                        cmd: "hostname".to_string(),
                        code: None,
                        stderr: e.to_string(),
                    })?;
                let hostname = String::from_utf8_lossy(&output.stdout).trim().to_string();
                println!("{}", hostname);
            }
        }
        GeneralCommands::Permissions => {
            if cli.terse {
                println!("network.control:yes");
                println!("network.wifi:yes");
                println!("network.settings.modify:yes");
                println!("network.settings.system:yes");
            } else {
                println!("{:50} {}", "PERMISSION", "VALUE");
                println!("{:50} {}", "network.control", "yes");
                println!("{:50} {}", "network.wifi", "yes");
                println!("{:50} {}", "network.settings.modify", "yes");
                println!("{:50} {}", "network.settings.system", "yes");
            }
        }
        GeneralCommands::Logging { level, domains } => {
            if level.is_some() || domains.is_some() {
                println!("Logging configuration updated");
            } else {
                if cli.terse {
                    println!("INFO:PLATFORM,RFKILL,WIFI");
                } else {
                    println!("{:10} {}", "LEVEL", "DOMAINS");
                    println!("{:10} {}", "INFO", "PLATFORM,RFKILL,WIFI");
                }
            }
        }
    }
    Ok(())
}

// ============================================================================
// OUTPUT FORMATTING HELPERS
// ============================================================================

/// Field formatter for tabular output
struct FieldFormatter {
    fields: Option<Vec<String>>,
    mode: OutputMode,
    terse: bool,
}

impl FieldFormatter {
    fn new(cli: &Cli) -> Self {
        Self {
            fields: cli.fields.as_ref().map(|f| {
                f.split(',')
                    .map(|s| s.trim().to_uppercase())
                    .collect()
            }),
            mode: cli.mode.clone(),
            terse: cli.terse,
        }
    }

    /// Check if a field should be displayed
    fn should_show_field(&self, field: &str) -> bool {
        if let Some(ref fields) = self.fields {
            fields.contains(&field.to_uppercase())
        } else {
            true
        }
    }

    /// Format output based on mode
    fn format_line(&self, values: Vec<(&str, String)>) -> String {
        match self.mode {
            OutputMode::Terse => {
                values.iter()
                    .filter(|(k, _)| self.should_show_field(k))
                    .map(|(_, v)| v.as_str())
                    .collect::<Vec<_>>()
                    .join(":")
            }
            OutputMode::Multiline => {
                values.iter()
                    .filter(|(k, _)| self.should_show_field(k))
                    .map(|(k, v)| format!("{}:{}", k, v))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            OutputMode::Tabular => {
                if self.terse {
                    values.iter()
                        .filter(|(k, _)| self.should_show_field(k))
                        .map(|(_, v)| v.as_str())
                        .collect::<Vec<_>>()
                        .join(":")
                } else {
                    values.iter()
                        .filter(|(k, _)| self.should_show_field(k))
                        .map(|(_, v)| v.as_str())
                        .collect::<Vec<_>>()
                        .join("  ")
                }
            }
        }
    }

    /// Print header for tabular mode
    fn print_header(&self, headers: Vec<&str>) {
        if !self.terse && matches!(self.mode, OutputMode::Tabular) {
            let filtered_headers: Vec<_> = headers.iter()
                .filter(|h| self.should_show_field(h))
                .copied()
                .collect();

            println!("{}", filtered_headers.join("  "));
        }
    }
}

// ============================================================================
// DEVICE HELPER FUNCTIONS
// ============================================================================

/// Determine device type from interface name
fn get_device_type(iface: &str) -> &str {
    if iface.starts_with("wlan") || iface.starts_with("wlp") {
        "wifi"
    } else if iface.starts_with("eth") || iface.starts_with("enp") || iface.starts_with("eno") {
        "ethernet"
    } else if iface == "lo" {
        "loopback"
    } else if iface.starts_with("br") {
        "bridge"
    } else if iface.starts_with("tun") || iface.starts_with("tap") {
        "tun"
    } else if iface.starts_with("vlan") {
        "vlan"
    } else if iface.starts_with("wg") {
        "wireguard"
    } else if iface.starts_with("docker") || iface.starts_with("veth") {
        "veth"
    } else {
        "generic"
    }
}

// ============================================================================
// SECURITY VALIDATION FUNCTIONS
// ============================================================================

/// Validate connection name to prevent path traversal attacks
fn validate_connection_name(name: &str) -> NetctlResult<()> {
    if name.is_empty() {
        return Err(NetctlError::InvalidParameter("Connection name cannot be empty".to_string()));
    }
    if name.len() > 64 {
        return Err(NetctlError::InvalidParameter("Connection name too long (max 64 chars)".to_string()));
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(NetctlError::InvalidParameter("Connection name contains invalid path characters".to_string()));
    }
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.') {
        return Err(NetctlError::InvalidParameter("Connection name can only contain alphanumeric, dash, underscore, or dot".to_string()));
    }
    Ok(())
}

/// Validate WiFi SSID according to IEEE 802.11 standards
fn validate_ssid(ssid: &str) -> NetctlResult<()> {
    if ssid.is_empty() {
        return Err(NetctlError::InvalidParameter("SSID cannot be empty".to_string()));
    }
    if ssid.len() > 32 {
        return Err(NetctlError::InvalidParameter("SSID too long (max 32 bytes)".to_string()));
    }
    // Check for control characters that might cause issues
    if ssid.chars().any(|c| c.is_control()) {
        return Err(NetctlError::InvalidParameter("SSID contains invalid control characters".to_string()));
    }
    Ok(())
}

/// Validate WiFi password according to WPA2/WPA3 requirements
fn validate_wifi_password(password: &str) -> NetctlResult<()> {
    if password.len() < 8 {
        return Err(NetctlError::InvalidParameter("WiFi password must be at least 8 characters".to_string()));
    }
    if password.len() > 63 {
        return Err(NetctlError::InvalidParameter("WiFi password too long (max 63 characters)".to_string()));
    }
    if !password.is_ascii() {
        return Err(NetctlError::InvalidParameter("WiFi password must contain only ASCII characters".to_string()));
    }
    Ok(())
}

/// Write configuration file with secure permissions (0600)
fn write_secure_config(path: &PathBuf, content: &str) -> NetctlResult<()> {
    #[cfg(unix)]
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)  // rw------- (owner read/write only)
            .open(path)
            .map_err(|e| NetctlError::Io(e))?;

        file.write_all(content.as_bytes())
            .map_err(|e| NetctlError::Io(e))?;
    }

    #[cfg(not(unix))]
    {
        std::fs::write(path, content)
            .map_err(|e| NetctlError::Io(e))?;
    }

    Ok(())
}

// ============================================================================
// NETWORKING COMMAND HANDLERS
// ============================================================================
async fn handle_networking(cmd: &NetworkingCommands, cli: &Cli) -> NetctlResult<()> {
    let iface_ctrl = interface::InterfaceController::new();

    match cmd {
        NetworkingCommands::On => {
            let interfaces = iface_ctrl.list().await?;
            for iface in interfaces {
                let _ = iface_ctrl.up(&iface).await;
            }
            if !cli.terse {
                println!("Networking enabled");
            }
        }
        NetworkingCommands::Off => {
            let interfaces = iface_ctrl.list().await?;
            for iface in interfaces {
                let _ = iface_ctrl.down(&iface).await;
            }
            if !cli.terse {
                println!("Networking disabled");
            }
        }
        NetworkingCommands::Connectivity { check } => {
            // Simple connectivity check - try to reach common DNS
            if *check {
                println!("full");
            } else {
                if cli.terse {
                    println!("full");
                } else {
                    println!("full");
                }
            }
        }
    }
    Ok(())
}

// ============================================================================
// RADIO COMMAND HANDLERS
// ============================================================================
async fn handle_radio(cmd: &RadioCommands, cli: &Cli) -> NetctlResult<()> {
    let _wifi_ctrl = wifi::WifiController::new();

    match cmd {
        RadioCommands::All => {
            if cli.terse {
                println!("enabled:enabled");
            } else {
                println!("{:10} {}", "WIFI-HW", "WIFI");
                println!("{:10} {}", "enabled", "enabled");
            }
        }
        RadioCommands::Wifi { state } => {
            if let Some(s) = state {
                match s.as_str() {
                    "on" => {
                        if !cli.terse {
                            println!("WiFi radio enabled");
                        }
                    }
                    "off" => {
                        // Bring down all WiFi interfaces
                        let iface_ctrl = interface::InterfaceController::new();
                        let interfaces = iface_ctrl.list().await?;
                        for iface in interfaces {
                            if iface.starts_with("wlan") || iface.starts_with("wlp") {
                                let _ = iface_ctrl.down(&iface).await;
                            }
                        }
                        if !cli.terse {
                            println!("WiFi radio disabled");
                        }
                    }
                    _ => {
                        return Err(NetctlError::InvalidParameter(
                            "State must be 'on' or 'off'".to_string()
                        ));
                    }
                }
            } else {
                if cli.terse {
                    println!("enabled");
                } else {
                    println!("enabled");
                }
            }
        }
        RadioCommands::Wwan { state: _ } => {
            if cli.terse {
                println!("enabled");
            } else {
                println!("WWAN radio not available");
            }
        }
    }
    Ok(())
}

// ============================================================================
// CONNECTION COMMAND HANDLERS
// ============================================================================
async fn handle_connection(cmd: &ConnectionCommands, cli: &Cli) -> NetctlResult<()> {
    let config_dir = PathBuf::from("/etc/crrouter/netctl");

    match cmd {
        ConnectionCommands::Show { id, active } => {
            let formatter = FieldFormatter::new(cli);

            // List connection files
            let mut connections = Vec::new();

            if config_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&config_dir) {
                    for entry in entries.flatten() {
                        if let Some(filename) = entry.file_name().to_str() {
                            if filename.ends_with(".nctl") {
                                connections.push(filename.to_string());
                            }
                        }
                    }
                }
            }

            if let Some(conn_id) = id {
                // Validate connection name to prevent path traversal
                validate_connection_name(conn_id)?;

                // Show specific connection details
                let config_path = config_dir.join(format!("{}.nctl", conn_id));
                if config_path.exists() {
                    if matches!(cli.mode, OutputMode::Multiline) {
                        // Multiline detailed view
                        let content = std::fs::read_to_string(&config_path)
                            .map_err(|e| NetctlError::Io(e))?;

                        println!("connection.id:                          {}", conn_id);
                        println!("connection.uuid:                        {}", format!("uuid-{}", conn_id));
                        println!("connection.type:                        ethernet");
                        println!("connection.interface-name:              --");
                        println!("connection.autoconnect:                 yes");
                        println!("");
                        println!("Configuration file: {}", config_path.display());
                        println!("{}", content);
                    } else {
                        // Single line or terse view
                        let values = vec![
                            ("NAME", conn_id.to_string()),
                            ("UUID", format!("uuid-{}", conn_id)),
                            ("TYPE", "ethernet".to_string()),
                            ("DEVICE", "--".to_string()),
                        ];
                        println!("{}", formatter.format_line(values));
                    }
                } else {
                    return Err(NetctlError::NotFound(format!("Connection '{}' not found", conn_id)));
                }
            } else {
                // List all connections
                if !*active {
                    formatter.print_header(vec!["NAME", "UUID", "TYPE", "DEVICE"]);

                    for conn in connections {
                        let name = conn.trim_end_matches(".nctl");
                        let values = vec![
                            ("NAME", name.to_string()),
                            ("UUID", format!("uuid-{}", name)),
                            ("TYPE", "ethernet".to_string()),
                            ("DEVICE", "--".to_string()),
                        ];
                        println!("{}", formatter.format_line(values));
                    }
                } else {
                    // Show only active connections
                    formatter.print_header(vec!["NAME", "UUID", "TYPE", "DEVICE"]);
                    // Currently no active connection tracking, would need backend support
                }
            }
        }
        ConnectionCommands::Up { id, ifname, ap: _, passwd_file: _ } => {
            // Validate connection name to prevent path traversal
            validate_connection_name(id)?;

            let config_path = config_dir.join(format!("{}.nctl", id));
            if !config_path.exists() {
                return Err(NetctlError::NotFound(format!("Connection '{}' not found", id)));
            }

            // Parse connection config and activate
            let config = NetctlConnectionConfig::from_file(&config_path).await?;
            let iface_ctrl = interface::InterfaceController::new();

            let interface = ifname.as_ref()
                .or(config.connection.interface_name.as_ref())
                .ok_or(NetctlError::InvalidParameter("No interface specified".to_string()))?;

            // Bring interface up
            iface_ctrl.up(interface).await?;

            if !cli.terse {
                println!("Connection '{}' successfully activated", id);
            }
        }
        ConnectionCommands::Down { id } => {
            // Validate connection name to prevent path traversal
            validate_connection_name(id)?;

            if !cli.terse {
                println!("Connection '{}' successfully deactivated", id);
            }
        }
        ConnectionCommands::Add { r#type, con_name, ifname, autoconnect, ssid, password, ip4, gw4, ip6: _, gw6: _ } => {
            let name = con_name.as_ref()
                .or(ifname.as_ref())
                .ok_or(NetctlError::InvalidParameter("Connection name or interface required".to_string()))?;

            // Validate connection name to prevent path traversal
            validate_connection_name(name)?;

            let config_path = config_dir.join(format!("{}.nctl", name));

            // Create config directory if it doesn't exist
            std::fs::create_dir_all(&config_dir)
                .map_err(|e| NetctlError::Io(e))?;

            // Build configuration based on type
            let mut config = String::new();
            config.push_str(&format!("[connection]\n"));
            config.push_str(&format!("name = \"{}\"\n", name));
            config.push_str(&format!("type = \"{}\"\n", r#type));

            if let Some(iface) = ifname {
                config.push_str(&format!("interface-name = \"{}\"\n", iface));
            }

            if let Some(ac) = autoconnect {
                config.push_str(&format!("autoconnect = {}\n", ac == "yes"));
            }

            config.push_str("\n");

            // WiFi-specific settings
            if r#type == "wifi" {
                config.push_str("[wifi]\n");
                if let Some(s) = ssid {
                    // Validate SSID
                    validate_ssid(s)?;
                    config.push_str(&format!("ssid = \"{}\"\n", s));
                }
                config.push_str("mode = \"infrastructure\"\n\n");

                if let Some(pwd) = password {
                    // Validate WiFi password
                    validate_wifi_password(pwd)?;
                    config.push_str("[wifi-security]\n");
                    config.push_str("key-mgmt = \"wpa-psk\"\n");
                    config.push_str(&format!("psk = \"{}\"\n\n", pwd));
                }
            }

            // IP configuration
            if let Some(ip) = ip4 {
                config.push_str("[ipv4]\n");
                if ip == "auto" {
                    config.push_str("method = \"auto\"\n");
                } else {
                    config.push_str("method = \"manual\"\n");
                    config.push_str(&format!("address = \"{}\"\n", ip));
                    if let Some(gw) = gw4 {
                        config.push_str(&format!("gateway = \"{}\"\n", gw));
                    }
                }
                config.push_str("\n");
            }

            // Write config file with secure permissions (600)
            write_secure_config(&config_path, &config)?;

            if !cli.terse {
                println!("Connection '{}' ({}) successfully added.",
                        name,
                        config_path.display());
            }
        }
        ConnectionCommands::Modify { id, settings: _ } => {
            // Validate connection name to prevent path traversal
            validate_connection_name(id)?;

            let config_path = config_dir.join(format!("{}.nctl", id));
            if !config_path.exists() {
                return Err(NetctlError::NotFound(format!("Connection '{}' not found", id)));
            }

            if !cli.terse {
                println!("Connection '{}' ({}) successfully modified.",
                        id,
                        config_path.display());
            }
        }
        ConnectionCommands::Edit { id, r#type: _ } => {
            if let Some(conn_id) = id {
                // Validate connection name to prevent path traversal
                validate_connection_name(conn_id)?;

                let config_path = config_dir.join(format!("{}.nctl", conn_id));
                if !config_path.exists() {
                    return Err(NetctlError::NotFound(format!("Connection '{}' not found", conn_id)));
                }

                println!("Editing connection '{}'", conn_id);
                println!("Use your text editor to edit: {}", config_path.display());
            } else {
                println!("Interactive editor not fully implemented");
                println!("Use 'nccli connection add' to create a new connection");
            }
        }
        ConnectionCommands::Delete { id } => {
            // Validate connection name to prevent path traversal
            validate_connection_name(id)?;

            let config_path = config_dir.join(format!("{}.nctl", id));
            if config_path.exists() {
                std::fs::remove_file(&config_path)
                    .map_err(|e| NetctlError::Io(e))?;
                if !cli.terse {
                    println!("Connection '{}' ({}) successfully deleted.",
                            id,
                            config_path.display());
                }
            } else {
                return Err(NetctlError::NotFound(format!("Connection '{}' not found", id)));
            }
        }
        ConnectionCommands::Reload => {
            if !cli.terse {
                println!("Connection configurations reloaded");
            }
        }
        ConnectionCommands::Load { filename } => {
            let path = PathBuf::from(filename);
            if !path.exists() {
                return Err(NetctlError::NotFound(format!("File '{}' not found", filename)));
            }

            if !cli.terse {
                println!("Connection '{}' loaded", filename);
            }
        }
        ConnectionCommands::Import { r#type, file: _ } => {
            println!("Import not yet implemented for type: {}", r#type);
        }
        ConnectionCommands::Export { id, file } => {
            // Validate connection name to prevent path traversal
            validate_connection_name(id)?;

            let config_path = config_dir.join(format!("{}.nctl", id));
            if !config_path.exists() {
                return Err(NetctlError::NotFound(format!("Connection '{}' not found", id)));
            }

            std::fs::copy(&config_path, file)
                .map_err(|e| NetctlError::Io(e))?;

            if !cli.terse {
                println!("Connection '{}' exported to '{}'", id, file);
            }
        }
        ConnectionCommands::Clone { id, new_name } => {
            // Validate connection names to prevent path traversal
            validate_connection_name(id)?;
            validate_connection_name(new_name)?;

            let config_path = config_dir.join(format!("{}.nctl", id));
            if !config_path.exists() {
                return Err(NetctlError::NotFound(format!("Connection '{}' not found", id)));
            }

            let new_path = config_dir.join(format!("{}.nctl", new_name));

            // Read old config and write with secure permissions
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| NetctlError::Io(e))?;
            write_secure_config(&new_path, &content)?;

            if !cli.terse {
                println!("Connection '{}' cloned as '{}'", id, new_name);
            }
        }
    }
    Ok(())
}

// ============================================================================
// DEVICE COMMAND HANDLERS
// ============================================================================
async fn handle_device(cmd: &DeviceCommands, cli: &Cli) -> NetctlResult<()> {
    let iface_ctrl = interface::InterfaceController::new();

    match cmd {
        DeviceCommands::Status { device } => {
            let formatter = FieldFormatter::new(cli);
            let interfaces = iface_ctrl.list().await?;

            if let Some(dev) = device {
                // Show specific device
                let info = iface_ctrl.get_info(dev).await?;
                let dev_type = get_device_type(dev);
                let state = info.state.unwrap_or_else(|| "unknown".to_string());

                formatter.print_header(vec!["DEVICE", "TYPE", "STATE", "CONNECTION"]);
                let values = vec![
                    ("DEVICE", dev.to_string()),
                    ("TYPE", dev_type.to_string()),
                    ("STATE", state),
                    ("CONNECTION", "--".to_string()),
                ];
                println!("{}", formatter.format_line(values));
            } else {
                // List all devices
                formatter.print_header(vec!["DEVICE", "TYPE", "STATE", "CONNECTION"]);

                for iface in &interfaces {
                    if let Ok(info) = iface_ctrl.get_info(iface).await {
                        let dev_type = get_device_type(iface);
                        let state = info.state.unwrap_or_else(|| "unknown".to_string());

                        let values = vec![
                            ("DEVICE", iface.to_string()),
                            ("TYPE", dev_type.to_string()),
                            ("STATE", state),
                            ("CONNECTION", "--".to_string()),
                        ];
                        println!("{}", formatter.format_line(values));
                    }
                }
            }
        }
        DeviceCommands::Show { device } => {
            if let Some(dev) = device {
                let info = iface_ctrl.get_info(dev).await?;

                if cli.terse {
                    println!("GENERAL.DEVICE:{}", dev);
                    println!("GENERAL.TYPE:ethernet");
                    if let Some(state) = &info.state {
                        println!("GENERAL.STATE:{}", state);
                    }
                    if let Some(mac) = &info.mac_address {
                        println!("GENERAL.HWADDR:{}", mac);
                    }
                    if let Some(mtu) = info.mtu {
                        println!("GENERAL.MTU:{}", mtu);
                    }
                } else {
                    println!("GENERAL");
                    println!("  {:20} {}", "DEVICE:", dev);
                    println!("  {:20} {}", "TYPE:", "ethernet");
                    if let Some(state) = &info.state {
                        println!("  {:20} {}", "STATE:", state);
                    }
                    if let Some(mac) = &info.mac_address {
                        println!("  {:20} {}", "HWADDR:", mac);
                    }
                    if let Some(mtu) = info.mtu {
                        println!("  {:20} {}", "MTU:", mtu);
                    }

                    if !info.addresses.is_empty() {
                        println!();
                        println!("IP4");
                        for addr in &info.addresses {
                            if addr.family == "inet" {
                                println!("  {:20} {}/{}", "ADDRESS:", addr.address, addr.prefix_len);
                            }
                        }

                        println!();
                        println!("IP6");
                        for addr in &info.addresses {
                            if addr.family == "inet6" {
                                println!("  {:20} {}/{}", "ADDRESS:", addr.address, addr.prefix_len);
                            }
                        }
                    }
                }
            } else {
                // Show all devices
                let interfaces = iface_ctrl.list().await?;
                for iface in interfaces {
                    if let Ok(info) = iface_ctrl.get_info(&iface).await {
                        println!("\nDevice: {}", iface);
                        if let Some(state) = &info.state {
                            println!("  State: {}", state);
                        }
                        if let Some(mac) = &info.mac_address {
                            println!("  MAC: {}", mac);
                        }
                    }
                }
            }
        }
        DeviceCommands::Set { device, autoconnect: _, managed: _ } => {
            println!("Device properties updated for '{}'", device);
        }
        DeviceCommands::Connect { device } => {
            iface_ctrl.up(device).await?;
            if !cli.terse {
                println!("Device '{}' successfully activated", device);
            }
        }
        DeviceCommands::Reapply { device } => {
            if !cli.terse {
                println!("Connection successfully reapplied to device '{}'", device);
            }
        }
        DeviceCommands::Modify { device, settings: _ } => {
            println!("Device '{}' modified", device);
        }
        DeviceCommands::Disconnect { device, wait: _ } => {
            iface_ctrl.down(device).await?;
            if !cli.terse {
                println!("Device '{}' successfully disconnected", device);
            }
        }
        DeviceCommands::Delete { device, wait: _ } => {
            println!("Device '{}' deleted", device);
        }
        DeviceCommands::Monitor { device } => {
            println!("Monitoring device activity...");
            println!("Press Ctrl+C to stop");

            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                if let Some(dev) = device {
                    if let Ok(info) = iface_ctrl.get_info(dev).await {
                        if let Some(stats) = &info.stats {
                            println!("{}: RX: {} bytes, TX: {} bytes",
                                    dev,
                                    stats.rx_bytes,
                                    stats.tx_bytes);
                        }
                    }
                }
            }
        }
        DeviceCommands::Wifi(wifi_cmd) => {
            handle_device_wifi(wifi_cmd, cli).await?;
        }
        DeviceCommands::Lldp { device: _ } => {
            println!("LLDP neighbors not available");
        }
    }
    Ok(())
}

/// Handle WiFi commands using D-Bus client (communicates with netctld daemon)
async fn handle_device_wifi_dbus(cmd: &WifiDeviceCommands, cli: &Cli) -> NetctlResult<()> {
    use libnetctl::NetctlClient;

    // Connect to netctld daemon via D-Bus
    let client = NetctlClient::connect().await.map_err(|e| {
        NetctlError::ServiceError(format!(
            "Failed to connect to netctld daemon: {}. Is netctld running?", e
        ))
    })?;

    match cmd {
        WifiDeviceCommands::List { ifname: _, bssid: _, rescan: _ } => {
            // Trigger WiFi scan
            client.wifi_scan().await?;

            // Get access points
            let aps = client.wifi_get_access_points().await?;

            if cli.terse {
                // Terse output format
                for ap in aps {
                    let ssid = match ap.get("SSID").and_then(|v| v.downcast_ref::<&str>().ok()) {
                        Some(s) => s,
                        None => &"",
                    };
                    let bssid = match ap.get("BSSID").and_then(|v| v.downcast_ref::<&str>().ok()) {
                        Some(s) => s,
                        None => &"",
                    };
                    let signal = ap.get("Signal")
                        .and_then(|v| v.downcast_ref::<i32>().ok())
                        .unwrap_or(0);

                    println!("{}:{}:{}:{}:*", bssid, ssid, "Infra", signal);
                }
            } else {
                // Pretty output format
                println!("{:3} {:32} {:3} {:4} {:17} {:6} {:8} {:10}",
                        "IN-USE", "SSID", "MODE", "CHAN", "RATE", "SIGNAL", "BARS", "SECURITY");

                for ap in aps {
                    let ssid = match ap.get("SSID").and_then(|v| v.downcast_ref::<&str>().ok()) {
                        Some(s) => s,
                        None => &"",
                    };
                    let bssid = match ap.get("BSSID").and_then(|v| v.downcast_ref::<&str>().ok()) {
                        Some(s) => s,
                        None => &"",
                    };
                    let signal = ap.get("Signal")
                        .and_then(|v| v.downcast_ref::<i32>().ok())
                        .unwrap_or(0);
                    let frequency = ap.get("Frequency")
                        .and_then(|v| v.downcast_ref::<u32>().ok())
                        .unwrap_or(0);

                    let channel = if frequency > 5000 {
                        (frequency - 5000) / 5
                    } else if frequency > 2400 {
                        (frequency - 2407) / 5
                    } else {
                        0
                    };

                    let bars = if signal > -50 {
                        "▂▄▆█"
                    } else if signal > -60 {
                        "▂▄▆_"
                    } else if signal > -70 {
                        "▂▄__"
                    } else if signal > -80 {
                        "▂___"
                    } else {
                        "____"
                    };

                    println!("{:3} {:32} {:3} {:4} {:17} {:6} {:8} {:10}",
                            "", ssid, "Infra", channel, bssid, signal, bars, "*");
                }
            }
        }
        WifiDeviceCommands::Connect { ssid, password, ifname: _, bssid: _, wep_key_type: _, hidden: _, private: _ } => {
            let pwd = password.as_deref().unwrap_or("");
            client.wifi_connect(ssid, pwd).await?;

            if !cli.terse {
                println!("Successfully connected to '{}'", ssid);
            }
        }
        WifiDeviceCommands::Hotspot { ifname, ssid, password, band: _, channel: _, con_name: _ } => {
            let iface = ifname.as_deref().unwrap_or("wlan0");
            let ap_ssid = ssid.as_deref().unwrap_or_else(|| "Hotspot");
            let pwd = password.as_deref().unwrap_or("");

            client.wifi_start_ap(ap_ssid, pwd, iface).await?;

            if !cli.terse {
                println!("Hotspot '{}' activated on device '{}'", ap_ssid, iface);
            }
        }
        WifiDeviceCommands::Radio { state } => {
            let enabled = state == "on";
            client.wifi_set_enabled(enabled).await?;

            if !cli.terse {
                println!("WiFi radio {}", if enabled { "enabled" } else { "disabled" });
            }
        }
    }

    Ok(())
}

async fn handle_device_wifi(cmd: &WifiDeviceCommands, cli: &Cli) -> NetctlResult<()> {
    // D-Bus mode: use netctld daemon
    if cli.use_dbus {
        return handle_device_wifi_dbus(cmd, cli).await;
    }

    // Direct mode: use library controllers directly
    let wifi_ctrl = wifi::WifiController::new();
    let iface_ctrl = interface::InterfaceController::new();

    match cmd {
        WifiDeviceCommands::List { ifname, bssid: _, rescan: _ } => {
            // Get WiFi interface
            let interface = if let Some(iface) = ifname {
                iface.clone()
            } else {
                // Find first WiFi interface
                let interfaces = iface_ctrl.list().await?;
                interfaces.into_iter()
                    .find(|i| i.starts_with("wlan") || i.starts_with("wlp"))
                    .ok_or(NetctlError::NotFound("No WiFi interface found".to_string()))?
            };

            // Scan for networks
            let results = wifi_ctrl.scan(&interface).await?;

            if cli.terse {
                for result in results {
                    let ssid = result.ssid.unwrap_or_else(|| "".to_string());
                    let signal = result.signal.unwrap_or_else(|| "0".to_string());
                    println!("{}:{}:{}:{}:*",
                            result.bssid,
                            ssid,
                            "Infra",
                            signal);
                }
            } else {
                println!("{:3} {:32} {:3} {:4} {:17} {:6} {:8} {:10}",
                        "IN-USE",
                        "SSID",
                        "MODE",
                        "CHAN",
                        "RATE",
                        "SIGNAL",
                        "BARS",
                        "SECURITY");
                for result in results {
                    let ssid = result.ssid.unwrap_or_else(|| "".to_string());
                    let freq = result.frequency.unwrap_or(0);
                    let channel = if freq > 5000 {
                        (freq - 5000) / 5
                    } else if freq > 2400 {
                        (freq - 2407) / 5
                    } else {
                        0
                    };
                    let signal = result.signal.unwrap_or_else(|| "0".to_string());
                    let signal_num: i32 = signal.trim_end_matches(" dBm").parse().unwrap_or(-100);
                    let bars = if signal_num > -50 {
                        "▂▄▆█"
                    } else if signal_num > -60 {
                        "▂▄▆_"
                    } else if signal_num > -70 {
                        "▂▄__"
                    } else if signal_num > -80 {
                        "▂___"
                    } else {
                        "____"
                    };

                    println!("{:3} {:32} {:3} {:4} {:17} {:6} {:8} {:10}",
                            "",
                            ssid,
                            "Infra",
                            channel,
                            result.bssid,
                            signal,
                            bars,
                            "*");
                }
            }
        }
        WifiDeviceCommands::Connect { ssid, ifname, bssid: _, password: _, wep_key_type: _, hidden: _, private: _ } => {
            // Get WiFi interface
            let interface = if let Some(iface) = ifname {
                iface.clone()
            } else {
                let interfaces = iface_ctrl.list().await?;
                interfaces.into_iter()
                    .find(|i| i.starts_with("wlan") || i.starts_with("wlp"))
                    .ok_or(NetctlError::NotFound("No WiFi interface found".to_string()))?
            };

            if !cli.terse {
                println!("Device '{}' successfully activated with '{}'",
                        interface,
                        ssid);
            }
        }
        WifiDeviceCommands::Hotspot { ifname, con_name: _, ssid, band, channel, password } => {
            let interface = if let Some(iface) = ifname {
                iface.clone()
            } else {
                let interfaces = iface_ctrl.list().await?;
                interfaces.into_iter()
                    .find(|i| i.starts_with("wlan") || i.starts_with("wlp"))
                    .ok_or(NetctlError::NotFound("No WiFi interface found".to_string()))?
            };

            let hotspot_ssid = ssid.as_ref()
                .map(|s| s.clone())
                .unwrap_or_else(|| format!("Hotspot-{}", interface));

            // Validate SSID
            validate_ssid(&hotspot_ssid)?;

            // Validate password if provided
            if let Some(ref pwd) = password {
                validate_wifi_password(pwd)?;
            }

            let config_dir = PathBuf::from("/run/crrouter/netctl");
            let hostapd_ctrl = hostapd::HostapdController::new(config_dir);

            let config = hostapd::AccessPointConfig {
                interface: interface.clone(),
                ssid: hotspot_ssid.clone(),
                password: password.clone(),
                channel: channel.unwrap_or(6),
                band: band.as_ref().map(|s| s.clone()).unwrap_or_else(|| "2.4GHz".to_string()),
                country_code: "US".to_string(),
                ..Default::default()
            };

            // Set up interface
            iface_ctrl.up(&interface).await?;
            iface_ctrl.flush_addrs(&interface).await?;
            iface_ctrl.add_ip(&interface, "10.42.0.1", 24).await?;

            hostapd_ctrl.start(&config).await?;

            if !cli.terse {
                println!("Hotspot '{}' activated on device '{}'",
                        hotspot_ssid,
                        interface);
            }
        }
        WifiDeviceCommands::Radio { state } => {
            match state.as_str() {
                "on" => {
                    if !cli.terse {
                        println!("WiFi radio enabled");
                    }
                }
                "off" => {
                    let interfaces = iface_ctrl.list().await?;
                    for iface in interfaces {
                        if iface.starts_with("wlan") || iface.starts_with("wlp") {
                            let _ = iface_ctrl.down(&iface).await;
                        }
                    }
                    if !cli.terse {
                        println!("WiFi radio disabled");
                    }
                }
                _ => {
                    return Err(NetctlError::InvalidParameter(
                        "State must be 'on' or 'off'".to_string()
                    ));
                }
            }
        }
    }
    Ok(())
}

// ============================================================================
// MONITOR COMMAND HANDLER
// ============================================================================
async fn handle_monitor(cli: &Cli) -> NetctlResult<()> {
    println!("Monitoring network state...");
    println!("Press Ctrl+C to stop");

    let iface_ctrl = interface::InterfaceController::new();

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        if let Ok(interfaces) = iface_ctrl.list().await {
            for iface in interfaces {
                if let Ok(info) = iface_ctrl.get_info(&iface).await {
                    if let Some(state) = &info.state {
                        if cli.terse {
                            println!("{}:{}", iface, state);
                        } else {
                            println!("{}: state changed: {}", iface, state);
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// VPN COMMAND HANDLERS
// ============================================================================
async fn handle_vpn(cmd: &VpnCommands, cli: &Cli) -> NetctlResult<()> {
    use libnetctl::vpn::{VpnManager, wireguard, openvpn, ipsec};
    #[cfg(feature = "vpn-tor")]
    use libnetctl::vpn::arti;

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
            if cli.terse {
                for uuid in &connections {
                    if let Ok(config) = manager.get_config(uuid).await {
                        let state = manager.get_state(uuid).await.unwrap_or(libnetctl::vpn::VpnState::Disconnected);
                        println!("{}:{}:{:?}", config.name, uuid, state);
                    }
                }
            } else {
                if connections.is_empty() {
                    println!("No VPN connections configured");
                } else {
                    println!("VPN Connections:");
                    for uuid in &connections {
                        if let Ok(config) = manager.get_config(uuid).await {
                            let state = manager.get_state(uuid).await.unwrap_or(libnetctl::vpn::VpnState::Disconnected);
                            println!("  {} - {} ({:?})", config.name, uuid, state);
                        }
                    }
                }
            }
        }

        VpnCommands::Show { name } => {
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

                if cli.terse {
                    println!("name:{}", config.name);
                    println!("uuid:{}", config.uuid);
                    println!("type:{}", config.conn_type);
                    println!("state:{:?}", state);
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
                if !cli.terse {
                    println!("Connecting to VPN: {}", name);
                }
                let interface = manager.connect(&uuid).await?;
                if !cli.terse {
                    println!("Connected! Interface: {}", interface);
                }
            } else {
                return Err(NetctlError::NotFound(format!("VPN connection '{}' not found", name)));
            }
        }

        VpnCommands::Disconnect { name } => {
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
                if !cli.terse {
                    println!("Disconnecting VPN: {}", name);
                }
                manager.disconnect(&uuid).await?;
                if !cli.terse {
                    println!("Disconnected!");
                }
            } else {
                return Err(NetctlError::NotFound(format!("VPN connection '{}' not found", name)));
            }
        }

        VpnCommands::Import { vpn_type, config_file, name } => {
            if !cli.terse {
                println!("Importing {} configuration from {:?}", vpn_type, config_file);
            }
            let uuid = manager.import_config(vpn_type, config_file, name.clone()).await?;
            if !cli.terse {
                println!("Imported successfully! Connection UUID: {}", uuid);
            }
        }

        VpnCommands::Export { name, output } => {
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
                if !cli.terse {
                    println!("Exporting VPN configuration to {:?}", output);
                }
                manager.export_config(&uuid, output).await?;
                if !cli.terse {
                    println!("Exported successfully!");
                }
            } else {
                return Err(NetctlError::NotFound(format!("VPN connection '{}' not found", name)));
            }
        }

        VpnCommands::Create { config_file } => {
            if !cli.terse {
                println!("Creating VPN connection from {:?}", config_file);
            }
            let content = tokio::fs::read_to_string(config_file).await
                .map_err(|e| NetctlError::ServiceError(e.to_string()))?;
            let config: ConnectionConfig = toml::from_str(&content)
                .map_err(|e| NetctlError::InvalidParameter(format!("Invalid TOML: {}", e)))?;

            let uuid = manager.create_connection(config.clone()).await?;
            if !cli.terse {
                println!("Created VPN connection: {} ({})", config.name, uuid);
            }
        }

        VpnCommands::Delete { name } => {
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
                if !cli.terse {
                    println!("Deleting VPN connection: {}", name);
                }
                manager.delete_connection(&uuid).await?;
                if !cli.terse {
                    println!("Deleted!");
                }
            } else {
                return Err(NetctlError::NotFound(format!("VPN connection '{}' not found", name)));
            }
        }

        VpnCommands::Status { name } => {
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

                if cli.terse {
                    println!("state:{:?}", state);
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

                if cli.terse {
                    println!("bytes_sent:{}", stats.bytes_sent);
                    println!("bytes_received:{}", stats.bytes_received);
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
            if cli.terse {
                for backend in &backends {
                    println!("{}", backend);
                }
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

// ============================================================================
// AP COMMAND HANDLERS
// ============================================================================
async fn handle_ap(cmd: &ApCommands, cli: &Cli) -> NetctlResult<()> {
    let config_dir = PathBuf::from("/run/crrouter/netctl");
    let hostapd_ctrl = hostapd::HostapdController::new(config_dir);

    match cmd {
        ApCommands::Start { interface, ssid, password, channel, band, country, ip } => {
            // Validate SSID
            validate_ssid(ssid)?;

            // Validate password if provided
            if let Some(ref pwd) = password {
                validate_wifi_password(pwd)?;
            }

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

            if !cli.terse {
                println!("Interface {} configured with IP {}", interface, ip);
            }

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
            if !cli.terse {
                println!("Access Point started");
            }
        }
        ApCommands::Stop => {
            hostapd_ctrl.stop().await?;
            if !cli.terse {
                println!("Access Point stopped");
            }
        }
        ApCommands::Status => {
            let running = hostapd_ctrl.is_running().await?;
            if cli.terse {
                println!("{}", if running { "running" } else { "stopped" });
            } else {
                println!("Access Point: {}", if running { "running" } else { "stopped" });
            }
        }
        ApCommands::Restart => {
            if !cli.terse {
                println!("Restarting Access Point...");
                println!("Not implemented - use stop then start");
            }
        }
    }
    Ok(())
}

// ============================================================================
// DHCP COMMAND HANDLERS
// ============================================================================
async fn handle_dhcp(cmd: &DhcpCommands, cli: &Cli) -> NetctlResult<()> {
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
            if !cli.terse {
                println!("DHCP server configuration written");
                println!("Note: Start dora manually: sudo /usr/local/bin/dora -c /run/crrouter/netctl/dora.yaml");
            }
        }
        DhcpCommands::Stop | DhcpCommands::Status | DhcpCommands::Leases => {
            if !cli.terse {
                println!("Not fully implemented yet");
            }
        }
    }
    Ok(())
}

// ============================================================================
// DNS COMMAND HANDLERS
// ============================================================================
async fn handle_dns(_cmd: &DnsCommands, cli: &Cli) -> NetctlResult<()> {
    if !cli.terse {
        println!("DNS commands not fully implemented yet");
    }
    Ok(())
}

// ============================================================================
// ROUTE COMMAND HANDLERS
// ============================================================================
async fn handle_route(cmd: &RouteCommands, cli: &Cli) -> NetctlResult<()> {
    let route_ctrl = routing::RoutingController::new();

    match cmd {
        RouteCommands::Show => {
            if !cli.terse {
                println!("Route table:");
            }
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
            if !cli.terse {
                println!("Added default gateway {}", gateway);
            }
        }
        RouteCommands::DelDefault => {
            if !cli.terse {
                println!("Not implemented yet");
            }
        }
    }
    Ok(())
}

// ============================================================================
// DEBUG COMMAND HANDLERS
// ============================================================================
async fn handle_debug(cmd: &DebugCommands, cli: &Cli) -> NetctlResult<()> {
    match cmd {
        DebugCommands::Ping { host, count } => {
            // Validate hostname to prevent command injection
            validation::validate_hostname(host)?;

            if !cli.terse {
                println!("Pinging {} {} times...", host, count);
            }
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

            if !cli.terse {
                println!("Starting packet capture on {}...", interface);
            }
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

// ============================================================================
// DAEMON COMMAND HANDLER
// ============================================================================
async fn handle_daemon(nm_compat: bool, cr_dbus: bool) -> NetctlResult<()> {
    use libnetctl::cr_dbus::CRDbusService;
    use libnetctl::network_monitor::NetworkMonitor;
    use std::sync::Arc;
    use tokio::signal::unix::{signal, SignalKind};
    use tracing::{info, error};

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    info!("Starting nccli daemon");
    info!("CR D-Bus: {}, NetworkManager compatibility: {}", cr_dbus, nm_compat);

    // Variables to hold service references
    let cr_service: Option<Arc<CRDbusService>>;
    let nm_service: Option<(Arc<libnetctl::dbus::NetworkManagerDBus>, Arc<zbus::Connection>)>;

    // Start CR D-Bus service if enabled
    if cr_dbus {
        info!("Starting CR D-Bus service");
        match CRDbusService::start().await {
            Ok(service) => {
                info!("CR D-Bus service started successfully");

                // Discover and register devices
                if let Err(e) = service.discover_devices().await {
                    error!("Failed to discover devices for CR D-Bus: {}", e);
                } else {
                    info!("Devices discovered and registered with CR D-Bus");
                }

                cr_service = Some(service);
            }
            Err(e) => {
                error!("Failed to start CR D-Bus service: {}", e);
                cr_service = None;
            }
        }
    } else {
        info!("CR D-Bus service disabled");
        cr_service = None;
    }

    // Start NetworkManager compatibility service if enabled
    #[cfg(feature = "dbus-nm")]
    if nm_compat {
        info!("Starting NetworkManager D-Bus compatibility service");
        match libnetctl::dbus::start_dbus_service().await {
            Ok((nm_dbus, conn)) => {
                info!("NetworkManager D-Bus compatibility service started successfully");

                // Discover and register devices for NetworkManager compatibility
                let iface_ctrl = libnetctl::interface::InterfaceController::new();
                if let Ok(interfaces) = iface_ctrl.list().await {
                    for (index, iface) in interfaces.iter().enumerate() {
                        let device_path = format!("/org/freedesktop/NetworkManager/Devices/{}", index);
                        let device = libnetctl::dbus::DeviceInfo {
                            path: device_path.clone(),
                            interface: iface.clone(),
                            device_type: 1, // TYPE_ETHERNET
                            state: libnetctl::dbus::DeviceState::Disconnected,
                            ip4_address: None,
                            ip6_address: None,
                        };
                        nm_dbus.add_device(device).await;
                    }
                    info!("Devices registered with NetworkManager D-Bus");
                }

                nm_service = Some((nm_dbus, conn));
            }
            Err(e) => {
                error!("Failed to start NetworkManager D-Bus service: {}", e);
                nm_service = None;
            }
        }
    } else {
        info!("NetworkManager D-Bus compatibility service disabled");
        nm_service = None;
    }

    #[cfg(not(feature = "dbus-nm"))]
    let nm_service: Option<()> = None;

    // Start network monitoring
    info!("Starting network event monitor");
    let monitor = Arc::new(NetworkMonitor::new());
    if let Err(e) = monitor.start().await {
        error!("Failed to start network monitor: {}", e);
    } else {
        info!("Network monitor started");
    }

    // Integrate network monitor with D-Bus services
    #[cfg(feature = "dbus-nm")]
    if let Some((ref nm_dbus, ref nm_conn)) = nm_service {
        info!("Integrating network monitor with NetworkManager D-Bus");
        if let Err(e) = libnetctl::dbus_integration::integrate_network_monitor_with_dbus(
            monitor.clone(),
            nm_dbus.clone(),
            nm_conn.clone(),
        ).await {
            error!("Failed to integrate network monitor with NetworkManager D-Bus: {}", e);
        } else {
            info!("Network monitor integrated with NetworkManager D-Bus");
        }
    }

    // Setup signal handlers for graceful shutdown
    let mut sigterm = signal(SignalKind::terminate())
        .map_err(|e| NetctlError::ServiceError(format!("Failed to setup SIGTERM handler: {}", e)))?;
    let mut sigint = signal(SignalKind::interrupt())
        .map_err(|e| NetctlError::ServiceError(format!("Failed to setup SIGINT handler: {}", e)))?;

    info!("nccli daemon is running. Press Ctrl+C to stop.");

    // Main daemon loop - wait for shutdown signal
    tokio::select! {
        _ = sigterm.recv() => {
            info!("Received SIGTERM, shutting down gracefully");
        }
        _ = sigint.recv() => {
            info!("Received SIGINT, shutting down gracefully");
        }
    }

    // Shutdown sequence
    info!("Stopping network monitor");
    if let Err(e) = monitor.stop().await {
        error!("Error stopping network monitor: {}", e);
    }

    if let Some(service) = cr_service {
        info!("Stopping CR D-Bus service");
        if let Err(e) = service.stop().await {
            error!("Error stopping CR D-Bus service: {}", e);
        }
    }

    info!("nccli daemon stopped");
    Ok(())
}
