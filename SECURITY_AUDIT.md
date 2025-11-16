# Security Audit Report for libnccli

**Date:** 2025-11-13
**Auditor:** Claude Code
**Component:** libnccli - Network Control CLI Tool
**Version:** 1.0.0

## Executive Summary

This security audit examined the libnccli command-line tool for potential security vulnerabilities. The audit focused on OWASP Top 10 web application security risks adapted for CLI applications, CWE Top 25, and general security best practices for system administration tools.

**Overall Risk Level:** MEDIUM

**Critical Findings:** 1
**High Findings:** 2
**Medium Findings:** 2
**Low Findings:** 1

## Methodology

The audit included:
1. Manual code review of all user input handling
2. Analysis of file operations and permissions
3. Review of command execution and injection risks
4. Assessment of configuration file security
5. Evaluation of input validation routines

## Findings

### 1. Path Traversal Vulnerability in Connection Names [CRITICAL]

**CWE-22: Improper Limitation of a Pathname to a Restricted Directory**

**Location:** `src/bin/libnccli.rs` lines 713, 749, 779, 842, 855, 868, 900, 913, 918

**Description:**
Connection names provided by users are directly used to construct file paths without validation. An attacker could use path traversal sequences like `../../../tmp/evil` to write or read files outside the intended configuration directory.

**Vulnerable Code:**
```rust
let config_path = config_dir.join(format!("{}.nctl", id));
```

**Attack Scenario:**
```bash
libnccli connection add --type ethernet --con-name "../../../tmp/malicious" --ip4 auto
# This would create /tmp/malicious.nctl instead of /etc/crrouter/netctl/../../../tmp/malicious.nctl
```

**Impact:** High - Arbitrary file read/write outside configuration directory

**Remediation:**
- Validate connection names to allow only alphanumeric characters, hyphens, and underscores
- Reject any connection names containing path separators (`/`, `\`, `..`)
- Implement whitelist-based validation

**Fixed Code:**
```rust
fn validate_connection_name(name: &str) -> Result<(), NetctlError> {
    if name.is_empty() {
        return Err(NetctlError::InvalidParameter("Connection name cannot be empty".to_string()));
    }
    if name.len() > 64 {
        return Err(NetctlError::InvalidParameter("Connection name too long (max 64 chars)".to_string()));
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(NetctlError::InvalidParameter("Connection name contains invalid characters".to_string()));
    }
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.') {
        return Err(NetctlError::InvalidParameter("Connection name can only contain alphanumeric, dash, underscore, or dot".to_string()));
    }
    Ok(())
}
```

---

### 2. Insufficient WiFi SSID Validation [HIGH]

**CWE-20: Improper Input Validation**

**Location:** `src/bin/libnccli.rs` lines 804-806, 1229-1231

**Description:**
WiFi SSID values are not validated for length or content. According to IEEE 802.11 standards, SSIDs must be 0-32 bytes. Accepting invalid SSIDs could cause buffer overflows in hostapd or other WiFi management tools.

**Vulnerable Code:**
```rust
if let Some(s) = ssid {
    config.push_str(&format!("ssid = \"{}\"\n", s));
}
```

**Impact:** Medium - Could cause crashes or unexpected behavior in WiFi stack

**Remediation:**
- Validate SSID length (1-32 bytes)
- Optionally validate for printable characters (some tools have issues with control characters)

**Fixed Code:**
```rust
fn validate_ssid(ssid: &str) -> Result<(), NetctlError> {
    if ssid.is_empty() {
        return Err(NetctlError::InvalidParameter("SSID cannot be empty".to_string()));
    }
    if ssid.len() > 32 {
        return Err(NetctlError::InvalidParameter("SSID too long (max 32 bytes)".to_string()));
    }
    Ok(())
}
```

---

### 3. Insufficient WiFi Password Validation [HIGH]

**CWE-521: Weak Password Requirements**

**Location:** `src/bin/libnccli.rs` lines 809-813, 1238-1239

**Description:**
WPA-PSK passwords are not validated for minimum/maximum length. WPA-PSK requires passwords to be 8-63 ASCII characters. Accepting invalid passwords will cause connection failures and poor user experience.

**Vulnerable Code:**
```rust
if let Some(pwd) = password {
    config.push_str("[wifi-security]\n");
    config.push_str("key-mgmt = \"wpa-psk\"\n");
    config.push_str(&format!("psk = \"{}\"\n\n", pwd));
}
```

**Impact:** Medium - Invalid configurations, potential security misconfiguration

**Remediation:**
- Validate password length (8-63 characters for WPA-PSK)
- Ensure ASCII characters only

**Fixed Code:**
```rust
fn validate_wifi_password(password: &str) -> Result<(), NetctlError> {
    if password.len() < 8 {
        return Err(NetctlError::InvalidParameter("WiFi password must be at least 8 characters".to_string()));
    }
    if password.len() > 63 {
        return Err(NetctlError::InvalidParameter("WiFi password too long (max 63 characters)".to_string()));
    }
    if !password.is_ascii() {
        return Err(NetctlError::InvalidParameter("WiFi password must contain only ASCII characters".to_string()));
    }
    Ok(())
}
```

---

### 4. Insecure File Permissions on Configuration Files [MEDIUM]

**CWE-732: Incorrect Permission Assignment for Critical Resource**

**Location:** `src/bin/libnccli.rs` lines 832, 920

**Description:**
Configuration files are created with default permissions (typically 644), making them world-readable. Since these files contain WiFi passwords and network credentials, they should be readable only by root.

**Vulnerable Code:**
```rust
std::fs::write(&config_path, config)
    .map_err(|e| NetctlError::Io(e))?;
```

**Impact:** Low - Password disclosure to local users (but this is typically a root-only tool)

**Remediation:**
- Set file permissions to 600 (read/write for owner only)
- Use `std::os::unix::fs::OpenOptionsExt` to set permissions atomically during creation

**Fixed Code:**
```rust
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;
use std::io::Write;

let mut file = OpenOptions::new()
    .write(true)
    .create(true)
    .truncate(true)
    .mode(0o600)  // rw-------
    .open(&config_path)
    .map_err(|e| NetctlError::Io(e))?;

file.write_all(config.as_bytes())
    .map_err(|e| NetctlError::Io(e))?;
```

---

### 5. Plain-Text Password Storage [MEDIUM]

**CWE-522: Insufficiently Protected Credentials**

**Location:** `src/bin/libnccli.rs` lines 809-813

**Description:**
WiFi passwords are stored in plain text in configuration files. While this is inherent to the NCTL format and most network configuration systems, it should be documented as a limitation.

**Impact:** Low - Standard limitation of most network management tools

**Remediation:**
- Document in user guide that passwords are stored in plain text
- Recommend file system encryption for sensitive deployments
- Ensure file permissions are restrictive (see Finding #4)

**Status:** Accepted Risk (inherent to network configuration format)

---

## Additional Security Observations

### Positive Security Controls

1. **Hostname Validation** - Already implemented with `validation::validate_hostname()` (line 530)
2. **Async/Await Usage** - Proper use of async reduces race conditions
3. **Error Handling** - Comprehensive error handling throughout
4. **Type Safety** - Strong typing through Rust prevents many vulnerability classes

### Recommendations for Future Development

1. **Audit Logging** - Add audit logging for all configuration changes
2. **Rate Limiting** - Consider rate limiting for WiFi scan operations
3. **Interface Name Validation** - Add explicit validation for interface names
4. **Command Injection Review** - While current code looks safe, explicitly document that interface names are validated before shell command execution
5. **Configuration Backup** - Implement automatic backup before modifying connection files

## Compliance Considerations

### OWASP Top 10 (Adapted for CLI)

- ✅ A01:2021 - Broken Access Control: Mitigated by OS-level permissions
- ⚠️ A02:2021 - Cryptographic Failures: Plain-text password storage (documented limitation)
- ⚠️ A03:2021 - Injection: Path traversal vulnerability identified (see Finding #1)
- ✅ A04:2021 - Insecure Design: Good separation of concerns
- ⚠️ A05:2021 - Security Misconfiguration: File permissions issue (see Finding #4)
- ✅ A06:2021 - Vulnerable Components: Using maintained dependencies
- ✅ A07:2021 - Identification and Authentication Failures: Relies on OS authentication
- ✅ A08:2021 - Software and Data Integrity Failures: No dynamic code execution
- ✅ A09:2021 - Security Logging Failures: Could be improved (see recommendations)
- ⚠️ A10:2021 - Server-Side Request Forgery: Not applicable to CLI tool

### CWE Top 25 Relevant Items

- ⚠️ CWE-22: Path Traversal (Finding #1)
- ⚠️ CWE-20: Improper Input Validation (Findings #2, #3)
- ⚠️ CWE-732: Incorrect Permission Assignment (Finding #4)
- ⚠️ CWE-522: Insufficiently Protected Credentials (Finding #5)

## Remediation Priority

**Immediate (Before Production Release):**
1. Fix path traversal vulnerability (Finding #1)
2. Add SSID validation (Finding #2)
3. Add WiFi password validation (Finding #3)
4. Fix file permissions (Finding #4)

**Short Term:**
1. Add audit logging
2. Improve documentation on security considerations
3. Add explicit interface name validation

**Long Term:**
1. Consider encrypted configuration storage
2. Implement configuration backup system
3. Add rate limiting for resource-intensive operations

## Testing Recommendations

1. **Fuzzing** - Use cargo-fuzz to test input parsers
2. **Integration Tests** - Add security-focused integration tests
3. **Penetration Testing** - Engage external security team before production deployment

## Conclusion

The libnccli tool has a solid foundation with good use of Rust's safety features. However, the identified vulnerabilities, particularly the path traversal issue, must be addressed before production use. The recommendations in this audit will significantly improve the security posture of the application.

## References

- OWASP Top 10 2021: https://owasp.org/Top10/
- CWE Top 25: https://cwe.mitre.org/top25/
- Rust Security Guidelines: https://anssi-fr.github.io/rust-guide/
- IEEE 802.11 Standards

---

**Report Generated:** 2025-11-13
**Next Audit Recommended:** After implementation of fixes
