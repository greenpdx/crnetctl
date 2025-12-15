//! Network Control Daemon (netctld)
//!
//! This daemon provides D-Bus control for network operations.
//! It exposes the CR D-Bus interfaces for network, WiFi, VPN, connection,
//! DHCP, DNS, and routing management.
//!
//! # Usage
//!
//! ```bash
//! # Start the daemon (requires root/sudo)
//! sudo netctld
//!
//! # Start with verbose logging
//! sudo netctld --verbose
//!
//! # Run in foreground (don't daemonize)
//! sudo netctld --foreground
//! ```

use clap::Parser;
use libnetctl::cr_dbus::CRDbusService;
use libnetctl::dbus::start_dbus_service as start_nm_dbus_service;
use libnetctl::error::{NetctlError, NetctlResult};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use tracing_subscriber::{EnvFilter, fmt};

/// Network Control Daemon
#[derive(Parser, Debug)]
#[command(name = "netctld")]
#[command(author = "netctl contributors")]
#[command(version)]
#[command(about = "Network Control Daemon - provides D-Bus interface for network management", long_about = None)]
struct Args {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Run in foreground (don't daemonize)
    #[arg(short, long)]
    foreground: bool,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Disable device discovery on startup
    #[arg(long)]
    no_discovery: bool,
}

/// Shared state for signal handling
struct DaemonState {
    /// Whether the daemon should continue running
    running: Arc<RwLock<bool>>,
}

impl DaemonState {
    fn new() -> Self {
        Self {
            running: Arc::new(RwLock::new(true)),
        }
    }

    async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        info!("Daemon stop requested");
    }
}

#[tokio::main]
async fn main() -> NetctlResult<()> {
    let args = Args::parse();

    // Initialize logging
    init_logging(&args);

    info!("Starting Network Control Daemon (netctld)");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Check if running as root
    #[cfg(target_os = "linux")]
    {
        let uid = unsafe { libc::getuid() };
        if uid != 0 {
            warn!("⚠️  Not running as root - some operations may fail");
            warn!("   Consider running with sudo for full functionality");
        }
    }

    // Create daemon state for signal handling
    let state = Arc::new(DaemonState::new());
    let state_clone = state.clone();

    // Setup signal handlers
    tokio::spawn(async move {
        if let Err(e) = handle_signals(state_clone).await {
            error!("Signal handler error: {}", e);
        }
    });

    // Start the CR D-Bus service
    info!("Initializing CR D-Bus service...");
    let service = match CRDbusService::start().await {
        Ok(svc) => {
            info!("✓ CR D-Bus service started successfully");
            svc
        }
        Err(e) => {
            error!("✗ Failed to start CR D-Bus service: {}", e);
            error!("  This may be due to:");
            error!("  - Another instance already running");
            error!("  - Insufficient permissions (try running as root)");
            error!("  - D-Bus system bus not available");
            return Err(e);
        }
    };

    // Start the NetworkManager compatibility D-Bus service
    info!("Initializing NetworkManager compatibility D-Bus service...");
    let _nm_service = match start_nm_dbus_service().await {
        Ok((nm_dbus, nm_conn)) => {
            info!("✓ NetworkManager D-Bus compatibility service started");
            Some((nm_dbus, nm_conn))
        }
        Err(e) => {
            warn!("⚠️  Failed to start NM compatibility service: {}", e);
            warn!("   Applications expecting org.freedesktop.NetworkManager may not work");
            None
        }
    };

    // Discover network devices unless disabled
    if !args.no_discovery {
        info!("Discovering network devices...");
        match service.discover_devices().await {
            Ok(_) => {
                info!("✓ Device discovery completed");
            }
            Err(e) => {
                warn!("⚠️  Device discovery failed: {}", e);
                warn!("   Continuing without initial device discovery");
            }
        }
    } else {
        info!("Device discovery disabled (--no-discovery)");
    }

    // Start network event monitoring
    info!("Starting network event monitor...");
    match service.start_network_monitor().await {
        Ok(_) => {
            info!("✓ Network event monitor started");
        }
        Err(e) => {
            warn!("⚠️  Failed to start network monitor: {}", e);
            warn!("   Continuing without network event monitoring");
        }
    }

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("  Network Control Daemon is ready");
    info!("  D-Bus Services:");
    info!("    • org.crrouter.NetworkControl");
    info!("    • org.freedesktop.NetworkManager (compatibility)");
    info!("  Interfaces available:");
    info!("    • Network Control   (/org/crrouter/NetworkControl)");
    info!("    • WiFi             (/org/crrouter/NetworkControl/WiFi)");
    info!("    • VPN              (/org/crrouter/NetworkControl/VPN)");
    info!("    • Connection       (/org/crrouter/NetworkControl/Connection)");
    info!("    • DHCP             (/org/crrouter/NetworkControl/DHCP)");
    info!("    • DNS              (/org/crrouter/NetworkControl/DNS)");
    info!("    • Routing          (/org/crrouter/NetworkControl/Routing)");
    info!("    • Privilege        (/org/crrouter/NetworkControl/Privilege)");
    info!("  Features:");
    info!("    • Network event monitoring (link up/down)");
    info!("    • Auto-DHCP on configured interfaces");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Main daemon loop
    while state.is_running().await && service.is_running().await {
        // Sleep for a bit to avoid busy-waiting
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    // Cleanup
    info!("Shutting down Network Control Daemon...");
    if let Err(e) = service.stop().await {
        error!("Error during shutdown: {}", e);
    }

    info!("Network Control Daemon stopped");
    Ok(())
}

/// Initialize logging based on command-line arguments
fn init_logging(args: &Args) {
    let log_level = if args.verbose {
        "debug"
    } else {
        &args.log_level
    };

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::new(format!("netctl={},netctld={},libnetctl={}", log_level, log_level, log_level))
        });

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .with_ansi(atty::is(atty::Stream::Stdout))
        .init();
}

/// Handle Unix signals (SIGTERM, SIGINT, SIGHUP)
async fn handle_signals(state: Arc<DaemonState>) -> NetctlResult<()> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm = signal(SignalKind::terminate())
            .map_err(|e| NetctlError::ServiceError(format!("Failed to register SIGTERM handler: {}", e)))?;
        let mut sigint = signal(SignalKind::interrupt())
            .map_err(|e| NetctlError::ServiceError(format!("Failed to register SIGINT handler: {}", e)))?;
        let mut sighup = signal(SignalKind::hangup())
            .map_err(|e| NetctlError::ServiceError(format!("Failed to register SIGHUP handler: {}", e)))?;

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM, initiating graceful shutdown");
                state.stop().await;
            }
            _ = sigint.recv() => {
                info!("Received SIGINT (Ctrl+C), initiating graceful shutdown");
                state.stop().await;
            }
            _ = sighup.recv() => {
                info!("Received SIGHUP, reloading configuration");
                // For now, just log it - configuration reloading can be implemented later
            }
        }
    }

    #[cfg(not(unix))]
    {
        use tokio::signal;

        // On non-Unix platforms, just wait for Ctrl+C
        signal::ctrl_c().await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to listen for Ctrl+C: {}", e)))?;
        info!("Received Ctrl+C, initiating graceful shutdown");
        state.stop().await;
    }

    Ok(())
}
