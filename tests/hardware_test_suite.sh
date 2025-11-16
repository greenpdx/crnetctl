#!/bin/bash
#
# Hardware Test Suite for libnccli
#
# This script runs automated hardware tests for libnccli
# Requires root privileges for network operations

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

# Configuration
NCCLI="${NCCLI:-libnccli}"
TEST_INTERFACE="${TEST_INTERFACE:-eth1}"
TEST_WIFI_INTERFACE="${TEST_WIFI_INTERFACE:-wlan0}"
TEST_SSID="libnccli-test-ap-$$"
TEST_PASSWORD="test-password-123"
VERBOSE="${VERBOSE:-0}"

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo -e "${RED}Error: This script must be run as root${NC}"
   echo "Try: sudo $0"
   exit 1
fi

# Check if libnccli is available
if ! command -v "$NCCLI" &> /dev/null; then
    echo -e "${RED}Error: libnccli not found${NC}"
    echo "Install libnccli first or set NCCLI environment variable"
    exit 1
fi

# Logging
LOG_FILE="/tmp/libnccli_test_$(date +%Y%m%d_%H%M%S).log"
exec > >(tee -a "$LOG_FILE")
exec 2>&1

# Print test header
print_header() {
    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

# Print test info
print_test() {
    echo ""
    echo -e "${YELLOW}▶ Test: $1${NC}"
    ((TESTS_RUN++))
}

# Print success
print_success() {
    echo -e "${GREEN}✓ PASS: $1${NC}"
    ((TESTS_PASSED++))
}

# Print failure
print_failure() {
    echo -e "${RED}✗ FAIL: $1${NC}"
    ((TESTS_FAILED++))
}

# Print skip
print_skip() {
    echo -e "${YELLOW}⊘ SKIP: $1${NC}"
    ((TESTS_SKIPPED++))
}

# Run command and check result
run_test() {
    local test_name="$1"
    shift
    local cmd="$@"

    print_test "$test_name"

    if [[ $VERBOSE -eq 1 ]]; then
        echo "Command: $cmd"
    fi

    if eval "$cmd" > /tmp/test_output.txt 2>&1; then
        print_success "$test_name"
        if [[ $VERBOSE -eq 1 ]]; then
            cat /tmp/test_output.txt
        fi
        return 0
    else
        print_failure "$test_name"
        echo "Command: $cmd"
        cat /tmp/test_output.txt
        return 1
    fi
}

# Check if interface exists
check_interface() {
    local iface="$1"
    if ip link show "$iface" &> /dev/null; then
        return 0
    else
        return 1
    fi
}

# Cleanup function
cleanup() {
    echo ""
    echo "Cleaning up test artifacts..."

    # Remove test connections
    for conn in /etc/crrouter/netctl/test-*.nctl; do
        if [ -f "$conn" ]; then
            local name=$(basename "$conn" .nctl)
            $NCCLI connection delete "$name" 2>/dev/null || true
        fi
    done

    # Stop any test hotspots
    pkill -f "hostapd.*$TEST_SSID" 2>/dev/null || true

    # Restore test interface
    if check_interface "$TEST_INTERFACE"; then
        ip addr flush dev "$TEST_INTERFACE" 2>/dev/null || true
        ip link set "$TEST_INTERFACE" down 2>/dev/null || true
    fi

    echo "Cleanup complete"
}

trap cleanup EXIT

# Start tests
print_header "libnccli Hardware Test Suite"
echo "Started: $(date)"
echo "libnccli: $NCCLI"
echo "Log file: $LOG_FILE"
echo "Test interface: $TEST_INTERFACE"
echo "WiFi interface: $TEST_WIFI_INTERFACE"

# =============================================================================
# Basic CLI Tests
# =============================================================================
print_header "1. Basic CLI Tests"

run_test "Help command" \
    "$NCCLI --help | grep -q 'Network Control CLI'"

run_test "General status" \
    "$NCCLI general status | grep -q 'STATE'"

run_test "General status terse" \
    "$NCCLI -t general status | grep -q 'running:enabled'"

run_test "General permissions" \
    "$NCCLI general permissions | grep -q 'network.control'"

run_test "General logging" \
    "$NCCLI general logging | grep -q 'LEVEL'"

run_test "Radio status" \
    "$NCCLI radio all | grep -q 'WIFI'"

# =============================================================================
# Device Tests
# =============================================================================
print_header "2. Device Management Tests"

run_test "Device status" \
    "$NCCLI device status | grep -q 'DEVICE'"

run_test "Device status terse" \
    "$NCCLI -t device status"

run_test "Show loopback device" \
    "$NCCLI device show lo | grep -q 'GENERAL'"

run_test "Show loopback terse" \
    "$NCCLI -t device show lo | grep -q 'GENERAL.DEVICE:lo'"

# Test with actual interface if available
if check_interface "$TEST_INTERFACE"; then
    run_test "Show test interface" \
        "$NCCLI device show $TEST_INTERFACE | grep -q 'DEVICE'"

    run_test "Interface up" \
        "$NCCLI device connect $TEST_INTERFACE"

    run_test "Verify interface up" \
        "ip link show $TEST_INTERFACE | grep -q 'state UP'"

    run_test "Interface down" \
        "$NCCLI device disconnect $TEST_INTERFACE"

    run_test "Verify interface down" \
        "ip link show $TEST_INTERFACE | grep -q 'state DOWN'"
else
    print_skip "Test interface $TEST_INTERFACE not available"
    ((TESTS_RUN+=4))
    ((TESTS_SKIPPED+=4))
fi

# =============================================================================
# Connection Tests
# =============================================================================
print_header "3. Connection Management Tests"

run_test "List connections" \
    "$NCCLI connection show"

run_test "List connections terse" \
    "$NCCLI -t connection show"

# Create test connection
if check_interface "$TEST_INTERFACE"; then
    run_test "Add ethernet connection" \
        "$NCCLI connection add --type ethernet --con-name test-eth-$$ --ifname $TEST_INTERFACE --ip4 192.168.99.99/24"

    run_test "Verify connection created" \
        "test -f /etc/crrouter/netctl/test-eth-$$.nctl"

    run_test "Show connection" \
        "$NCCLI connection show test-eth-$$"

    run_test "Connection up" \
        "$NCCLI connection up test-eth-$$ --ifname $TEST_INTERFACE"

    run_test "Verify IP assigned" \
        "ip addr show $TEST_INTERFACE | grep -q '192.168.99.99'"

    run_test "Connection down" \
        "$NCCLI connection down test-eth-$$"

    run_test "Delete connection" \
        "$NCCLI connection delete test-eth-$$"

    run_test "Verify connection deleted" \
        "! test -f /etc/crrouter/netctl/test-eth-$$.nctl"
else
    print_skip "Test interface not available for connection tests"
    ((TESTS_RUN+=8))
    ((TESTS_SKIPPED+=8))
fi

# =============================================================================
# WiFi Tests
# =============================================================================
print_header "4. WiFi Tests"

if check_interface "$TEST_WIFI_INTERFACE"; then
    run_test "WiFi device list" \
        "$NCCLI device wifi list --ifname $TEST_WIFI_INTERFACE"

    run_test "WiFi device list terse" \
        "$NCCLI -t device wifi list --ifname $TEST_WIFI_INTERFACE"

    # Check if we can create AP (requires specific hardware capabilities)
    if iw list 2>/dev/null | grep -q "AP"; then
        run_test "Create WiFi hotspot" \
            "$NCCLI device wifi hotspot --ifname $TEST_WIFI_INTERFACE --ssid $TEST_SSID --password $TEST_PASSWORD --channel 6"

        sleep 3

        run_test "Verify hotspot running" \
            "ps aux | grep -q '[h]ostapd.*$TEST_SSID'"

        # Kill hotspot
        pkill -f "hostapd.*$TEST_SSID" 2>/dev/null || true
        sleep 1
    else
        print_skip "WiFi AP mode not supported on this hardware"
        ((TESTS_RUN+=2))
        ((TESTS_SKIPPED+=2))
    fi

    run_test "WiFi radio status" \
        "$NCCLI radio wifi"

else
    print_skip "WiFi interface $TEST_WIFI_INTERFACE not available"
    ((TESTS_RUN+=5))
    ((TESTS_SKIPPED+=5))
fi

# =============================================================================
# Networking Tests
# =============================================================================
print_header "5. Networking Tests"

run_test "Check connectivity" \
    "$NCCLI networking connectivity | grep -q 'full'"

run_test "Connectivity check flag" \
    "$NCCLI networking connectivity --check | grep -q 'full'"

# =============================================================================
# Output Format Tests
# =============================================================================
print_header "6. Output Format Tests"

run_test "Tabular output" \
    "$NCCLI -m tabular device status"

run_test "Terse output" \
    "$NCCLI -m terse -t device status"

run_test "Multiline output" \
    "$NCCLI -m multiline device status"

# =============================================================================
# Error Handling Tests
# =============================================================================
print_header "7. Error Handling Tests"

run_test "Invalid interface error" \
    "! $NCCLI device show nonexistent-interface-xyz 2>&1 | grep -q 'not found'"

run_test "Invalid connection error" \
    "! $NCCLI connection up nonexistent-connection 2>&1 | grep -q 'not found'"

run_test "Invalid command" \
    "! $NCCLI invalid-command 2>&1"

# =============================================================================
# Stress Tests (if enabled)
# =============================================================================
if [[ "${STRESS_TEST:-0}" == "1" ]] && check_interface "$TEST_INTERFACE"; then
    print_header "8. Stress Tests"

    print_test "Rapid interface up/down (10 iterations)"
    local stress_pass=1
    for i in {1..10}; do
        $NCCLI device disconnect "$TEST_INTERFACE" >/dev/null 2>&1 || stress_pass=0
        sleep 0.5
        $NCCLI device connect "$TEST_INTERFACE" >/dev/null 2>&1 || stress_pass=0
        sleep 0.5
    done

    if [[ $stress_pass -eq 1 ]]; then
        print_success "Rapid interface changes"
    else
        print_failure "Rapid interface changes"
    fi

    print_test "Multiple WiFi scans (5 iterations)"
    if check_interface "$TEST_WIFI_INTERFACE"; then
        local scan_pass=1
        for i in {1..5}; do
            $NCCLI device wifi list --ifname "$TEST_WIFI_INTERFACE" >/dev/null 2>&1 || scan_pass=0
            sleep 1
        done

        if [[ $scan_pass -eq 1 ]]; then
            print_success "Multiple WiFi scans"
        else
            print_failure "Multiple WiFi scans"
        fi
    else
        print_skip "WiFi interface not available for stress test"
        ((TESTS_SKIPPED++))
    fi
fi

# =============================================================================
# Test Summary
# =============================================================================
print_header "Test Summary"

echo "Total tests run:    $TESTS_RUN"
echo -e "${GREEN}Tests passed:       $TESTS_PASSED${NC}"
echo -e "${RED}Tests failed:       $TESTS_FAILED${NC}"
echo -e "${YELLOW}Tests skipped:      $TESTS_SKIPPED${NC}"
echo ""
echo "Success rate: $(awk "BEGIN {printf \"%.1f\", ($TESTS_PASSED/$TESTS_RUN)*100}")%"
echo ""
echo "Completed: $(date)"
echo "Log file: $LOG_FILE"

# Exit with appropriate code
if [[ $TESTS_FAILED -gt 0 ]]; then
    echo -e "${RED}FAILED: Some tests did not pass${NC}"
    exit 1
else
    echo -e "${GREEN}SUCCESS: All tests passed${NC}"
    exit 0
fi
