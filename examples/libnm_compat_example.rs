//! Example demonstrating the libnm-compatible CR API
//!
//! This example shows how to use the CR* API which provides
//! NetworkManager libnm-compatible functionality.

use netctl::{CRClient, CRDeviceType, CRDeviceState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the CR client (similar to nm_client_new)
    println!("Creating CRClient...");
    let client = CRClient::new().await?;

    // Get version
    println!("CRClient version: {}", client.get_version());

    // Check if networking is running
    println!("Networking running: {}", client.get_nm_running());

    // Get all devices (similar to nm_client_get_devices)
    println!("\nEnumerating network devices...");
    let devices = client.get_devices().await?;

    for device in &devices {
        println!("\nDevice: {}", device.get_iface());
        println!("  Type: {:?}", device.get_device_type());
        println!("  State: {:?}", device.get_state());

        if let Some(mac) = device.get_hw_address() {
            println!("  MAC Address: {}", mac);
        }

        if let Some(driver) = device.get_driver() {
            println!("  Driver: {}", driver);
        }

        println!("  MTU: {}", device.get_mtu());
        println!("  Managed: {}", device.get_managed());
        println!("  Autoconnect: {}", device.get_autoconnect());

        // Show capabilities
        let caps = device.get_capabilities();
        println!("  Capabilities:");
        println!("    NM Supported: {}", caps.nm_supported);
        println!("    Carrier Detect: {}", caps.carrier_detect);
        println!("    Software Device: {}", caps.is_software);

        // Get IP configuration if device is active
        if device.get_state() == CRDeviceState::Activated {
            if let Some(ip4_config) = device.get_ip4_config().await {
                println!("  IPv4 Configuration:");
                for addr in ip4_config.get_addresses() {
                    println!("    Address: {}", addr.to_cidr());
                }
                if let Some(gw) = ip4_config.get_gateway() {
                    println!("    Gateway: {}", gw);
                }
                for ns in ip4_config.get_nameservers() {
                    println!("    DNS: {}", ns);
                }
            }

            if let Some(ip6_config) = device.get_ip6_config().await {
                println!("  IPv6 Configuration:");
                for addr in ip6_config.get_addresses() {
                    println!("    Address: {}", addr.to_cidr());
                }
            }
        }

        // For WiFi devices, list available access points
        if device.get_device_type() == CRDeviceType::Wifi {
            println!("  Scanning for WiFi access points...");
            match device.wifi_get_access_points().await {
                Ok(aps) => {
                    for ap in aps.iter().take(5) {
                        println!("    SSID: {}", ap.get_ssid_string());
                        println!("      BSSID: {}", ap.get_bssid());
                        println!("      Frequency: {} MHz", ap.get_frequency());
                        println!("      Channel: {}", ap.get_channel());
                        println!("      Strength: {}%", ap.get_strength());
                        println!("      Security: {}", ap.get_security_type());
                    }
                }
                Err(e) => println!("    Failed to scan: {}", e),
            }
        }
    }

    // Get active connections
    println!("\nActive Connections:");
    let active_conns = client.get_active_connections().await?;
    for conn in &active_conns {
        println!("  {}", conn.get_id());
        println!("    UUID: {}", conn.get_uuid());
        println!("    Type: {}", conn.get_connection_type());
        println!("    State: {:?}", conn.get_state());
        println!("    Default (IPv4): {}", conn.get_default());
        println!("    Default (IPv6): {}", conn.get_default6());
    }

    // Get network state
    println!("\nNetwork State: {:?}", client.get_state().await);
    println!("Connectivity: {:?}", client.get_connectivity().await);

    // Example: Get a specific device by name
    if let Some(device) = client.get_device_by_iface("eth0").await? {
        println!("\nFound eth0 device:");
        println!("  State: {:?}", device.get_state());
    }

    println!("\nDone!");
    Ok(())
}
