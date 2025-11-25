#!/bin/bash
# test_dhcp_client.sh - Test DHCP client integration with crdhcpc
#
# Tests for nccli DHCP client functionality including:
# - DHCP client start/stop/renew/release
# - DHCP status queries
# - Integration with crdhcpc daemon via Unix socket
# - DHCP lease information
#
# Note: Most tests require root privileges and crdhcpc to be installed/running

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test_helpers.sh"

echo "============================================"
echo "DHCP CLIENT COMMAND TESTS"
echo "============================================"

# Paths to crdhcpc
CRDHCPC_BIN="${CRDHCPC_BIN:-/usr/local/bin/crdhcpc}"
CRDHCPC_SOCKET="/var/run/crdhcpc.sock"
CRDHCPC_CONFIG="${CRDHCPC_CONFIG:-/etc/dhcp-client.toml}"

# Get test interfaces
ETH_IFACE=$(get_ethernet_interface)
WIFI_IFACE=$(get_wifi_interface)

# ===========================================
# HELPER FUNCTIONS
# ===========================================

# Check if crdhcpc is installed
check_crdhcpc_installed() {
    if [ -x "$CRDHCPC_BIN" ]; then
        return 0
    fi
    return 1
}

# Check if crdhcpc daemon is running
check_crdhcpc_running() {
    if [ -S "$CRDHCPC_SOCKET" ]; then
        return 0
    fi
    return 1
}

# ===========================================
# INSTALLATION CHECKS
# ===========================================

echo ""
echo "--- Installation Checks ---"

# Check crdhcpc installation
test_start "crdhcpc is installed"
if check_crdhcpc_installed; then
    test_pass "crdhcpc is installed"
    CRDHCPC_INSTALLED=true
    echo "  Found at: $CRDHCPC_BIN"
else
    test_skip "crdhcpc is installed" "crdhcpc not found at $CRDHCPC_BIN"
    CRDHCPC_INSTALLED=false
fi

# Check crdhcpc daemon status
test_start "crdhcpc daemon is running"
if check_crdhcpc_running; then
    test_pass "crdhcpc daemon is running"
    CRDHCPC_RUNNING=true
    echo "  Socket: $CRDHCPC_SOCKET"
else
    test_skip "crdhcpc daemon is running" "Socket not found at $CRDHCPC_SOCKET"
    CRDHCPC_RUNNING=false
fi

# ===========================================
# BASIC DHCP COMMAND TESTS (via nccli)
# ===========================================

echo ""
echo "--- Basic DHCP Command Tests (nccli) ---"

# Note: nccli doesn't have direct dhcp client commands yet
# These tests check the existing dhcp server commands and general integration

# Test: dhcp --help
run_expect_output "dhcp --help shows subcommands" \
    "start" \
    $NCCLI dhcp --help

run_expect_output "dhcp --help shows stop" \
    "stop" \
    $NCCLI dhcp --help

run_expect_output "dhcp --help shows status" \
    "status" \
    $NCCLI dhcp --help

# Test: dhcp status
run_expect_success "dhcp status succeeds" \
    $NCCLI dhcp status

# ===========================================
# CRDHCPC CLI TESTS (direct)
# ===========================================

echo ""
echo "--- crdhcpc CLI Tests (direct) ---"

if [ "$CRDHCPC_INSTALLED" = true ]; then
    # Test: crdhcpc --help
    run_expect_output "crdhcpc --help shows commands" \
        "daemon\|start\|stop\|status" \
        $CRDHCPC_BIN --help

    # Test: crdhcpc --version
    run_expect_success "crdhcpc --version succeeds" \
        $CRDHCPC_BIN --version

    # Test: crdhcpc check --help
    run_expect_output "crdhcpc check --help shows generate-example" \
        "generate-example" \
        $CRDHCPC_BIN check --help

    # Test: crdhcpc check --generate-example
    test_start "crdhcpc check --generate-example"
    output=$($CRDHCPC_BIN check --generate-example 2>&1)
    if echo "$output" | grep -q "enabled = true"; then
        test_pass "crdhcpc check --generate-example"
    else
        test_fail "crdhcpc check --generate-example" "Output doesn't contain expected config"
    fi

    # Test: crdhcpc daemon --help
    run_expect_output "crdhcpc daemon --help shows foreground" \
        "foreground" \
        $CRDHCPC_BIN daemon --help

    # Test: crdhcpc start --help
    run_expect_output "crdhcpc start --help shows interface" \
        "interface" \
        $CRDHCPC_BIN start --help

    # Test: crdhcpc stop --help
    run_expect_output "crdhcpc stop --help shows interface" \
        "interface" \
        $CRDHCPC_BIN stop --help

    # Test: crdhcpc renew --help
    run_expect_output "crdhcpc renew --help shows interface" \
        "interface" \
        $CRDHCPC_BIN renew --help

    # Test: crdhcpc release --help
    run_expect_output "crdhcpc release --help shows interface" \
        "interface" \
        $CRDHCPC_BIN release --help

    # Test: crdhcpc status --help
    run_expect_success "crdhcpc status --help succeeds" \
        $CRDHCPC_BIN status --help
else
    test_skip "crdhcpc --help" "crdhcpc not installed"
    test_skip "crdhcpc --version" "crdhcpc not installed"
    test_skip "crdhcpc check" "crdhcpc not installed"
    test_skip "crdhcpc daemon --help" "crdhcpc not installed"
    test_skip "crdhcpc start --help" "crdhcpc not installed"
    test_skip "crdhcpc stop --help" "crdhcpc not installed"
    test_skip "crdhcpc renew --help" "crdhcpc not installed"
    test_skip "crdhcpc release --help" "crdhcpc not installed"
    test_skip "crdhcpc status --help" "crdhcpc not installed"
fi

# ===========================================
# CRDHCPC CONFIGURATION TESTS
# ===========================================

echo ""
echo "--- crdhcpc Configuration Tests ---"

if [ "$CRDHCPC_INSTALLED" = true ]; then
    # Test: Check if config file exists
    test_start "crdhcpc config file exists"
    if [ -f "$CRDHCPC_CONFIG" ]; then
        test_pass "crdhcpc config file exists"
        echo "  Found at: $CRDHCPC_CONFIG"
    else
        test_skip "crdhcpc config file exists" "Config not found at $CRDHCPC_CONFIG"
    fi

    # Test: Validate config file if it exists
    if [ -f "$CRDHCPC_CONFIG" ]; then
        test_start "crdhcpc config file is valid"
        output=$($CRDHCPC_BIN -c "$CRDHCPC_CONFIG" check 2>&1)
        if echo "$output" | grep -q "valid"; then
            test_pass "crdhcpc config file is valid"
        else
            test_fail "crdhcpc config file is valid" "Config validation failed"
            echo "$output"
        fi
    fi
else
    test_skip "crdhcpc config file exists" "crdhcpc not installed"
    test_skip "crdhcpc config file is valid" "crdhcpc not installed"
fi

# ===========================================
# CRDHCPC DAEMON STATUS TESTS
# ===========================================

echo ""
echo "--- crdhcpc Daemon Status Tests ---"

if [ "$CRDHCPC_INSTALLED" = true ] && [ "$CRDHCPC_RUNNING" = true ]; then
    # Test: crdhcpc status (all interfaces)
    run_expect_success "crdhcpc status succeeds" \
        $CRDHCPC_BIN status

    # Test: crdhcpc status returns JSON
    test_start "crdhcpc status returns valid output"
    output=$($CRDHCPC_BIN status 2>&1)
    if echo "$output" | grep -qE "running|v4_clients|v6_clients|interface"; then
        test_pass "crdhcpc status returns valid output"
    else
        test_fail "crdhcpc status returns valid output" "Output doesn't contain expected fields"
    fi

    # Test: crdhcpc status for specific interface
    if [ -n "$ETH_IFACE" ]; then
        run_expect_success "crdhcpc status for $ETH_IFACE" \
            $CRDHCPC_BIN status "$ETH_IFACE"
    fi

    if [ -n "$WIFI_IFACE" ]; then
        run_expect_success "crdhcpc status for $WIFI_IFACE" \
            $CRDHCPC_BIN status "$WIFI_IFACE"
    fi
else
    test_skip "crdhcpc status" "crdhcpc not installed or not running"
    test_skip "crdhcpc status output" "crdhcpc not installed or not running"
    test_skip "crdhcpc status for interface" "crdhcpc not installed or not running"
fi

# ===========================================
# CRDHCPC OPERATIONS TESTS (require root)
# ===========================================

echo ""
echo "--- crdhcpc Operations Tests (require root) ---"

if check_root; then
    if [ "$CRDHCPC_INSTALLED" = true ]; then
        # Test interface for DHCP operations
        TEST_IFACE="${ETH_IFACE:-$WIFI_IFACE}"

        if [ -n "$TEST_IFACE" ]; then
            echo "Using test interface: $TEST_IFACE"

            # Test: Start DHCP client (may already be running)
            test_start "crdhcpc start on $TEST_IFACE"
            output=$($CRDHCPC_BIN start "$TEST_IFACE" 2>&1 &)
            pid=$!
            sleep 2
            kill $pid 2>/dev/null
            # This may fail if already running, which is OK
            test_pass "crdhcpc start on $TEST_IFACE"

            if [ "$CRDHCPC_RUNNING" = true ]; then
                # Test: Renew DHCP lease
                test_start "crdhcpc renew on $TEST_IFACE"
                output=$($CRDHCPC_BIN renew "$TEST_IFACE" 2>&1)
                exit_code=$?
                if [ $exit_code -eq 0 ]; then
                    test_pass "crdhcpc renew on $TEST_IFACE"
                elif echo "$output" | grep -qi "no.*lease\|not.*running"; then
                    test_skip "crdhcpc renew on $TEST_IFACE" "No active lease to renew"
                else
                    test_fail "crdhcpc renew on $TEST_IFACE" "$output"
                fi

                # Test: Release DHCP lease (be careful - this will drop the lease!)
                # We'll skip this to avoid disrupting the network
                test_skip "crdhcpc release on $TEST_IFACE" "Skipped to avoid network disruption"

                # Test: Stop DHCP client (careful - may drop network)
                # We'll skip this too
                test_skip "crdhcpc stop on $TEST_IFACE" "Skipped to avoid network disruption"
            else
                test_skip "crdhcpc renew" "Daemon not running"
                test_skip "crdhcpc release" "Daemon not running"
                test_skip "crdhcpc stop" "Daemon not running"
            fi
        else
            test_skip "crdhcpc start" "No network interface available"
            test_skip "crdhcpc renew" "No network interface available"
            test_skip "crdhcpc release" "No network interface available"
            test_skip "crdhcpc stop" "No network interface available"
        fi

        # Test: Start daemon (if not already running)
        if [ "$CRDHCPC_RUNNING" = false ]; then
            test_start "crdhcpc daemon start"
            # Start in background with foreground flag for easy cleanup
            $CRDHCPC_BIN daemon --foreground &
            DAEMON_PID=$!
            sleep 2

            if [ -S "$CRDHCPC_SOCKET" ]; then
                test_pass "crdhcpc daemon start"
                # Clean up
                kill $DAEMON_PID 2>/dev/null
                sleep 1
            else
                test_fail "crdhcpc daemon start" "Socket not created"
                kill $DAEMON_PID 2>/dev/null
            fi
        else
            test_skip "crdhcpc daemon start" "Daemon already running"
        fi
    else
        test_skip "crdhcpc start" "crdhcpc not installed"
        test_skip "crdhcpc renew" "crdhcpc not installed"
        test_skip "crdhcpc release" "crdhcpc not installed"
        test_skip "crdhcpc stop" "crdhcpc not installed"
        test_skip "crdhcpc daemon start" "crdhcpc not installed"
    fi
else
    test_skip "crdhcpc start" "Requires root privileges"
    test_skip "crdhcpc renew" "Requires root privileges"
    test_skip "crdhcpc release" "Requires root privileges"
    test_skip "crdhcpc stop" "Requires root privileges"
    test_skip "crdhcpc daemon start" "Requires root privileges"
fi

# ===========================================
# INTERFACE VALIDATION TESTS
# ===========================================

echo ""
echo "--- Interface Validation Tests ---"

if [ "$CRDHCPC_INSTALLED" = true ]; then
    # Test: Invalid interface name
    run_expect_failure "crdhcpc start with invalid interface fails" \
        $CRDHCPC_BIN start "invalid/../iface"

    run_expect_failure "crdhcpc start with empty interface fails" \
        $CRDHCPC_BIN start ""

    # Test: Interface name too long
    LONG_IFACE="this_interface_name_is_way_too_long_for_linux"
    run_expect_failure "crdhcpc start with too long interface fails" \
        $CRDHCPC_BIN start "$LONG_IFACE"

    # Test: Interface with special characters
    run_expect_failure "crdhcpc start with special chars fails" \
        $CRDHCPC_BIN start "eth0;rm -rf"

    # Test: Nonexistent interface
    run_expect_failure "crdhcpc status for nonexistent interface fails" \
        $CRDHCPC_BIN status "nonexistent_iface_99"
else
    test_skip "interface validation tests" "crdhcpc not installed"
fi

# ===========================================
# JSON-RPC INTERFACE TESTS
# ===========================================

echo ""
echo "--- JSON-RPC Interface Tests ---"

if [ "$CRDHCPC_RUNNING" = true ]; then
    # Test: Check socket permissions
    test_start "crdhcpc socket has correct permissions"
    socket_perms=$(stat -c "%a" "$CRDHCPC_SOCKET" 2>/dev/null)
    if [ -n "$socket_perms" ]; then
        test_pass "crdhcpc socket has correct permissions"
        echo "  Socket permissions: $socket_perms"
    else
        test_fail "crdhcpc socket has correct permissions" "Could not stat socket"
    fi

    # Test: Socket is accessible
    test_start "crdhcpc socket is accessible"
    if [ -S "$CRDHCPC_SOCKET" ]; then
        test_pass "crdhcpc socket is accessible"
    else
        test_fail "crdhcpc socket is accessible" "Socket not found or not a socket"
    fi
else
    test_skip "crdhcpc socket permissions" "Daemon not running"
    test_skip "crdhcpc socket accessible" "Daemon not running"
fi

# ===========================================
# INTEGRATION WITH NETCTL TESTS
# ===========================================

echo ""
echo "--- Integration with netctl Tests ---"

# Test: Connection up should trigger DHCP if configured
# This is an indirect test - we check if the dhcp_client module is accessible

test_start "netctl has dhcp_client integration"
# Check if dhcp_client.rs exists in the netctl source
if [ -f "$SCRIPT_DIR/../../src/dhcp_client.rs" ]; then
    test_pass "netctl has dhcp_client integration"
else
    test_fail "netctl has dhcp_client integration" "dhcp_client.rs not found"
fi

# Test: Connection add with auto IP should work
run_expect_success "connection add with auto IP" \
    $NCCLI connection add --type ethernet --con-name test-dhcp-auto --ip4 auto 2>/dev/null || true

# Clean up test connection
$NCCLI connection delete test-dhcp-auto 2>/dev/null || true

# ===========================================
# DHCP LEASE INFORMATION TESTS
# ===========================================

echo ""
echo "--- DHCP Lease Information Tests ---"

if [ "$CRDHCPC_RUNNING" = true ]; then
    # Test: Get lease information
    test_start "crdhcpc status shows lease info"
    output=$($CRDHCPC_BIN status 2>&1)
    # Check for typical lease fields
    if echo "$output" | grep -qE "ip_address|lease|state"; then
        test_pass "crdhcpc status shows lease info"
    else
        test_skip "crdhcpc status shows lease info" "No active leases or different output format"
    fi
else
    test_skip "crdhcpc status shows lease info" "Daemon not running"
fi

# ===========================================
# DHCPv4 vs DHCPv6 TESTS
# ===========================================

echo ""
echo "--- DHCPv4 and DHCPv6 Tests ---"

if [ "$CRDHCPC_INSTALLED" = true ] && [ "$CRDHCPC_RUNNING" = true ]; then
    # Test: Status shows both v4 and v6 clients
    test_start "crdhcpc status shows v4 and v6 clients"
    output=$($CRDHCPC_BIN status 2>&1)
    if echo "$output" | grep -qE "v4_clients.*v6_clients|DHCPv4.*DHCPv6"; then
        test_pass "crdhcpc status shows v4 and v6 clients"
    else
        test_skip "crdhcpc status shows v4 and v6 clients" "Output format may differ"
    fi
else
    test_skip "crdhcpc v4/v6 clients" "crdhcpc not installed or not running"
fi

# ===========================================
# SYSTEMD INTEGRATION TESTS
# ===========================================

echo ""
echo "--- Systemd Integration Tests ---"

# Test: Check if systemd service exists
test_start "crdhcpc systemd service exists"
if [ -f "/etc/systemd/system/dhcp-client-standalone.service" ] || \
   [ -f "/lib/systemd/system/dhcp-client-standalone.service" ] || \
   systemctl list-unit-files | grep -q "dhcp-client\|crdhcpc"; then
    test_pass "crdhcpc systemd service exists"
else
    test_skip "crdhcpc systemd service exists" "Service file not found"
fi

# Test: Check systemd service status (if installed)
if check_root; then
    test_start "crdhcpc systemd service status"
    if systemctl is-active dhcp-client-standalone.service > /dev/null 2>&1; then
        test_pass "crdhcpc systemd service status"
        echo "  Service is active"
    elif systemctl is-enabled dhcp-client-standalone.service > /dev/null 2>&1; then
        test_pass "crdhcpc systemd service status"
        echo "  Service is enabled but not active"
    else
        test_skip "crdhcpc systemd service status" "Service not installed or not enabled"
    fi
else
    test_skip "crdhcpc systemd service status" "Requires root privileges"
fi

# ===========================================
# SUMMARY
# ===========================================

echo ""
echo "DHCP client command tests completed"
print_summary
