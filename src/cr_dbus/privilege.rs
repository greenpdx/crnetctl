//! CR Privilege Token D-Bus interface
//!
//! D-Bus interface for managing time-limited privilege escalation tokens.
//! This allows users to grant, revoke, and query privilege tokens via D-Bus.

use crate::error::{NetctlError, NetctlResult};
use crate::privilege_token::{PrivilegeToken, revoke_token, has_valid_token};
use std::collections::HashMap;
use tracing::{info, warn, debug};
use zbus::{Connection, fdo, interface};
use zbus::object_server::SignalEmitter;
use zbus::zvariant::Value;

/// CR Privilege Token D-Bus interface
#[derive(Clone, Default)]
pub struct CRPrivilege;

impl CRPrivilege {
    /// Create a new CR Privilege interface
    pub fn new() -> Self {
        Self
    }
}

#[interface(name = "org.crrouter.NetworkControl.Privilege")]
impl CRPrivilege {
    /// Grant a privilege token for time-limited root access
    ///
    /// # Arguments
    /// * `duration_minutes` - How long the token should be valid (in minutes)
    /// * `uid` - Optional UID to restrict the token to (0 = any user)
    ///
    /// # Returns
    /// A dictionary containing token information on success
    ///
    /// # Errors
    /// Returns an error if called by non-root or if token creation fails
    async fn grant_privileges(
        &self,
        duration_minutes: u32,
        uid: u32,
    ) -> fdo::Result<HashMap<String, Value<'_>>> {
        info!("CR: GrantPrivileges called - duration: {} minutes, uid: {}", duration_minutes, uid);

        // Validate duration
        if duration_minutes == 0 {
            return Err(fdo::Error::InvalidArgs("Duration must be greater than 0".to_string()));
        }

        if duration_minutes > 1440 {
            // Max 24 hours
            return Err(fdo::Error::InvalidArgs("Duration cannot exceed 1440 minutes (24 hours)".to_string()));
        }

        // Create the token (the create function checks for root)
        let allowed_uid = if uid == 0 { None } else { Some(uid) };

        match PrivilegeToken::create(duration_minutes, allowed_uid) {
            Ok(token) => {
                let mut result = HashMap::new();
                result.insert("GrantedByUid".to_string(), Value::new(token.granted_by_uid));
                result.insert("CreatedAt".to_string(), Value::new(token.created_at));
                result.insert("DurationMinutes".to_string(), Value::new(token.duration_minutes));
                result.insert("ExpiresAt".to_string(), Value::new(token.expires_at));
                result.insert("AllowedUid".to_string(), Value::new(token.allowed_uid.unwrap_or(0)));
                result.insert("RemainingSeconds".to_string(), Value::new(token.remaining_seconds()));
                result.insert("ExpiryFormatted".to_string(), Value::new(token.format_expiry()));

                info!("CR: Privilege token granted, expires at {}", token.format_expiry());
                Ok(result)
            }
            Err(e) => {
                warn!("CR: Failed to grant privilege token: {}", e);
                Err(fdo::Error::Failed(format!("Failed to grant privileges: {}", e)))
            }
        }
    }

    /// Revoke the current privilege token
    ///
    /// # Returns
    /// true if a token was revoked, false if no token existed
    ///
    /// # Errors
    /// Returns an error if called by non-root
    async fn revoke_privileges(&self) -> fdo::Result<bool> {
        info!("CR: RevokePrivileges called");

        match revoke_token() {
            Ok(_) => {
                info!("CR: Privilege token revoked");
                Ok(true)
            }
            Err(NetctlError::PermissionDenied(msg)) => {
                warn!("CR: Permission denied revoking token: {}", msg);
                Err(fdo::Error::AccessDenied(msg))
            }
            Err(e) => {
                warn!("CR: Failed to revoke token: {}", e);
                Err(fdo::Error::Failed(format!("Failed to revoke privileges: {}", e)))
            }
        }
    }

    /// Get the current privilege token status
    ///
    /// # Returns
    /// A dictionary containing:
    /// - "HasValidToken": bool - whether a valid token exists
    /// - Token details if a valid token exists
    async fn get_privilege_status(&self) -> fdo::Result<HashMap<String, Value<'_>>> {
        debug!("CR: GetPrivilegeStatus called");

        let mut result = HashMap::new();

        match PrivilegeToken::load() {
            Ok(Some(token)) => {
                let is_valid = token.verify().unwrap_or(false);
                result.insert("HasValidToken".to_string(), Value::new(is_valid));

                if is_valid {
                    result.insert("GrantedByUid".to_string(), Value::new(token.granted_by_uid));
                    result.insert("CreatedAt".to_string(), Value::new(token.created_at));
                    result.insert("DurationMinutes".to_string(), Value::new(token.duration_minutes));
                    result.insert("ExpiresAt".to_string(), Value::new(token.expires_at));
                    result.insert("AllowedUid".to_string(), Value::new(token.allowed_uid.unwrap_or(0)));
                    result.insert("RemainingSeconds".to_string(), Value::new(token.remaining_seconds()));
                    result.insert("ExpiryFormatted".to_string(), Value::new(token.format_expiry()));
                }

                Ok(result)
            }
            Ok(None) => {
                result.insert("HasValidToken".to_string(), Value::new(false));
                Ok(result)
            }
            Err(e) => {
                warn!("CR: Failed to get privilege status: {}", e);
                result.insert("HasValidToken".to_string(), Value::new(false));
                result.insert("Error".to_string(), Value::new(format!("{}", e)));
                Ok(result)
            }
        }
    }

    /// Verify if the current user has a valid privilege token
    ///
    /// This performs signature verification (only works when called from netctld as root)
    ///
    /// # Returns
    /// true if a valid, non-expired token exists for the requesting user
    async fn verify_token(&self) -> fdo::Result<bool> {
        debug!("CR: VerifyToken called");

        match PrivilegeToken::load() {
            Ok(Some(token)) => {
                match token.verify() {
                    Ok(valid) => {
                        debug!("CR: Token verification result: {}", valid);
                        Ok(valid)
                    }
                    Err(e) => {
                        warn!("CR: Token verification error: {}", e);
                        Err(fdo::Error::Failed(format!("Token verification failed: {}", e)))
                    }
                }
            }
            Ok(None) => {
                debug!("CR: No token found");
                Ok(false)
            }
            Err(e) => {
                warn!("CR: Failed to load token: {}", e);
                Err(fdo::Error::Failed(format!("Failed to load token: {}", e)))
            }
        }
    }

    /// Check if a valid token exists (quick check without full verification)
    ///
    /// # Returns
    /// true if a valid token exists
    async fn has_valid_token_method(&self) -> bool {
        has_valid_token()
    }

    /// Get remaining time on the current privilege token
    ///
    /// # Returns
    /// Remaining seconds, or 0 if no valid token exists
    async fn get_remaining_time(&self) -> u64 {
        match PrivilegeToken::load() {
            Ok(Some(token)) if token.verify().unwrap_or(false) => {
                token.remaining_seconds()
            }
            _ => 0,
        }
    }

    // ============ D-Bus Signals ============

    /// PrivilegeGranted signal - emitted when a privilege token is granted
    #[zbus(signal)]
    async fn privilege_granted(
        signal_emitter: &SignalEmitter<'_>,
        duration_minutes: u32,
        allowed_uid: u32,
        expires_at: u64,
    ) -> zbus::Result<()>;

    /// PrivilegeRevoked signal - emitted when a privilege token is revoked
    #[zbus(signal)]
    async fn privilege_revoked(signal_emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    /// PrivilegeExpired signal - emitted when a privilege token expires
    #[zbus(signal)]
    async fn privilege_expired(signal_emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

/// Helper module for emitting privilege signals
pub mod signals {
    use super::*;
    use super::super::types::CR_PRIVILEGE_PATH;

    /// Emit PrivilegeGranted signal
    pub async fn emit_privilege_granted(
        conn: &Connection,
        duration_minutes: u32,
        allowed_uid: u32,
        expires_at: u64,
    ) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRPrivilege>(CR_PRIVILEGE_PATH)
            .await
        {
            CRPrivilege::privilege_granted(
                iface_ref.signal_emitter(),
                duration_minutes,
                allowed_uid,
                expires_at,
            )
            .await
            .map_err(|e| NetctlError::ServiceError(format!("Failed to emit PrivilegeGranted: {}", e)))?;
        }
        Ok(())
    }

    /// Emit PrivilegeRevoked signal
    pub async fn emit_privilege_revoked(conn: &Connection) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRPrivilege>(CR_PRIVILEGE_PATH)
            .await
        {
            CRPrivilege::privilege_revoked(iface_ref.signal_emitter())
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit PrivilegeRevoked: {}", e)))?;
        }
        Ok(())
    }

    /// Emit PrivilegeExpired signal
    pub async fn emit_privilege_expired(conn: &Connection) -> NetctlResult<()> {
        if let Ok(iface_ref) = conn
            .object_server()
            .interface::<_, CRPrivilege>(CR_PRIVILEGE_PATH)
            .await
        {
            CRPrivilege::privilege_expired(iface_ref.signal_emitter())
                .await
                .map_err(|e| NetctlError::ServiceError(format!("Failed to emit PrivilegeExpired: {}", e)))?;
        }
        Ok(())
    }
}
