//! Standalone D-Bus connectivity test
//!
//! This test verifies D-Bus connectivity without depending on netctl library

use zbus::Connection;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== D-Bus Connectivity Test ===\n");

    // Test 1: System Bus Connection
    println!("Test 1: Connecting to System D-Bus...");
    match Connection::system().await {
        Ok(conn) => {
            println!("✓ SUCCESS: Connected to system D-Bus");
            if let Some(name) = conn.unique_name() {
                println!("  Unique name: {}", name);
            }
        }
        Err(e) => {
            println!("✗ FAILED: Could not connect to system D-Bus");
            println!("  Error: {}", e);
            println!("  Note: This may be expected in containers or CI environments\n");
        }
    }

    // Test 2: Session Bus Connection
    println!("\nTest 2: Connecting to Session D-Bus...");
    match Connection::session().await {
        Ok(conn) => {
            println!("✓ SUCCESS: Connected to session D-Bus");
            if let Some(name) = conn.unique_name() {
                println!("  Unique name: {}", name);
            }
        }
        Err(e) => {
            println!("✗ FAILED: Could not connect to session D-Bus");
            println!("  Error: {}", e);
            println!("  Note: Session bus may not be available\n");
        }
    }

    // Test 3: List D-Bus Services
    println!("\nTest 3: Listing D-Bus services...");
    if let Ok(conn) = Connection::system().await {
        match conn
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "ListNames",
                &(),
            )
            .await
        {
            Ok(reply) => {
                let names: Vec<String> = reply.body().deserialize()?;
                println!("✓ SUCCESS: Found {} services on system bus", names.len());

                // Check for common network-related services
                println!("\n  Network-related services:");
                for name in &names {
                    if name.contains("Network") || name.contains("network") {
                        println!("    - {}", name);
                    }
                }
            }
            Err(e) => {
                println!("✗ FAILED: Could not list services");
                println!("  Error: {}", e);
            }
        }
    }

    // Test 4: Check for NetworkManager
    println!("\nTest 4: Checking for NetworkManager...");
    if let Ok(conn) = Connection::system().await {
        match conn
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "NameHasOwner",
                &("org.freedesktop.NetworkManager"),
            )
            .await
        {
            Ok(reply) => {
                let has_owner: bool = reply.body().deserialize()?;
                if has_owner {
                    println!("✓ SUCCESS: NetworkManager is running");
                } else {
                    println!("⚠ INFO: NetworkManager is not running");
                }
            }
            Err(e) => {
                println!("✗ FAILED: Could not check for NetworkManager");
                println!("  Error: {}", e);
            }
        }
    }

    // Test 5: Check for our custom service
    println!("\nTest 5: Checking for netctl D-Bus service...");
    if let Ok(conn) = Connection::system().await {
        let service_names = vec![
            "org.crrouter.NetworkControl",
            "org.netctl.NetworkControl",
        ];

        for service in &service_names {
            match conn
                .call_method(
                    Some("org.freedesktop.DBus"),
                    "/org/freedesktop/DBus",
                    Some("org.freedesktop.DBus"),
                    "NameHasOwner",
                    &(*service),
                )
                .await
            {
                Ok(reply) => {
                    let has_owner: bool = reply.body().deserialize()?;
                    if has_owner {
                        println!("✓ SUCCESS: {} is running", service);
                    } else {
                        println!("⚠ INFO: {} is not running", service);
                    }
                }
                Err(e) => {
                    println!("✗ FAILED: Could not check for {}", service);
                    println!("  Error: {}", e);
                }
            }
        }
    }

    println!("\n=== Test Complete ===");
    Ok(())
}
