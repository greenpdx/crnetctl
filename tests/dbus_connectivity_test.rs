//! Basic D-Bus connectivity tests
//!
//! Tests that verify we can connect to the D-Bus system and session buses

use zbus::Connection;

#[tokio::test]
async fn test_system_bus_connection() {
    // Attempt to connect to the system D-Bus
    let result = Connection::system().await;

    match result {
        Ok(conn) => {
            println!("✓ Successfully connected to system D-Bus");

            // Verify we can get the unique name
            let unique_name = conn.unique_name();
            println!("  Unique name: {:?}", unique_name);
            assert!(unique_name.is_some());
        }
        Err(e) => {
            // Connection might fail in CI or containerized environments
            eprintln!("⚠ Could not connect to system D-Bus: {}", e);
            eprintln!("  This is expected in some environments (CI, containers)");
        }
    }
}

#[tokio::test]
async fn test_session_bus_connection() {
    // Attempt to connect to the session D-Bus
    let result = Connection::session().await;

    match result {
        Ok(conn) => {
            println!("✓ Successfully connected to session D-Bus");

            // Verify we can get the unique name
            let unique_name = conn.unique_name();
            println!("  Unique name: {:?}", unique_name);
            assert!(unique_name.is_some());
        }
        Err(e) => {
            // Connection might fail if no session bus is available
            eprintln!("⚠ Could not connect to session D-Bus: {}", e);
            eprintln!("  This is expected when no session bus is running");
        }
    }
}

#[tokio::test]
async fn test_dbus_introspection() {
    // Try to introspect the D-Bus daemon itself
    if let Ok(conn) = Connection::system().await {
        println!("✓ Testing D-Bus introspection capabilities");

        // Try to call a basic D-Bus method
        let result = conn
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "ListNames",
                &(),
            )
            .await;

        match result {
            Ok(reply) => {
                println!("  Successfully called ListNames method");
                let names: Vec<String> = reply.body().deserialize().unwrap_or_default();
                println!("  Found {} services on the bus", names.len());
                assert!(!names.is_empty(), "D-Bus should have at least one service");
            }
            Err(e) => {
                eprintln!("  Could not call ListNames: {}", e);
            }
        }
    }
}

#[tokio::test]
async fn test_networkmanager_presence() {
    // Check if NetworkManager is available on the system D-Bus
    if let Ok(conn) = Connection::system().await {
        let result = conn
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "ListNames",
                &(),
            )
            .await;

        if let Ok(reply) = result {
            let names: Vec<String> = reply.body().deserialize().unwrap_or_default();

            if names.contains(&"org.freedesktop.NetworkManager".to_string()) {
                println!("✓ NetworkManager is available on D-Bus");
            } else {
                println!("⚠ NetworkManager is not running");
            }
        }
    }
}
