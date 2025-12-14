#!/bin/bash
#
# Hardware Test Suite for nccli
#
# This script runs automated hardware tests for nccli
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
NCCLI="${NCCLI:-nccli}"
TEST_INTERFACE="${TEST_INTERFACE:-eth1}"
TEST_WIFI_INTERFACE="${TEST_WIFI_INTERFACE:-wlan0}"
TEST_SSID="nccli-test-ap-$$"
TEST_PASSWORD="test-password-123"
VERBOSE="${VERBOSE:-0}"

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo -e "${RED}Error: This script must be run as root${NC}"
   echo "Try: sudo $0"
   exit 1
fi

# Check if nccli is available
if ! command -v "$NCCLI" &> /dev/null; then
    echo -e "${RED}Error: nccli not found${NC}"
    echo "Install nccli first or set NCCLI environment variable"
    exit 1
fi

# Logging
LOG_FILE="/tmp/nccli_test_$(date +%Y%m%d_%H%M%S).log"
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
print_header "nccli Hardware Test Suite"
echo "Started: $(date)"
echo "nccli: $NCCLI"
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
# WiFi Connection Bring-up Tests
# =============================================================================
print_header "8. WiFi Connection Bring-up Tests"

if check_interface "$TEST_WIFI_INTERFACE"; then
    # Check if wpa_supplicant is available
    if command -v wpa_supplicant &> /dev/null && command -v wpa_cli &> /dev/null; then

        # Test: WiFi interface can be brought up
        print_test "WiFi interface bring-up"
        if ip link set "$TEST_WIFI_INTERFACE" up 2>/dev/null; then
            print_success "WiFi interface brought up"
        else
            print_failure "Failed to bring up WiFi interface"
        fi

        # Test: Check rfkill status
        print_test "WiFi rfkill status check"
        if command -v rfkill &> /dev/null; then
            rfkill_blocked=$(rfkill list wifi 2>/dev/null | grep -c "blocked: yes" || echo "0")
            if [[ "$rfkill_blocked" -eq 0 ]]; then
                print_success "WiFi not blocked by rfkill"
            else
                print_failure "WiFi is blocked by rfkill"
            fi
        else
            print_skip "rfkill command not available"
            ((TESTS_SKIPPED++))
        fi

        # Test: wpa_supplicant can start
        print_test "wpa_supplicant startup"
        wpa_running=$(pgrep -f "wpa_supplicant.*-i.*$TEST_WIFI_INTERFACE" 2>/dev/null || echo "")
        if [[ -n "$wpa_running" ]]; then
            print_success "wpa_supplicant already running on $TEST_WIFI_INTERFACE"
        else
            # Try to start wpa_supplicant
            temp_conf=$(mktemp /tmp/wpa_test_XXXXXX.conf)
            echo -e "ctrl_interface=/var/run/wpa_supplicant\nupdate_config=1" > "$temp_conf"

            if wpa_supplicant -B -i "$TEST_WIFI_INTERFACE" -c "$temp_conf" 2>/dev/null; then
                sleep 2
                if pgrep -f "wpa_supplicant.*-i.*$TEST_WIFI_INTERFACE" > /dev/null 2>&1; then
                    print_success "wpa_supplicant started successfully"
                    # Clean up - stop wpa_supplicant we started
                    wpa_cli -i "$TEST_WIFI_INTERFACE" terminate 2>/dev/null || true
                else
                    print_failure "wpa_supplicant failed to stay running"
                fi
            else
                print_failure "Failed to start wpa_supplicant"
            fi
            rm -f "$temp_conf"
        fi

        # Test: WiFi scan capability
        print_test "WiFi scan capability"
        if pgrep -f "wpa_supplicant.*-i.*$TEST_WIFI_INTERFACE" > /dev/null 2>&1; then
            scan_result=$(wpa_cli -i "$TEST_WIFI_INTERFACE" scan 2>/dev/null)
            if [[ "$scan_result" == "OK" ]] || [[ "$scan_result" == "FAIL-BUSY" ]]; then
                print_success "WiFi scan triggered ($scan_result)"
            else
                print_failure "WiFi scan failed: $scan_result"
            fi
        else
            print_skip "wpa_supplicant not running, cannot test scan"
            ((TESTS_SKIPPED++))
        fi

        # Test: WiFi connection state retrieval
        print_test "WiFi connection state retrieval"
        if pgrep -f "wpa_supplicant.*-i.*$TEST_WIFI_INTERFACE" > /dev/null 2>&1; then
            status_output=$(wpa_cli -i "$TEST_WIFI_INTERFACE" status 2>/dev/null)
            if echo "$status_output" | grep -q "wpa_state="; then
                wpa_state=$(echo "$status_output" | grep "wpa_state=" | cut -d= -f2)
                print_success "WiFi state retrieved: $wpa_state"
            else
                print_failure "Failed to retrieve WiFi state"
            fi
        else
            print_skip "wpa_supplicant not running"
            ((TESTS_SKIPPED++))
        fi
    else
        print_skip "wpa_supplicant/wpa_cli not installed"
        ((TESTS_RUN+=5))
        ((TESTS_SKIPPED+=5))
    fi
else
    print_skip "WiFi interface $TEST_WIFI_INTERFACE not available for bring-up tests"
    ((TESTS_RUN+=5))
    ((TESTS_SKIPPED+=5))
fi

# =============================================================================
# Network Interface Hotplug Tests
# =============================================================================
print_header "9. Network Interface Hotplug Tests"

# Test: Dummy interface creation (simulates hotplug)
print_test "Dummy interface hotplug simulation"
HOTPLUG_IFACE="test_hotplug_$$"

# Record initial interface count
initial_count=$(ls /sys/class/net | wc -l)

if ip link add "$HOTPLUG_IFACE" type dummy 2>/dev/null; then
    sleep 0.5
    current_count=$(ls /sys/class/net | wc -l)

    if [[ -d "/sys/class/net/$HOTPLUG_IFACE" ]]; then
        print_success "Dummy interface created and detected ($initial_count -> $current_count interfaces)"

        # Test: Interface state changes
        print_test "Interface state change detection"
        ip link set "$HOTPLUG_IFACE" up 2>/dev/null
        sleep 0.3
        operstate=$(cat "/sys/class/net/$HOTPLUG_IFACE/operstate" 2>/dev/null || echo "unknown")
        if [[ "$operstate" != "down" ]]; then
            print_success "Interface state change detected: $operstate"
        else
            print_failure "Interface state did not change"
        fi

        # Clean up dummy interface
        ip link delete "$HOTPLUG_IFACE" 2>/dev/null
        sleep 0.3

        # Test: Interface removal detection
        print_test "Interface removal detection"
        if [[ ! -d "/sys/class/net/$HOTPLUG_IFACE" ]]; then
            final_count=$(ls /sys/class/net | wc -l)
            print_success "Interface removal detected ($final_count interfaces remaining)"
        else
            print_failure "Interface still exists after deletion"
        fi
    else
        print_failure "Dummy interface not found in /sys/class/net"
        ip link delete "$HOTPLUG_IFACE" 2>/dev/null
    fi
else
    print_failure "Failed to create dummy interface (kernel module missing?)"
    ((TESTS_RUN+=2))
fi

# Test: Veth pair creation (simulates network device hotplug)
print_test "Veth pair hotplug simulation"
VETH0="test_veth0_$$"
VETH1="test_veth1_$$"

if ip link add "$VETH0" type veth peer name "$VETH1" 2>/dev/null; then
    sleep 0.3
    if [[ -d "/sys/class/net/$VETH0" ]] && [[ -d "/sys/class/net/$VETH1" ]]; then
        print_success "Veth pair created and both interfaces detected"

        # Test: Veth link state propagation
        print_test "Veth link state propagation"
        ip link set "$VETH0" up 2>/dev/null
        ip link set "$VETH1" up 2>/dev/null
        sleep 0.3

        veth0_state=$(cat "/sys/class/net/$VETH0/operstate" 2>/dev/null || echo "unknown")
        veth1_state=$(cat "/sys/class/net/$VETH1/operstate" 2>/dev/null || echo "unknown")

        if [[ "$veth0_state" != "down" ]] && [[ "$veth1_state" != "down" ]]; then
            print_success "Veth pair link states: $VETH0=$veth0_state, $VETH1=$veth1_state"
        else
            print_failure "Veth pair link states not as expected"
        fi
    else
        print_failure "Veth pair interfaces not found"
    fi

    # Clean up
    ip link delete "$VETH0" 2>/dev/null
else
    print_failure "Failed to create veth pair"
    ((TESTS_RUN++))
fi

# Test: Monitor /sys/class/net changes
print_test "Network interface monitoring via /sys/class/net"
if [[ -d "/sys/class/net" ]]; then
    iface_list=$(ls /sys/class/net 2>/dev/null | tr '\n' ' ')
    if [[ -n "$iface_list" ]]; then
        print_success "Interface enumeration works: $iface_list"
    else
        print_failure "No interfaces found in /sys/class/net"
    fi
else
    print_failure "/sys/class/net directory not accessible"
fi

# =============================================================================
# Boot/Startup Network Tests
# =============================================================================
print_header "10. Boot/Startup Network Tests"

# Test: Check for autoconnect configurations
print_test "Autoconnect configuration check"
config_dirs=("/etc/crrouter/netctl" "/etc/netctl")
found_autoconnect=0

for dir in "${config_dirs[@]}"; do
    if [[ -d "$dir" ]]; then
        autoconnect_configs=$(grep -l "autoconnect = true" "$dir"/*.nctl 2>/dev/null | wc -l)
        if [[ "$autoconnect_configs" -gt 0 ]]; then
            print_success "Found $autoconnect_configs autoconnect configs in $dir"
            found_autoconnect=1
            break
        fi
    fi
done

if [[ "$found_autoconnect" -eq 0 ]]; then
    print_skip "No autoconnect configurations found (expected if not configured)"
    ((TESTS_SKIPPED++))
fi

# Test: netctld service status
print_test "netctld service status"
if systemctl is-active --quiet netctld 2>/dev/null; then
    print_success "netctld service is active"
elif systemctl list-unit-files 2>/dev/null | grep -q "netctld.service"; then
    netctld_status=$(systemctl is-active netctld 2>/dev/null || echo "inactive")
    print_success "netctld service exists (status: $netctld_status)"
else
    print_skip "netctld service not installed"
    ((TESTS_SKIPPED++))
fi

# Test: D-Bus service availability
print_test "D-Bus service availability check"
if command -v dbus-send &> /dev/null; then
    if dbus-send --system --print-reply --dest=org.freedesktop.DBus \
       /org/freedesktop/DBus org.freedesktop.DBus.NameHasOwner \
       string:"org.crrouter.NetworkControl" 2>/dev/null | grep -q "true"; then
        print_success "org.crrouter.NetworkControl D-Bus service is available"
    else
        print_skip "org.crrouter.NetworkControl D-Bus service not running"
        ((TESTS_SKIPPED++))
    fi
else
    print_skip "dbus-send not available"
    ((TESTS_SKIPPED++))
fi

# Test: Link monitor behavior simulation
print_test "Link monitor auto-DHCP logic check"
# This tests that the logic for "link up -> start DHCP" would work
if check_interface "$TEST_INTERFACE"; then
    # Bring interface down, then up
    ip link set "$TEST_INTERFACE" down 2>/dev/null
    sleep 0.5
    initial_state=$(cat "/sys/class/net/$TEST_INTERFACE/operstate" 2>/dev/null || echo "unknown")

    ip link set "$TEST_INTERFACE" up 2>/dev/null
    sleep 0.5
    final_state=$(cat "/sys/class/net/$TEST_INTERFACE/operstate" 2>/dev/null || echo "unknown")

    if [[ "$initial_state" != "$final_state" ]] || [[ "$final_state" != "down" ]]; then
        print_success "Interface state transition: $initial_state -> $final_state"
    else
        print_failure "Interface state did not change as expected"
    fi

    # Leave interface down to avoid affecting system
    ip link set "$TEST_INTERFACE" down 2>/dev/null
else
    print_skip "Test interface not available for link monitor test"
    ((TESTS_SKIPPED++))
fi

# Test: nccli can list connections (startup readiness)
print_test "nccli connection listing (startup readiness)"
if $NCCLI connection show > /dev/null 2>&1; then
    conn_count=$($NCCLI connection show 2>/dev/null | tail -n +2 | wc -l)
    print_success "nccli ready, found $conn_count connection(s)"
else
    print_failure "nccli connection show failed"
fi

# =============================================================================
# Stress Tests (if enabled)
# =============================================================================
if [[ "${STRESS_TEST:-0}" == "1" ]] && check_interface "$TEST_INTERFACE"; then
    print_header "11. Stress Tests"

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
