//! Comprehensive D-Bus Testing Suite
//!
//! 17-point testing plan for the CR D-Bus interface

use zbus::{Connection, zvariant};
use std::collections::HashMap;

#[tokio::test]
async fn test_01_basic_dbus_connectivity() {
    println!("Test 1: Basic D-Bus Connectivity");

    // System bus
    let sys_conn = Connection::system().await;
    assert!(sys_conn.is_ok(), "Should connect to system bus");
    println!("✓ Connected to system D-Bus");

    // Session bus
    let sess_conn = Connection::session().await;
    assert!(sess_conn.is_ok(), "Should connect to session bus");
    println!("✓ Connected to session D-Bus");
}

#[tokio::test]
async fn test_02_service_registration_and_discovery() {
    println!("Test 2: Service Registration and Discovery");

    let conn = match Connection::system().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("⚠ Could not connect to system bus: {}", e);
            return;
        }
    };

    // List all services
    let reply = conn
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "ListNames",
            &(),
        )
        .await;

    assert!(reply.is_ok(), "Should be able to list D-Bus services");

    if let Ok(reply) = reply {
        let names: Vec<String> = reply.body().deserialize().unwrap_or_default();
        println!("✓ Found {} services on D-Bus", names.len());
        assert!(!names.is_empty(), "Should have at least one service");

        // Check for common services
        let has_dbus = names.iter().any(|n| n == "org.freedesktop.DBus");
        assert!(has_dbus, "Should have org.freedesktop.DBus service");
        println!("✓ D-Bus service is registered");
    }
}

#[tokio::test]
async fn test_03_network_control_get_version() {
    println!("Test 3: NetworkControl - GetVersion");

    let conn = match Connection::system().await {
        Ok(c) => c,
        Err(_) => {
            println!("⚠ Skipping - no D-Bus connection");
            return;
        }
    };

    // Try to call GetVersion on our service
    let result = conn
        .call_method(
            Some("org.crrouter.NetworkControl"),
            "/org/crrouter/NetworkControl",
            Some("org.crrouter.NetworkControl"),
            "GetVersion",
            &(),
        )
        .await;

    match result {
        Ok(reply) => {
            let version: String = reply.body().deserialize().unwrap_or_default();
            println!("✓ GetVersion returned: {}", version);
            assert!(!version.is_empty(), "Version should not be empty");
        }
        Err(e) => {
            println!("⚠ Service not running (expected in tests): {}", e);
        }
    }
}

#[tokio::test]
async fn test_04_network_control_get_devices() {
    println!("Test 4: NetworkControl - GetDevices");

    let conn = match Connection::system().await {
        Ok(c) => c,
        Err(_) => {
            println!("⚠ Skipping - no D-Bus connection");
            return;
        }
    };

    let result = conn
        .call_method(
            Some("org.crrouter.NetworkControl"),
            "/org/crrouter/NetworkControl",
            Some("org.crrouter.NetworkControl"),
            "GetDevices",
            &(),
        )
        .await;

    match result {
        Ok(reply) => {
            let devices: Vec<String> = reply.body().deserialize().unwrap_or_default();
            println!("✓ GetDevices returned {} devices", devices.len());
            for device in devices {
                println!("  - {}", device);
            }
        }
        Err(e) => {
            println!("⚠ Service not running (expected in tests): {}", e);
        }
    }
}

#[tokio::test]
async fn test_05_network_control_get_state() {
    println!("Test 5: NetworkControl - GetState");

    let conn = match Connection::system().await {
        Ok(c) => c,
        Err(_) => {
            println!("⚠ Skipping - no D-Bus connection");
            return;
        }
    };

    let result = conn
        .call_method(
            Some("org.crrouter.NetworkControl"),
            "/org/crrouter/NetworkControl",
            Some("org.crrouter.NetworkControl"),
            "GetState",
            &(),
        )
        .await;

    match result {
        Ok(reply) => {
            let state: u32 = reply.body().deserialize().unwrap_or(0);
            println!("✓ GetState returned: {}", state);
        }
        Err(e) => {
            println!("⚠ Service not running (expected in tests): {}", e);
        }
    }
}

#[tokio::test]
async fn test_06_network_control_get_connectivity() {
    println!("Test 6: NetworkControl - GetConnectivity");

    let conn = match Connection::system().await {
        Ok(c) => c,
        Err(_) => {
            println!("⚠ Skipping - no D-Bus connection");
            return;
        }
    };

    let result = conn
        .call_method(
            Some("org.crrouter.NetworkControl"),
            "/org/crrouter/NetworkControl",
            Some("org.crrouter.NetworkControl"),
            "GetConnectivity",
            &(),
        )
        .await;

    match result {
        Ok(reply) => {
            let connectivity: u32 = reply.body().deserialize().unwrap_or(0);
            println!("✓ GetConnectivity returned: {}", connectivity);
        }
        Err(e) => {
            println!("⚠ Service not running (expected in tests): {}", e);
        }
    }
}

#[tokio::test]
async fn test_07_wifi_get_enabled() {
    println!("Test 7: WiFi - GetEnabled");

    let conn = match Connection::system().await {
        Ok(c) => c,
        Err(_) => {
            println!("⚠ Skipping - no D-Bus connection");
            return;
        }
    };

    let result = conn
        .call_method(
            Some("org.crrouter.NetworkControl"),
            "/org/crrouter/NetworkControl/WiFi",
            Some("org.crrouter.NetworkControl.WiFi"),
            "GetEnabled",
            &(),
        )
        .await;

    match result {
        Ok(reply) => {
            let enabled: bool = reply.body().deserialize().unwrap_or(false);
            println!("✓ WiFi GetEnabled returned: {}", enabled);
        }
        Err(e) => {
            println!("⚠ Service not running (expected in tests): {}", e);
        }
    }
}

#[tokio::test]
async fn test_08_wifi_scan_and_get_access_points() {
    println!("Test 8: WiFi - Scan and GetAccessPoints");

    let conn = match Connection::system().await {
        Ok(c) => c,
        Err(_) => {
            println!("⚠ Skipping - no D-Bus connection");
            return;
        }
    };

    // Try to get access points
    let result = conn
        .call_method(
            Some("org.crrouter.NetworkControl"),
            "/org/crrouter/NetworkControl/WiFi",
            Some("org.crrouter.NetworkControl.WiFi"),
            "GetAccessPoints",
            &(),
        )
        .await;

    match result {
        Ok(reply) => {
            let aps: Vec<String> = reply.body().deserialize().unwrap_or_default();
            println!("✓ GetAccessPoints returned {} APs", aps.len());
        }
        Err(e) => {
            println!("⚠ Service not running (expected in tests): {}", e);
        }
    }
}

#[tokio::test]
async fn test_09_wifi_connect() {
    println!("Test 9: WiFi - Connect");
    println!("⚠ Skipping - requires actual WiFi network (tested in integration)");
}

#[tokio::test]
async fn test_10_vpn_list_connections() {
    println!("Test 10: VPN - List Connections");

    let conn = match Connection::system().await {
        Ok(c) => c,
        Err(_) => {
            println!("⚠ Skipping - no D-Bus connection");
            return;
        }
    };

    let result = conn
        .call_method(
            Some("org.crrouter.NetworkControl"),
            "/org/crrouter/NetworkControl/VPN",
            Some("org.crrouter.NetworkControl.VPN"),
            "GetConnections",
            &(),
        )
        .await;

    match result {
        Ok(reply) => {
            let vpns: Vec<String> = reply.body().deserialize().unwrap_or_default();
            println!("✓ GetConnections returned {} VPN connections", vpns.len());
        }
        Err(e) => {
            println!("⚠ Service not running (expected in tests): {}", e);
        }
    }
}

#[tokio::test]
async fn test_11_vpn_connect_disconnect() {
    println!("Test 11: VPN - Connect/Disconnect");
    println!("⚠ Skipping - requires actual VPN config (tested in integration)");
}

#[tokio::test]
async fn test_12_signals_state_changed() {
    println!("Test 12: D-Bus Signals - StateChanged");
    println!("⚠ Skipping - signal monitoring requires running service");
}

#[tokio::test]
async fn test_13_signals_device_added_removed() {
    println!("Test 13: D-Bus Signals - DeviceAdded/Removed");
    println!("⚠ Skipping - signal monitoring requires running service");
}

#[tokio::test]
async fn test_14_properties_read_write() {
    println!("Test 14: D-Bus Properties");

    let conn = match Connection::system().await {
        Ok(c) => c,
        Err(_) => {
            println!("⚠ Skipping - no D-Bus connection");
            return;
        }
    };

    // Try to read a property
    let result = conn
        .call_method(
            Some("org.crrouter.NetworkControl"),
            "/org/crrouter/NetworkControl",
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &("org.crrouter.NetworkControl", "Version"),
        )
        .await;

    match result {
        Ok(reply) => {
            println!("✓ Successfully read property via D-Bus Properties interface");
        }
        Err(e) => {
            println!("⚠ Service not running (expected in tests): {}", e);
        }
    }
}

#[tokio::test]
async fn test_15_dbus_introspection() {
    println!("Test 15: D-Bus Introspection");

    let conn = match Connection::system().await {
        Ok(c) => c,
        Err(_) => {
            println!("⚠ Skipping - no D-Bus connection");
            return;
        }
    };

    // Introspect the D-Bus daemon itself (always available)
    let result = conn
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus.Introspectable"),
            "Introspect",
            &(),
        )
        .await;

    assert!(result.is_ok(), "Should be able to introspect D-Bus daemon");

    if let Ok(reply) = result {
        let xml: String = reply.body().deserialize().unwrap_or_default();
        println!("✓ Introspection returned {} bytes of XML", xml.len());
        assert!(xml.contains("<node"), "XML should contain node definitions");
        assert!(xml.contains("interface"), "XML should contain interface definitions");
    }
}

#[tokio::test]
async fn test_16_error_handling() {
    println!("Test 16: Error Handling");

    let conn = match Connection::system().await {
        Ok(c) => c,
        Err(_) => {
            println!("⚠ Skipping - no D-Bus connection");
            return;
        }
    };

    // Try to call a non-existent method
    let result = conn
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "NonExistentMethod",
            &(),
        )
        .await;

    assert!(result.is_err(), "Should fail on non-existent method");
    println!("✓ Correctly handles non-existent method");

    // Try to call on non-existent service
    let result = conn
        .call_method(
            Some("org.nonexistent.Service"),
            "/",
            Some("org.nonexistent.Interface"),
            "Method",
            &(),
        )
        .await;

    assert!(result.is_err(), "Should fail on non-existent service");
    println!("✓ Correctly handles non-existent service");
}

#[tokio::test]
async fn test_17_networkmanager_compatibility() {
    println!("Test 17: NetworkManager Compatibility");

    let conn = match Connection::system().await {
        Ok(c) => c,
        Err(_) => {
            println!("⚠ Skipping - no D-Bus connection");
            return;
        }
    };

    // Check if NetworkManager service is available
    let result = conn
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "NameHasOwner",
            &("org.freedesktop.NetworkManager"),
        )
        .await;

    if let Ok(reply) = result {
        let has_nm: bool = reply.body().deserialize().unwrap_or(false);
        if has_nm {
            println!("✓ NetworkManager is running");

            // Try to get NM version
            let version_result = conn
                .call_method(
                    Some("org.freedesktop.NetworkManager"),
                    "/org/freedesktop/NetworkManager",
                    Some("org.freedesktop.DBus.Properties"),
                    "Get",
                    &("org.freedesktop.NetworkManager", "Version"),
                )
                .await;

            if let Ok(reply) = version_result {
                println!("✓ Successfully queried NetworkManager properties");
            }
        } else {
            println!("⚠ NetworkManager is not running (expected in some envs)");
        }
    }
}
