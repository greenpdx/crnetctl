# Plan: Time-Limited Root Operations via Cryptographic Token

## Overview

Allow root to grant temporary elevated privileges to users by running:
```bash
sudo nccli --allow-root-ops 30   # Grant 30 minutes of privileged access
```

This creates a cryptographic token that allows non-root users to execute privileged nccli commands for the specified duration.

## Design

### Token Structure

```rust
struct PrivilegeToken {
    // Who created the token (must be root)
    granted_by_uid: u32,

    // When the token was created (Unix timestamp)
    created_at: u64,

    // Duration in minutes
    duration_minutes: u32,

    // Expiry time (Unix timestamp)
    expires_at: u64,

    // Optional: restrict to specific user
    allowed_uid: Option<u32>,

    // Optional: restrict to specific operations
    allowed_ops: Option<Vec<PrivilegedOp>>,

    // Random nonce for uniqueness
    nonce: [u8; 16],

    // HMAC-SHA256 signature
    signature: [u8; 32],
}
```

### Security Model

1. **Token Creation (root only)**
   - Root runs `sudo nccli --allow-root-ops <MINUTES>`
   - **New secret key is generated for EVERY token** (stored in `/run/netctl/secret.key`, mode 0600)
   - Token is created with expiry time and signed with HMAC-SHA256
   - Token is written to `/run/netctl/privilege-token` (mode 0644, readable by all)
   - Previous token is automatically invalidated (new key = old signature invalid)

2. **Token Verification (any user)**
   - Non-root user runs privileged command
   - nccli checks for token at `/run/netctl/privilege-token`
   - Verifies signature using secret key (read via netctld D-Bus call)
   - Checks expiry time
   - If valid, allows operation

3. **Token Revocation**
   - `sudo nccli --revoke-root-ops` deletes both token AND secret key
   - Token auto-expires after duration
   - Reboot clears `/run/` directory (both key and token)

4. **Key Rotation Security**
   - Each `--allow-root-ops` generates a fresh random key
   - Old tokens cannot be replayed (signature won't match new key)
   - No persistent secret key on disk (only in `/run/`, cleared on reboot)

### File Locations

| File | Permissions | Purpose |
|------|-------------|---------|
| `/run/netctl/secret.key` | 0600 root:root | HMAC signing key (32 bytes, regenerated each grant) |
| `/run/netctl/privilege-token` | 0644 root:root | Current privilege token |

**Note:** Both files are in `/run/` which is a tmpfs cleared on reboot. No persistent secrets on disk.

### Command Line Interface

```bash
# Grant 30 minutes of privileged access (root only)
sudo nccli --allow-root-ops 30

# Grant 60 minutes, restricted to current user only
sudo nccli --allow-root-ops 60 --user $USER

# Revoke all temporary privileges (root only)
sudo nccli --revoke-root-ops

# Show current privilege status
nccli --show-privileges
```

### Implementation Steps

#### Step 1: Add CLI Arguments

```rust
#[derive(Parser)]
struct Cli {
    // ... existing args ...

    /// Grant temporary root privileges for MINUTES (requires root)
    #[arg(long, value_name = "MINUTES")]
    allow_root_ops: Option<u32>,

    /// Restrict --allow-root-ops to specific user
    #[arg(long, requires = "allow_root_ops")]
    user: Option<String>,

    /// Revoke temporary root privileges (requires root)
    #[arg(long)]
    revoke_root_ops: bool,

    /// Show current privilege status
    #[arg(long)]
    show_privileges: bool,
}
```

#### Step 2: Token Module (`src/privilege_token.rs`)

```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

const SECRET_KEY_PATH: &str = "/run/netctl/secret.key";
const TOKEN_PATH: &str = "/run/netctl/privilege-token";
const KEY_SIZE: usize = 32;

pub struct PrivilegeToken {
    pub granted_by_uid: u32,
    pub created_at: u64,
    pub duration_minutes: u32,
    pub expires_at: u64,
    pub allowed_uid: Option<u32>,
    pub nonce: [u8; 16],
    pub signature: [u8; 32],
}

impl PrivilegeToken {
    /// Create a new token (must be called as root)
    /// Generates a NEW secret key, invalidating any previous tokens
    pub fn create(duration_minutes: u32, allowed_uid: Option<u32>) -> Result<Self, Error> {
        if !is_root() {
            return Err(Error::PermissionDenied("Must be root to create token"));
        }

        // Always create a fresh key - this invalidates any previous tokens
        let secret_key = create_new_secret_key()?;
        let now = current_timestamp();
        let expires_at = now + (duration_minutes as u64 * 60);
        let nonce = generate_nonce();

        let mut token = PrivilegeToken {
            granted_by_uid: unsafe { libc::getuid() },
            created_at: now,
            duration_minutes,
            expires_at,
            allowed_uid,
            nonce,
            signature: [0u8; 32],
        };

        token.signature = token.compute_signature(&secret_key);
        token.save()?;

        Ok(token)
    }

    /// Verify token is valid for current user
    pub fn verify(&self) -> Result<bool, Error> {
        // Check expiry
        if current_timestamp() > self.expires_at {
            return Ok(false);
        }

        // Check user restriction
        if let Some(allowed) = self.allowed_uid {
            if unsafe { libc::getuid() } != allowed {
                return Ok(false);
            }
        }

        // Verify signature (need to get key via D-Bus if not root)
        let secret_key = get_secret_key()?;
        let expected_sig = self.compute_signature(&secret_key);

        Ok(constant_time_eq(&self.signature, &expected_sig))
    }

    /// Load token from disk
    pub fn load() -> Result<Option<Self>, Error> {
        if !Path::new(TOKEN_PATH).exists() {
            return Ok(None);
        }
        // Deserialize from file
        let data = std::fs::read(TOKEN_PATH)?;
        let token: PrivilegeToken = bincode::deserialize(&data)?;
        Ok(Some(token))
    }

    /// Save token to disk
    fn save(&self) -> Result<(), Error> {
        std::fs::create_dir_all("/run/netctl")?;
        let data = bincode::serialize(self)?;
        std::fs::write(TOKEN_PATH, data)?;
        Ok(())
    }

    /// Compute HMAC-SHA256 signature
    fn compute_signature(&self, key: &[u8]) -> [u8; 32] {
        let mut mac = Hmac::<Sha256>::new_from_slice(key).unwrap();
        mac.update(&self.granted_by_uid.to_le_bytes());
        mac.update(&self.created_at.to_le_bytes());
        mac.update(&self.duration_minutes.to_le_bytes());
        mac.update(&self.expires_at.to_le_bytes());
        mac.update(&self.allowed_uid.unwrap_or(0).to_le_bytes());
        mac.update(&self.nonce);
        mac.finalize().into_bytes().into()
    }
}

/// Delete the current token AND secret key
pub fn revoke_token() -> Result<(), Error> {
    if !is_root() {
        return Err(Error::PermissionDenied("Must be root to revoke token"));
    }
    // Remove token
    if Path::new(TOKEN_PATH).exists() {
        std::fs::remove_file(TOKEN_PATH)?;
    }
    // Remove secret key (makes any cached token invalid)
    if Path::new(SECRET_KEY_PATH).exists() {
        std::fs::remove_file(SECRET_KEY_PATH)?;
    }
    Ok(())
}

/// Generate a NEW secret key for each token (root only)
/// This invalidates any previous tokens automatically
fn create_new_secret_key() -> Result<[u8; KEY_SIZE], Error> {
    let key: [u8; KEY_SIZE] = rand::random();
    std::fs::create_dir_all("/run/netctl")?;

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)  // Overwrite any existing key
        .mode(0o600)
        .open(SECRET_KEY_PATH)?;
    file.write_all(&key)?;

    Ok(key)
}

/// Read existing secret key (for verification)
fn get_secret_key() -> Result<[u8; KEY_SIZE], Error> {
    if !Path::new(SECRET_KEY_PATH).exists() {
        return Err(Error::NoSecretKey);
    }
    let key = std::fs::read(SECRET_KEY_PATH)?;
    key.try_into().map_err(|_| Error::InvalidKey)
}
```

#### Step 3: Update Privilege Check

```rust
fn check_privileged_op(op: PrivilegedOp, cli: &Cli) -> NetctlResult<()> {
    // Always allow if root
    if is_root() {
        return Ok(());
    }

    // Check for valid privilege token
    if let Ok(Some(token)) = PrivilegeToken::load() {
        if token.verify().unwrap_or(false) {
            return Ok(());
        }
    }

    Err(NetctlError::PermissionDenied(format!(
        "Operation '{}' requires root privileges.\n\
         Run with sudo, or ask root to grant temporary access:\n\
         sudo nccli --allow-root-ops <MINUTES>",
        op.description()
    )))
}
```

#### Step 4: Handle CLI Arguments in Main

```rust
#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Handle --allow-root-ops
    if let Some(minutes) = cli.allow_root_ops {
        if !is_root() {
            eprintln!("Error: --allow-root-ops requires root privileges");
            process::exit(1);
        }

        let allowed_uid = cli.user.map(|u| get_uid_for_user(&u));
        match PrivilegeToken::create(minutes, allowed_uid) {
            Ok(token) => {
                println!("Granted {} minutes of privileged access", minutes);
                println!("Expires at: {}", format_time(token.expires_at));
                if let Some(uid) = token.allowed_uid {
                    println!("Restricted to UID: {}", uid);
                }
            }
            Err(e) => {
                eprintln!("Error creating privilege token: {}", e);
                process::exit(1);
            }
        }
        return;
    }

    // Handle --revoke-root-ops
    if cli.revoke_root_ops {
        if !is_root() {
            eprintln!("Error: --revoke-root-ops requires root privileges");
            process::exit(1);
        }

        match revoke_token() {
            Ok(_) => println!("Privilege token revoked"),
            Err(e) => {
                eprintln!("Error revoking token: {}", e);
                process::exit(1);
            }
        }
        return;
    }

    // Handle --show-privileges
    if cli.show_privileges {
        match PrivilegeToken::load() {
            Ok(Some(token)) => {
                if token.verify().unwrap_or(false) {
                    let remaining = token.expires_at - current_timestamp();
                    println!("Privilege Status: GRANTED");
                    println!("Remaining time: {} minutes", remaining / 60);
                    println!("Expires at: {}", format_time(token.expires_at));
                } else {
                    println!("Privilege Status: EXPIRED");
                }
            }
            Ok(None) => {
                println!("Privilege Status: NONE");
                println!("Run as root: sudo nccli --allow-root-ops <MINUTES>");
            }
            Err(e) => {
                eprintln!("Error checking privileges: {}", e);
            }
        }
        return;
    }

    // ... rest of main() ...
}
```

### Security Considerations

1. **Secret Key Protection**
   - Key stored in `/run/netctl/` with 0600 permissions, only root can read
   - Non-root users verify via D-Bus call to netctld (which runs as root)
   - Key is in tmpfs, never persisted to disk

2. **Token Tampering**
   - HMAC-SHA256 prevents modification of token contents
   - Signature covers all fields including expiry time

3. **Replay Protection**
   - Random nonce ensures unique tokens
   - Expiry time limits token lifetime
   - **New key generated for each `--allow-root-ops` call**
   - Previous tokens automatically invalidated when new token is created

4. **User Restriction**
   - Optional `--user` flag restricts token to specific user
   - Prevents one user from using another's granted privileges

5. **Revocation**
   - Root can immediately revoke with `--revoke-root-ops`
   - Revocation deletes both token AND secret key
   - Reboot clears `/run/` directory automatically (both files)
   - Creating a new token also invalidates the old one (key rotation)

### Dependencies to Add

```toml
[dependencies]
hmac = "0.12"
sha2 = "0.10"
bincode = "1.3"
rand = "0.8"
```

### Testing

```bash
# Test granting privileges
sudo nccli --allow-root-ops 5

# Test as non-root user
nccli --show-privileges
nccli connection up my-connection  # Should work

# Wait 5 minutes or revoke
sudo nccli --revoke-root-ops

# Test again (should fail)
nccli connection up my-connection  # Should fail with permission error
```

## Timeline

1. Add privilege_token module with token structure
2. Implement token creation/verification
3. Add CLI arguments
4. Update main() to handle new arguments
5. Update check_privileged_op() to check token
6. Add D-Bus method for non-root signature verification
7. Testing and documentation
