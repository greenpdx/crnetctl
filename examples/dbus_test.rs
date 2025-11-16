//! Standalone D-Bus Test Program
//!
//! This program exercises the CR D-Bus interface in both real mode (connecting
//! to an actual D-Bus service) and mock mode (running a simulated D-Bus service).
//!
//! # Usage
//!
//! ```bash
//! # Run in mock mode (simulates D-Bus service)
//! cargo run --example dbus_test -- --mode mock
//!
//! # Run in real mode (connects to actual D-Bus service)
//! cargo run --example dbus_test -- --mode real
//!
//! # Run specific test suite
//! cargo run --example dbus_test -- --mode mock --test network-control
//! cargo run --example dbus_test -- --mode mock --test wifi
//! cargo run --example dbus_test -- --mode mock --test vpn
//! cargo run --example dbus_test -- --mode mock --test all
//! ```

use netctl::cr_dbus::{
    CRDbusService,
    CRNetworkState, CRConnectivity, CRDeviceType, CRDeviceState,
    CRDeviceInfo, CRAccessPointInfo, CRVpnInfo, CRWiFiSecurity,
    CRWiFiMode, CRVpnType, CRVpnState,
    CR_DBUS_SERVICE, CR_WIFI_PATH,
};
use clap::{Parser, ValueEnum};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error};
use tracing_subscriber;
use zbus::{Connection, dbus_proxy};

/// Test mode
#[derive(Debug, Clone, ValueEnum)]
enum Mode {
    /// Mock mode - runs a simulated D-Bus service
    Mock,
    /// Real mode - connects to actual D-Bus service
    Real,
}

/// Test suite selection
#[derive(Debug, Clone, ValueEnum)]
enum TestSuite {
    /// Test all interfaces
    All,
    /// Test NetworkControl interface only
    NetworkControl,
    /// Test WiFi interface only
    Wifi,
    /// Test VPN interface only
    Vpn,
    /// Test signals only
    Signals,
}

/// Command line arguments
#[derive(Parser, Debug)]
#[command(name = "dbus-test")]
#[command(about = "D-Bus interface test program", long_about = None)]
struct Args {
    /// Operating mode (mock or real)
    #[arg(short, long, value_enum, default_value = "mock")]
    mode: Mode,

    /// Test suite to run
    #[arg(short, long, value_enum, default_value = "all")]
    test: TestSuite,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// D-Bus service name (for real mode)
    #[arg(long, default_value = CR_DBUS_SERVICE)]
    service: String,
}

// ============================================================================
// D-Bus Proxy Definitions (for Real Mode)
// ============================================================================

#[dbus_proxy(
    interface = "org.crrouter.NetworkControl",
    default_service = "org.crrouter.NetworkControl",
    default_path = "/org/crrouter/NetworkControl"
)]
trait NetworkControl {
    /// Get API version
    fn get_version(&self) -> zbus::Result<String>;

    /// Get all network devices
    fn get_devices(&self) -> zbus::Result<Vec<String>>;

    /// Get device by interface name
    fn get_device_by_interface(&self, iface: &str) -> zbus::Result<String>;

    /// Get device information
    fn get_device_info(&self, device_path: &str) -> zbus::Result<HashMap<String, zbus::zvariant::OwnedValue>>;

    /// Activate a device
    fn activate_device(&self, device_path: &str) -> zbus::Result<()>;

    /// Deactivate a device
    fn deactivate_device(&self, device_path: &str) -> zbus::Result<()>;

    /// Get global network state
    fn get_state(&self) -> zbus::Result<u32>;

    /// Get connectivity state
    fn get_connectivity(&self) -> zbus::Result<u32>;

    /// Check connectivity
    fn check_connectivity(&self) -> zbus::Result<u32>;

    /// Get networking enabled state
    fn get_networking_enabled(&self) -> zbus::Result<bool>;

    /// Set networking enabled state
    fn set_networking_enabled_method(&self, enabled: bool) -> zbus::Result<()>;

    /// Get wireless enabled state
    fn get_wireless_enabled(&self) -> zbus::Result<bool>;

    /// Set wireless enabled state
    fn set_wireless_enabled_method(&self, enabled: bool) -> zbus::Result<()>;

    /// Reload configuration
    fn reload(&self) -> zbus::Result<()>;

    // Note: Signals are typically monitored separately in zbus, not called directly
}

#[dbus_proxy(
    interface = "org.crrouter.NetworkControl.WiFi",
    default_service = "org.crrouter.NetworkControl",
    default_path = "/org/crrouter/NetworkControl/WiFi"
)]
trait WiFi {
    /// Get WiFi enabled state
    fn get_enabled(&self) -> zbus::Result<bool>;

    /// Set WiFi enabled state
    fn set_enabled(&self, enabled: bool) -> zbus::Result<()>;

    /// Start a WiFi scan
    fn scan(&self) -> zbus::Result<()>;

    /// Get list of scanned access points
    fn get_access_points(&self) -> zbus::Result<Vec<HashMap<String, zbus::zvariant::OwnedValue>>>;

    /// Get current connected SSID
    fn get_current_ssid(&self) -> zbus::Result<String>;

    /// Connect to a WiFi network
    fn connect(&self, ssid: &str, password: &str, security: u32) -> zbus::Result<()>;

    /// Disconnect from current WiFi network
    fn disconnect(&self) -> zbus::Result<()>;

    /// Start WiFi Access Point mode
    fn start_access_point(&self, ssid: &str, password: &str, channel: u32) -> zbus::Result<()>;

    /// Stop WiFi Access Point mode
    fn stop_access_point(&self) -> zbus::Result<()>;

    /// Get whether scanning is in progress
    fn is_scanning(&self) -> zbus::Result<bool>;

    // Note: Signals are typically monitored separately in zbus, not called directly
}

#[dbus_proxy(
    interface = "org.crrouter.NetworkControl.VPN",
    default_service = "org.crrouter.NetworkControl",
    default_path = "/org/crrouter/NetworkControl/VPN"
)]
trait VPN {
    /// Get list of VPN connections
    fn get_connections(&self) -> zbus::Result<Vec<String>>;

    /// Get VPN connection info
    fn get_connection_info(&self, name: &str) -> zbus::Result<HashMap<String, zbus::zvariant::OwnedValue>>;

    /// Get VPN connection state
    fn get_state(&self, name: &str) -> zbus::Result<u32>;

    /// Connect to OpenVPN
    fn connect_openvpn(&self, name: &str, config_file: &str) -> zbus::Result<()>;

    /// Connect to WireGuard VPN
    fn connect_wireguard(&self, name: &str, config_file: &str) -> zbus::Result<()>;

    /// Connect to IPsec VPN
    fn connect_ipsec(&self, name: &str, remote: &str, auth_method: &str, credentials: HashMap<String, String>) -> zbus::Result<()>;

    /// Connect to Arti/Tor
    fn connect_arti(&self, name: &str, config: HashMap<String, String>) -> zbus::Result<()>;

    /// Disconnect from VPN
    fn disconnect(&self, name: &str) -> zbus::Result<()>;

    // Note: Signals are typically monitored separately in zbus, not called directly
}

// ============================================================================
// Mock Mode Implementation
// ============================================================================

/// Run the test program in mock mode
async fn run_mock_mode(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting D-Bus test in MOCK mode");

    // Start the mock D-Bus service
    info!("Starting mock CR D-Bus service...");
    let service = CRDbusService::start().await?;
    info!("Mock service started successfully");

    // Give the service time to initialize
    sleep(Duration::from_millis(500)).await;

    // Populate the service with mock data
    populate_mock_data(&service).await?;

    // Get connection for testing
    let conn = Connection::system().await?;

    // Run tests
    match args.test {
        TestSuite::All => {
            run_network_control_tests(&conn, &service).await?;
            run_wifi_tests(&conn, &service).await?;
            run_vpn_tests(&conn, &service).await?;
            run_signal_tests(&conn, &service).await?;
        }
        TestSuite::NetworkControl => {
            run_network_control_tests(&conn, &service).await?;
        }
        TestSuite::Wifi => {
            run_wifi_tests(&conn, &service).await?;
        }
        TestSuite::Vpn => {
            run_vpn_tests(&conn, &service).await?;
        }
        TestSuite::Signals => {
            run_signal_tests(&conn, &service).await?;
        }
    }

    info!("All mock mode tests completed successfully!");

    // Keep service running for a bit to allow inspection
    info!("Service will remain running for 5 seconds...");
    sleep(Duration::from_secs(5)).await;

    service.stop().await?;
    info!("Mock service stopped");

    Ok(())
}

/// Populate the mock service with test data
async fn populate_mock_data(service: &CRDbusService) -> Result<(), Box<dyn std::error::Error>> {
    info!("Populating mock service with test data");

    // Add mock devices
    let eth0 = CRDeviceInfo {
        path: "/org/crrouter/NetworkControl/Devices/eth0".to_string(),
        interface: "eth0".to_string(),
        device_type: CRDeviceType::Ethernet,
        state: CRDeviceState::Activated,
        ipv4_address: Some("192.168.1.100".to_string()),
        ipv6_address: Some("fe80::1".to_string()),
        hw_address: Some("00:11:22:33:44:55".to_string()),
        mtu: 1500,
    };

    let wlan0 = CRDeviceInfo {
        path: "/org/crrouter/NetworkControl/Devices/wlan0".to_string(),
        interface: "wlan0".to_string(),
        device_type: CRDeviceType::WiFi,
        state: CRDeviceState::Disconnected,
        ipv4_address: None,
        ipv6_address: None,
        hw_address: Some("AA:BB:CC:DD:EE:FF".to_string()),
        mtu: 1500,
    };

    service.network_control().add_device(eth0.clone()).await;
    service.network_control().add_device(wlan0.clone()).await;

    // Add mock WiFi access points
    let mock_aps = vec![
        CRAccessPointInfo {
            ssid: "TestNetwork1".to_string(),
            bssid: "00:11:22:33:44:55".to_string(),
            strength: 85,
            security: CRWiFiSecurity::Wpa2,
            frequency: 2437,
            mode: CRWiFiMode::Infrastructure,
        },
        CRAccessPointInfo {
            ssid: "TestNetwork2".to_string(),
            bssid: "AA:BB:CC:DD:EE:FF".to_string(),
            strength: 65,
            security: CRWiFiSecurity::Wpa3,
            frequency: 5180,
            mode: CRWiFiMode::Infrastructure,
        },
        CRAccessPointInfo {
            ssid: "OpenNetwork".to_string(),
            bssid: "11:22:33:44:55:66".to_string(),
            strength: 45,
            security: CRWiFiSecurity::None,
            frequency: 2412,
            mode: CRWiFiMode::Infrastructure,
        },
    ];

    service.wifi().update_access_points(mock_aps).await;

    // Add mock VPN connections
    let vpn1 = CRVpnInfo {
        name: "work-vpn".to_string(),
        path: "/org/crrouter/NetworkControl/VPN/work-vpn".to_string(),
        vpn_type: CRVpnType::OpenVpn,
        state: CRVpnState::Disconnected,
        local_ip: None,
        remote_address: Some("vpn.example.com:1194".to_string()),
    };

    let vpn2 = CRVpnInfo {
        name: "home-vpn".to_string(),
        path: "/org/crrouter/NetworkControl/VPN/home-vpn".to_string(),
        vpn_type: CRVpnType::WireGuard,
        state: CRVpnState::Disconnected,
        local_ip: None,
        remote_address: Some("home.example.com:51820".to_string()),
    };

    service.vpn().add_connection(vpn1).await;
    service.vpn().add_connection(vpn2).await;

    // Set initial network state
    service.update_network_state(CRNetworkState::ConnectedGlobal).await?;
    service.update_connectivity(CRConnectivity::Full).await?;

    info!("Mock data populated successfully");
    Ok(())
}

// ============================================================================
// Real Mode Implementation
// ============================================================================

/// Run the test program in real mode
async fn run_real_mode(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting D-Bus test in REAL mode");
    info!("Connecting to D-Bus service: {}", args.service);

    // Connect to system bus
    let conn = Connection::system().await?;

    // Create proxies
    let nc_proxy = NetworkControlProxy::builder(&conn)
        .destination(args.service.as_str())?
        .build()
        .await?;

    let wifi_proxy = WiFiProxy::builder(&conn)
        .destination(args.service.as_str())?
        .build()
        .await?;

    let vpn_proxy = VPNProxy::builder(&conn)
        .destination(args.service.as_str())?
        .build()
        .await?;

    info!("Connected to D-Bus service successfully");

    // Run tests
    match args.test {
        TestSuite::All => {
            test_real_network_control(&nc_proxy).await?;
            test_real_wifi(&wifi_proxy).await?;
            test_real_vpn(&vpn_proxy).await?;
        }
        TestSuite::NetworkControl => {
            test_real_network_control(&nc_proxy).await?;
        }
        TestSuite::Wifi => {
            test_real_wifi(&wifi_proxy).await?;
        }
        TestSuite::Vpn => {
            test_real_vpn(&vpn_proxy).await?;
        }
        TestSuite::Signals => {
            info!("Signal testing in real mode - monitoring for 30 seconds...");
            sleep(Duration::from_secs(30)).await;
        }
    }

    info!("All real mode tests completed successfully!");

    Ok(())
}

/// Test real NetworkControl interface
async fn test_real_network_control(proxy: &NetworkControlProxy<'_>) -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Testing NetworkControl Interface (Real Mode) ===");

    // Get version
    match proxy.get_version().await {
        Ok(version) => info!("✓ API Version: {}", version),
        Err(e) => error!("✗ Failed to get version: {}", e),
    }

    // Get devices
    match proxy.get_devices().await {
        Ok(devices) => {
            info!("✓ Found {} devices", devices.len());
            for device_path in &devices {
                info!("  - {}", device_path);

                // Get device info
                match proxy.get_device_info(device_path).await {
                    Ok(info) => {
                        info!("    Interface: {:?}", info.get("Interface"));
                        info!("    State: {:?}", info.get("State"));
                        info!("    IPv4: {:?}", info.get("IPv4Address"));
                    }
                    Err(e) => warn!("    Failed to get device info: {}", e),
                }
            }
        }
        Err(e) => error!("✗ Failed to get devices: {}", e),
    }

    // Get state
    match proxy.get_state().await {
        Ok(state) => info!("✓ Network State: {}", state),
        Err(e) => error!("✗ Failed to get state: {}", e),
    }

    // Get connectivity
    match proxy.get_connectivity().await {
        Ok(connectivity) => info!("✓ Connectivity: {}", connectivity),
        Err(e) => error!("✗ Failed to get connectivity: {}", e),
    }

    // Get networking enabled
    match proxy.get_networking_enabled().await {
        Ok(enabled) => info!("✓ Networking Enabled: {}", enabled),
        Err(e) => error!("✗ Failed to get networking enabled: {}", e),
    }

    // Get wireless enabled
    match proxy.get_wireless_enabled().await {
        Ok(enabled) => info!("✓ Wireless Enabled: {}", enabled),
        Err(e) => error!("✗ Failed to get wireless enabled: {}", e),
    }

    Ok(())
}

/// Test real WiFi interface
async fn test_real_wifi(proxy: &WiFiProxy<'_>) -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Testing WiFi Interface (Real Mode) ===");

    // Get WiFi enabled state
    match proxy.get_enabled().await {
        Ok(enabled) => info!("✓ WiFi Enabled: {}", enabled),
        Err(e) => error!("✗ Failed to get WiFi enabled: {}", e),
    }

    // Check if scanning
    match proxy.is_scanning().await {
        Ok(scanning) => info!("✓ Scanning in progress: {}", scanning),
        Err(e) => error!("✗ Failed to check scanning state: {}", e),
    }

    // Get access points
    match proxy.get_access_points().await {
        Ok(aps) => {
            info!("✓ Found {} access points", aps.len());
            for (i, ap) in aps.iter().enumerate() {
                info!("  AP {}: SSID={:?}, Strength={:?}",
                    i + 1,
                    ap.get("SSID"),
                    ap.get("Strength")
                );
            }
        }
        Err(e) => error!("✗ Failed to get access points: {}", e),
    }

    // Get current SSID
    match proxy.get_current_ssid().await {
        Ok(ssid) => {
            if ssid.is_empty() {
                info!("✓ Not connected to any network");
            } else {
                info!("✓ Connected to: {}", ssid);
            }
        }
        Err(e) => error!("✗ Failed to get current SSID: {}", e),
    }

    Ok(())
}

/// Test real VPN interface
async fn test_real_vpn(proxy: &VPNProxy<'_>) -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Testing VPN Interface (Real Mode) ===");

    // Get VPN connections
    match proxy.get_connections().await {
        Ok(connections) => {
            info!("✓ Found {} VPN connections", connections.len());
            for conn_name in &connections {
                info!("  - {}", conn_name);

                // Get connection info
                match proxy.get_connection_info(conn_name).await {
                    Ok(info) => {
                        info!("    Type: {:?}", info.get("Type"));
                        info!("    State: {:?}", info.get("State"));
                        info!("    Remote: {:?}", info.get("RemoteAddress"));
                    }
                    Err(e) => warn!("    Failed to get connection info: {}", e),
                }
            }
        }
        Err(e) => error!("✗ Failed to get VPN connections: {}", e),
    }

    Ok(())
}

// ============================================================================
// Mock Mode Test Functions
// ============================================================================

/// Run NetworkControl tests in mock mode
async fn run_network_control_tests(
    conn: &Connection,
    _service: &CRDbusService,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Testing NetworkControl Interface (Mock Mode) ===");

    let proxy = NetworkControlProxy::builder(conn)
        .destination(CR_DBUS_SERVICE)?
        .build()
        .await?;

    // Test GetVersion
    let version = proxy.get_version().await?;
    info!("✓ GetVersion: {}", version);

    // Test GetDevices
    let devices = proxy.get_devices().await?;
    info!("✓ GetDevices: {} devices", devices.len());
    for device_path in &devices {
        info!("  - {}", device_path);

        // Test GetDeviceInfo
        let device_info = proxy.get_device_info(device_path).await?;
        info!("    Interface: {:?}", device_info.get("Interface"));
        info!("    DeviceType: {:?}", device_info.get("DeviceType"));
        info!("    State: {:?}", device_info.get("State"));
        info!("    IPv4Address: {:?}", device_info.get("IPv4Address"));
        info!("    HwAddress: {:?}", device_info.get("HwAddress"));
    }

    // Test GetState
    let state = proxy.get_state().await?;
    info!("✓ GetState: {}", state);

    // Test GetConnectivity
    let connectivity = proxy.get_connectivity().await?;
    info!("✓ GetConnectivity: {}", connectivity);

    // Test CheckConnectivity
    let checked_connectivity = proxy.check_connectivity().await?;
    info!("✓ CheckConnectivity: {}", checked_connectivity);

    // Test GetNetworkingEnabled
    let networking_enabled = proxy.get_networking_enabled().await?;
    info!("✓ GetNetworkingEnabled: {}", networking_enabled);

    // Test GetWirelessEnabled
    let wireless_enabled = proxy.get_wireless_enabled().await?;
    info!("✓ GetWirelessEnabled: {}", wireless_enabled);

    // Test device lookup by interface
    match proxy.get_device_by_interface("eth0").await {
        Ok(device_path) => info!("✓ GetDeviceByInterface('eth0'): {}", device_path),
        Err(e) => warn!("✗ GetDeviceByInterface failed: {}", e),
    }

    info!("NetworkControl tests completed\n");
    Ok(())
}

/// Run WiFi tests in mock mode
async fn run_wifi_tests(
    conn: &Connection,
    _service: &CRDbusService,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Testing WiFi Interface (Mock Mode) ===");

    let proxy = WiFiProxy::builder(conn)
        .destination(CR_DBUS_SERVICE)?
        .path(CR_WIFI_PATH)?
        .build()
        .await?;

    // Test GetEnabled
    let enabled = proxy.get_enabled().await?;
    info!("✓ GetEnabled: {}", enabled);

    // Test IsScanning
    let scanning = proxy.is_scanning().await?;
    info!("✓ IsScanning: {}", scanning);

    // Test GetAccessPoints
    let aps = proxy.get_access_points().await?;
    info!("✓ GetAccessPoints: {} access points", aps.len());
    for (i, ap) in aps.iter().enumerate() {
        info!("  AP {}:", i + 1);
        info!("    SSID: {:?}", ap.get("SSID"));
        info!("    BSSID: {:?}", ap.get("BSSID"));
        info!("    Strength: {:?}", ap.get("Strength"));
        info!("    Security: {:?}", ap.get("Security"));
        info!("    Frequency: {:?}", ap.get("Frequency"));
    }

    // Test GetCurrentSSID
    let current_ssid = proxy.get_current_ssid().await?;
    if current_ssid.is_empty() {
        info!("✓ GetCurrentSSID: Not connected");
    } else {
        info!("✓ GetCurrentSSID: {}", current_ssid);
    }

    // Test Scan (just triggers, doesn't wait)
    proxy.scan().await?;
    info!("✓ Scan: Scan initiated");

    info!("WiFi tests completed\n");
    Ok(())
}

/// Run VPN tests in mock mode
async fn run_vpn_tests(
    conn: &Connection,
    _service: &CRDbusService,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Testing VPN Interface (Mock Mode) ===");

    let proxy = VPNProxy::builder(conn)
        .destination(CR_DBUS_SERVICE)?
        .path("/org/crrouter/NetworkControl/VPN")?
        .build()
        .await?;

    // Test GetConnections
    let connections = proxy.get_connections().await?;
    info!("✓ GetConnections: {} VPN connections", connections.len());
    for conn_name in &connections {
        info!("  VPN: {}", conn_name);

        // Test GetConnectionInfo
        match proxy.get_connection_info(conn_name).await {
            Ok(info) => {
                info!("    Type: {:?}", info.get("Type"));
                info!("    State: {:?}", info.get("State"));
                info!("    RemoteAddress: {:?}", info.get("RemoteAddress"));
            }
            Err(e) => warn!("    Failed to get connection info: {}", e),
        }
    }

    info!("VPN tests completed\n");
    Ok(())
}

/// Run signal monitoring tests
async fn run_signal_tests(
    _conn: &Connection,
    service: &CRDbusService,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("=== Testing D-Bus Signals (Mock Mode) ===");

    // Trigger various state changes to emit signals
    info!("Triggering network state changes...");

    // Change network state
    service.update_network_state(CRNetworkState::Initializing).await?;
    sleep(Duration::from_millis(200)).await;

    service.update_network_state(CRNetworkState::ConnectedGlobal).await?;
    sleep(Duration::from_millis(200)).await;

    // Change connectivity
    service.update_connectivity(CRConnectivity::Limited).await?;
    sleep(Duration::from_millis(200)).await;

    service.update_connectivity(CRConnectivity::Full).await?;
    sleep(Duration::from_millis(200)).await;

    // Update device state
    service.update_device_state("wlan0", CRDeviceState::Preparing).await?;
    sleep(Duration::from_millis(200)).await;

    service.update_device_state("wlan0", CRDeviceState::Activated).await?;
    sleep(Duration::from_millis(200)).await;

    // Simulate WiFi connection
    service.wifi().set_current_ssid(Some("TestNetwork1".to_string())).await;
    if let Err(e) = netctl::cr_dbus::wifi::signals::emit_connected(
        service.connection().as_ref(),
        "TestNetwork1"
    ).await {
        warn!("Failed to emit WiFi connected signal: {}", e);
    }
    sleep(Duration::from_millis(200)).await;

    // Update VPN state
    service.update_vpn_state("work-vpn", CRVpnState::Connecting).await?;
    sleep(Duration::from_millis(200)).await;

    service.update_vpn_state("work-vpn", CRVpnState::Connected).await?;
    sleep(Duration::from_millis(200)).await;

    info!("✓ All signals emitted successfully");
    info!("Signal tests completed\n");

    Ok(())
}

// ============================================================================
// Main Entry Point
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("dbus_test={},netctl={}", log_level, log_level))
        .init();

    info!("D-Bus Test Program");
    info!("==================");
    info!("Mode: {:?}", args.mode);
    info!("Test Suite: {:?}", args.test);
    info!("");

    // Run tests based on mode
    match args.mode {
        Mode::Mock => {
            run_mock_mode(&args).await?;
        }
        Mode::Real => {
            run_real_mode(&args).await?;
        }
    }

    info!("Test program completed successfully!");
    Ok(())
}
