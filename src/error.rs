//! Error types for netctl

use std::fmt;
use std::io;

#[derive(Debug)]
pub enum NetctlError {
    /// IO error
    Io(io::Error),
    /// Command execution failed
    CommandFailed { cmd: String, code: Option<i32>, stderr: String },
    /// Invalid parameter
    InvalidParameter(String),
    /// Interface not found
    InterfaceNotFound(String),
    /// Device not found
    DeviceNotFound(String),
    /// Configuration error
    ConfigError(String),
    /// Service error (hostapd, dora, unbound)
    ServiceError(String),
    /// Permission denied
    PermissionDenied(String),
    /// Not supported
    NotSupported(String),
    /// Parse error
    ParseError(String),
    /// Already exists
    AlreadyExists(String),
    /// Timeout
    Timeout(String),
    /// Not found
    NotFound(String),
    /// Invalid state
    InvalidState(String),
    /// Connection failed
    ConnectionFailed { reason: String },
}

impl fmt::Display for NetctlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetctlError::Io(e) => write!(f, "IO error: {}", e),
            NetctlError::CommandFailed { cmd, code, stderr } => {
                if let Some(code) = code {
                    write!(f, "Command '{}' failed with code {}: {}", cmd, code, stderr)
                } else {
                    write!(f, "Command '{}' failed: {}", cmd, stderr)
                }
            }
            NetctlError::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
            NetctlError::InterfaceNotFound(name) => write!(f, "Interface not found: {}", name),
            NetctlError::DeviceNotFound(name) => write!(f, "Device not found: {}", name),
            NetctlError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            NetctlError::ServiceError(msg) => write!(f, "Service error: {}", msg),
            NetctlError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            NetctlError::NotSupported(msg) => write!(f, "Not supported: {}", msg),
            NetctlError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            NetctlError::AlreadyExists(msg) => write!(f, "Already exists: {}", msg),
            NetctlError::Timeout(msg) => write!(f, "Timeout: {}", msg),
            NetctlError::NotFound(msg) => write!(f, "Not found: {}", msg),
            NetctlError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            NetctlError::ConnectionFailed { reason } => write!(f, "Connection failed: {}", reason),
        }
    }
}

impl std::error::Error for NetctlError {}

impl From<io::Error> for NetctlError {
    fn from(error: io::Error) -> Self {
        NetctlError::Io(error)
    }
}

impl From<serde_json::Error> for NetctlError {
    fn from(error: serde_json::Error) -> Self {
        NetctlError::ParseError(error.to_string())
    }
}

pub type NetctlResult<T> = Result<T, NetctlError>;
