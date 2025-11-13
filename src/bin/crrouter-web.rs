//! crrouter-web - Web API for CRRouter network management
//!
//! Provides REST API endpoints for network management operations including:
//! - DHCP testing and diagnostics
//! - Interface management
//! - WiFi operations
//! - Access Point control

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use netctl::{
    DhcpTestConfig, DhcpTestResult, DhcpmController, InterfaceController,
    NetctlError, NetctlResult, WifiController,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use tracing_subscriber;

/// Application state shared across handlers
#[derive(Clone)]
struct AppState {
    dhcpm: Arc<DhcpmController>,
    interface: Arc<InterfaceController>,
    wifi: Arc<WifiController>,
}

/// API error response
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    details: Option<String>,
}

/// Wrapper for API errors to implement IntoResponse
struct ApiError(NetctlError);

impl From<NetctlError> for ApiError {
    fn from(err: NetctlError) -> Self {
        ApiError(err)
    }
}

/// Convert NetctlError to HTTP response
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self.0 {
            NetctlError::InvalidParameter(msg) => (StatusCode::BAD_REQUEST, msg),
            NetctlError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            NetctlError::InterfaceNotFound(msg) => (StatusCode::NOT_FOUND, msg),
            NetctlError::DeviceNotFound(msg) => (StatusCode::NOT_FOUND, msg),
            NetctlError::PermissionDenied(msg) => (StatusCode::FORBIDDEN, msg),
            NetctlError::CommandFailed { cmd, code, stderr } => {
                let msg = if let Some(code) = code {
                    format!("Command '{}' failed with code {}: {}", cmd, code, stderr)
                } else {
                    format!("Command '{}' failed: {}", cmd, stderr)
                };
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
            NetctlError::Io(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            NetctlError::ParseError(msg) => (StatusCode::BAD_REQUEST, msg),
            NetctlError::Timeout(msg) => (StatusCode::REQUEST_TIMEOUT, msg),
            NetctlError::AlreadyExists(msg) => (StatusCode::CONFLICT, msg),
            NetctlError::ConfigError(msg) => (StatusCode::BAD_REQUEST, msg),
            NetctlError::ServiceError(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
            NetctlError::NotSupported(msg) => (StatusCode::NOT_IMPLEMENTED, msg),
        };

        let body = Json(ErrorResponse {
            error: error_message,
            details: None,
        });

        (status, body).into_response()
    }
}

/// Health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "crrouter-web",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// API info endpoint
async fn api_info() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "CRRouter Web API",
        "version": env!("CARGO_PKG_VERSION"),
        "endpoints": {
            "health": "/health",
            "api": "/api",
            "dhcp_test": "/api/dhcp/test",
            "dhcp_discover": "/api/dhcp/discover",
            "dhcp_request": "/api/dhcp/request",
            "dhcp_release": "/api/dhcp/release",
            "dhcp_test_sequence": "/api/dhcp/test-sequence/:interface",
            "interfaces": "/api/interfaces",
            "wifi_scan": "/api/wifi/scan/:interface"
        }
    }))
}

// ============================================================================
// DHCP Testing Endpoints
// ============================================================================

/// Request for DHCP discover test
#[derive(Debug, Deserialize)]
struct DhcpDiscoverRequest {
    interface: String,
    #[serde(flatten)]
    config: Option<DhcpTestConfig>,
}

/// Send DHCP discover message
async fn dhcp_discover(
    State(state): State<AppState>,
    Json(req): Json<DhcpDiscoverRequest>,
) -> Result<Json<DhcpTestResult>, ApiError> {
    info!("DHCP discover test on interface: {}", req.interface);

    let mut config = req.config.unwrap_or_else(DhcpTestConfig::default);
    config.interface = req.interface;
    config.message_type = netctl::DhcpMessageType::Discover;

    let result = state.dhcpm.send_discover(&config).await?;
    Ok(Json(result))
}

/// Send DHCP request message
async fn dhcp_request(
    State(state): State<AppState>,
    Json(config): Json<DhcpTestConfig>,
) -> Result<Json<DhcpTestResult>, ApiError> {
    info!("DHCP request test on interface: {}", config.interface);

    let result = state.dhcpm.send_request(&config).await?;
    Ok(Json(result))
}

/// Send DHCP release message
async fn dhcp_release(
    State(state): State<AppState>,
    Json(config): Json<DhcpTestConfig>,
) -> Result<Json<DhcpTestResult>, ApiError> {
    info!("DHCP release test on interface: {}", config.interface);

    let result = state.dhcpm.send_release(&config).await?;
    Ok(Json(result))
}

/// Run comprehensive DHCP test
async fn dhcp_test(
    State(state): State<AppState>,
    Json(config): Json<DhcpTestConfig>,
) -> Result<Json<DhcpTestResult>, ApiError> {
    info!("DHCP test on interface: {}", config.interface);

    // Execute based on message type
    let result = match config.message_type {
        netctl::DhcpMessageType::Discover => state.dhcpm.send_discover(&config).await?,
        netctl::DhcpMessageType::Request => state.dhcpm.send_request(&config).await?,
        netctl::DhcpMessageType::Release => state.dhcpm.send_release(&config).await?,
        netctl::DhcpMessageType::Inform => state.dhcpm.send_inform(&config).await?,
        netctl::DhcpMessageType::Decline => {
            return Err(ApiError(NetctlError::NotSupported(
                "DECLINE message type not yet supported".to_string(),
            )))
        }
    };

    Ok(Json(result))
}

/// Run full DHCP test sequence on interface
async fn dhcp_test_sequence(
    State(state): State<AppState>,
    Path(interface): Path<String>,
) -> Result<Json<Vec<DhcpTestResult>>, ApiError> {
    info!("DHCP test sequence on interface: {}", interface);

    let results = state.dhcpm.run_test_sequence(&interface).await?;
    Ok(Json(results))
}

// ============================================================================
// Interface Management Endpoints
// ============================================================================

/// List all network interfaces
async fn list_interfaces(
    State(state): State<AppState>,
) -> Result<Json<Vec<String>>, ApiError> {
    info!("Listing network interfaces");

    let interfaces = state.interface.list().await?;
    Ok(Json(interfaces))
}

/// Get interface information
async fn get_interface_info(
    State(state): State<AppState>,
    Path(interface): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    info!("Getting info for interface: {}", interface);

    let info = state.interface.get_info(&interface).await?;
    Ok(Json(serde_json::json!({
        "name": info.name,
        "state": info.state,
        "mac_address": info.mac_address,
        "mtu": info.mtu,
        "addresses": info.addresses,
        "flags": info.flags
    })))
}

// ============================================================================
// WiFi Endpoints
// ============================================================================

/// Scan WiFi networks
async fn wifi_scan(
    State(state): State<AppState>,
    Path(interface): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    info!("WiFi scan on interface: {}", interface);

    let results = state.wifi.scan(&interface).await?;

    let networks: Vec<_> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "ssid": r.ssid,
                "bssid": r.bssid,
                "signal": r.signal,
                "frequency": r.frequency,
                "capabilities": r.capabilities
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "interface": interface,
        "networks": networks
    })))
}

// ============================================================================
// Main Application Setup
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Starting CRRouter Web API v{}", env!("CARGO_PKG_VERSION"));

    // Initialize controllers
    let dhcpm = Arc::new(
        DhcpmController::new("eth0".to_string()).map_err(|e| {
            warn!("Failed to initialize DHCP testing controller: {}", e);
            e
        })?,
    );

    let interface = Arc::new(InterfaceController::new());
    let wifi = Arc::new(WifiController::new());

    let state = AppState {
        dhcpm,
        interface,
        wifi,
    };

    // Build router
    let app = Router::new()
        // Health and info
        .route("/health", get(health_check))
        .route("/api", get(api_info))
        // DHCP testing
        .route("/api/dhcp/test", post(dhcp_test))
        .route("/api/dhcp/discover", post(dhcp_discover))
        .route("/api/dhcp/request", post(dhcp_request))
        .route("/api/dhcp/release", post(dhcp_release))
        .route(
            "/api/dhcp/test-sequence/:interface",
            get(dhcp_test_sequence),
        )
        // Interface management
        .route("/api/interfaces", get(list_interfaces))
        .route("/api/interfaces/:interface", get(get_interface_info))
        // WiFi
        .route("/api/wifi/scan/:interface", get(wifi_scan))
        // Add state and middleware
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    // Determine bind address from environment or use default
    let port = std::env::var("CRROUTER_WEB_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    info!("Listening on http://{}", addr);
    info!("API documentation available at http://{}/api", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
