#!/bin/bash
# test_ap.sh - Test Access Point commands
#
# Tests for nccli Access Point functionality including:
# - AP start/stop/status commands
# - AP configuration validation
# - AP parameter handling
#
# Note: Most tests require root privileges and a WiFi interface that supports AP mode

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test_helpers.sh"

echo "============================================"
echo "ACCESS POINT COMMAND TESTS"
echo "============================================"

# Get WiFi interface for tests
WIFI_IFACE=$(get_wifi_interface)

# ===========================================
# BASIC AP COMMAND TESTS
# ===========================================

echo ""
echo "--- Basic AP Command Tests ---"

# Test: ap --help
run_expect_output "ap --help shows subcommands" \
    "start" \
    $NCCLI ap --help

run_expect_output "ap --help shows stop" \
    "stop" \
    $NCCLI ap --help

run_expect_output "ap --help shows status" \
    "status" \
    $NCCLI ap --help

run_expect_output "ap --help shows restart" \
    "restart" \
    $NCCLI ap --help

# ===========================================
# AP START HELP TESTS
# ===========================================

echo ""
echo "--- AP Start Help Tests ---"

# Test: ap start --help shows all options
run_expect_output "ap start --help shows interface argument" \
    "interface" \
    $NCCLI ap start --help

run_expect_output "ap start --help shows ssid option" \
    "ssid" \
    $NCCLI ap start --help

run_expect_output "ap start --help shows password option" \
    "password" \
    $NCCLI ap start --help

run_expect_output "ap start --help shows channel option" \
    "channel" \
    $NCCLI ap start --help

run_expect_output "ap start --help shows band option" \
    "band" \
    $NCCLI ap start --help

run_expect_output "ap start --help shows country option" \
    "country" \
    $NCCLI ap start --help

run_expect_output "ap start --help shows ip option" \
    "ip" \
    $NCCLI ap start --help

# ===========================================
# AP STATUS TESTS
# ===========================================

echo ""
echo "--- AP Status Tests ---"

# Test: ap status (should work without root)
run_expect_success "ap status succeeds" \
    $NCCLI ap status

# Test: ap status shows running or stopped
run_expect_regex "ap status shows state" \
    "running|stopped" \
    $NCCLI ap status

# Test: ap status terse mode
run_expect_regex "ap status terse mode" \
    "running|stopped" \
    $NCCLI -t ap status

# ===========================================
# AP PARAMETER VALIDATION TESTS
# ===========================================

echo ""
echo "--- AP Parameter Validation Tests ---"

# Test: ap start without required arguments
run_expect_failure "ap start without interface fails" \
    $NCCLI ap start

run_expect_failure "ap start without ssid fails" \
    $NCCLI ap start wlan0

# Test: Invalid SSID (too long)
LONG_SSID="ThisSSIDIsWayTooLongForWiFiNetworksAndShouldFail"
run_expect_failure "ap start with too long SSID fails" \
    $NCCLI ap start wlan0 --ssid "$LONG_SSID"

# Test: Invalid SSID (empty)
run_expect_failure "ap start with empty SSID fails" \
    $NCCLI ap start wlan0 --ssid ""

# Test: Invalid password (too short)
run_expect_failure "ap start with short password fails" \
    $NCCLI ap start wlan0 --ssid "TestAP" --password "short"

# Test: Invalid password (too long - over 63 chars)
LONG_PASS="ThisPasswordIsWayTooLongForWPA2SecurityAndShouldFailValidation1234567890"
run_expect_failure "ap start with too long password fails" \
    $NCCLI ap start wlan0 --ssid "TestAP" --password "$LONG_PASS"

# Test: Invalid channel
run_expect_failure "ap start with invalid channel fails" \
    $NCCLI ap start wlan0 --ssid "TestAP" --channel 200

# Test: Invalid IP format
run_expect_failure "ap start with invalid IP format fails" \
    $NCCLI ap start wlan0 --ssid "TestAP" --ip "invalid-ip"

run_expect_failure "ap start with IP missing prefix fails" \
    $NCCLI ap start wlan0 --ssid "TestAP" --ip "10.0.0.1"

# Test: Invalid interface name
run_expect_failure "ap start with invalid interface name fails" \
    $NCCLI ap start "invalid/../iface" --ssid "TestAP"

# ===========================================
# AP CHANNEL/BAND VALIDATION TESTS
# ===========================================

echo ""
echo "--- AP Channel/Band Validation Tests ---"

# Test: Valid 2.4GHz channels (1-14)
test_start "valid 2.4GHz channels accepted"
valid_channels=true
for ch in 1 6 11; do
    # This should fail for other reasons (like no interface), not channel validation
    output=$($NCCLI ap start wlan_nonexistent --ssid "TestAP" --channel $ch --band "2.4GHz" 2>&1)
    if echo "$output" | grep -qi "invalid.*channel"; then
        valid_channels=false
        break
    fi
done
if $valid_channels; then
    test_pass "valid 2.4GHz channels accepted"
else
    test_fail "valid 2.4GHz channels accepted" "Valid channel was rejected"
fi

# Test: Invalid 2.4GHz channel (15 is invalid for most countries)
run_expect_failure "ap start with channel 15 on 2.4GHz fails" \
    $NCCLI ap start wlan0 --ssid "TestAP" --channel 15 --band "2.4GHz"

# Test: Valid 5GHz channels
test_start "valid 5GHz channels accepted"
valid_channels=true
for ch in 36 40 44 48; do
    output=$($NCCLI ap start wlan_nonexistent --ssid "TestAP" --channel $ch --band "5GHz" 2>&1)
    if echo "$output" | grep -qi "invalid.*channel"; then
        valid_channels=false
        break
    fi
done
if $valid_channels; then
    test_pass "valid 5GHz channels accepted"
else
    test_fail "valid 5GHz channels accepted" "Valid 5GHz channel was rejected"
fi

# ===========================================
# AP HOTSPOT COMMAND TESTS (via device wifi)
# ===========================================

echo ""
echo "--- Hotspot Command Tests (device wifi hotspot) ---"

# Test: device wifi hotspot --help
run_expect_output "device wifi hotspot --help shows options" \
    "ssid" \
    $NCCLI device wifi hotspot --help

run_expect_output "device wifi hotspot --help shows con-name" \
    "con-name" \
    $NCCLI device wifi hotspot --help

run_expect_output "device wifi hotspot --help shows band" \
    "band" \
    $NCCLI device wifi hotspot --help

run_expect_output "device wifi hotspot --help shows channel" \
    "channel" \
    $NCCLI device wifi hotspot --help

# ===========================================
# AP OPERATIONS TESTS (via D-Bus to netctld)
# ===========================================

echo ""
echo "--- AP Operations Tests (via D-Bus) ---"

# Note: nccli communicates with netctld via D-Bus, so root is NOT required
# for most operations. netctld runs as root and handles privileged ops.

if [ -n "$WIFI_IFACE" ]; then
    # Check if interface supports AP mode
    test_start "check WiFi AP mode support"
    if iw list 2>/dev/null | grep -q "* AP"; then
        test_pass "check WiFi AP mode support"
        AP_SUPPORTED=true
    else
        test_skip "check WiFi AP mode support" "Interface may not support AP mode"
        AP_SUPPORTED=false
    fi

    if [ "$AP_SUPPORTED" = true ]; then
        # Test: Stop any existing AP first
        $NCCLI ap stop > /dev/null 2>&1

        # Test: AP status before starting
        run_expect_output "ap status before start shows stopped" \
            "stopped" \
            $NCCLI ap status

        # Test: Start AP with valid parameters
        # Note: This test may still fail if hostapd is not installed or
        # if the interface doesn't support AP mode
        test_start "ap start with valid parameters"
        output=$($NCCLI ap start "$WIFI_IFACE" \
            --ssid "TestAP-$$" \
            --password "testpassword123" \
            --channel 6 \
            --band "2.4GHz" \
            --ip "10.255.99.1/24" 2>&1)
        exit_code=$?

        if [ $exit_code -eq 0 ]; then
            test_pass "ap start with valid parameters"

            # Test: AP status while running
            run_expect_output "ap status while running shows running" \
                "running" \
                $NCCLI ap status

            # Test: AP status terse while running
            run_expect_output "ap status terse while running" \
                "running" \
                $NCCLI -t ap status

            # Test: Starting AP while already running should fail
            run_expect_failure "ap start while already running fails" \
                $NCCLI ap start "$WIFI_IFACE" --ssid "AnotherAP" --ip "10.255.98.1/24"

            # Test: Stop AP
            run_expect_success "ap stop succeeds" \
                $NCCLI ap stop

            # Test: AP status after stopping
            sleep 1
            run_expect_output "ap status after stop shows stopped" \
                "stopped" \
                $NCCLI ap status

        elif echo "$output" | grep -qi "hostapd"; then
            test_skip "ap start with valid parameters" "hostapd not available or failed"
            test_skip "ap status while running" "AP not started"
            test_skip "ap status terse while running" "AP not started"
            test_skip "ap start while already running" "AP not started"
            test_skip "ap stop" "AP not started"
            test_skip "ap status after stop" "AP not started"
        else
            test_fail "ap start with valid parameters" "Failed: $output"
            test_skip "ap status while running" "AP not started"
            test_skip "ap status terse while running" "AP not started"
            test_skip "ap start while already running" "AP not started"
            test_skip "ap stop" "AP not started"
            test_skip "ap status after stop" "AP not started"
        fi

        # Cleanup: ensure AP is stopped
        $NCCLI ap stop > /dev/null 2>&1

        # Test: AP with open network (no password)
        test_start "ap start with open network"
        output=$($NCCLI ap start "$WIFI_IFACE" \
            --ssid "OpenTestAP-$$" \
            --channel 11 \
            --band "2.4GHz" \
            --ip "10.255.97.1/24" 2>&1)
        exit_code=$?

        if [ $exit_code -eq 0 ]; then
            test_pass "ap start with open network"
            $NCCLI ap stop > /dev/null 2>&1
        elif echo "$output" | grep -qi "hostapd"; then
            test_skip "ap start with open network" "hostapd not available"
        else
            test_fail "ap start with open network" "Failed: $output"
        fi

    else
        test_skip "ap start with valid parameters" "AP mode not supported"
        test_skip "ap status while running" "AP mode not supported"
        test_skip "ap start while already running" "AP mode not supported"
        test_skip "ap stop" "AP mode not supported"
        test_skip "ap start with open network" "AP mode not supported"
    fi
else
    test_skip "check WiFi AP mode support" "No WiFi interface found"
    test_skip "ap start with valid parameters" "No WiFi interface found"
    test_skip "ap status while running" "No WiFi interface found"
    test_skip "ap start while already running" "No WiFi interface found"
    test_skip "ap stop" "No WiFi interface found"
    test_skip "ap start with open network" "No WiFi interface found"
fi

# ===========================================
# AP STOP TESTS (safe to run without AP)
# ===========================================

echo ""
echo "--- AP Stop Tests ---"

# Test: ap stop when not running should succeed (or gracefully handle)
run_expect_success "ap stop when not running succeeds" \
    $NCCLI ap stop

# Test: ap stop terse mode
run_expect_success "ap stop terse mode" \
    $NCCLI -t ap stop

# ===========================================
# AP RESTART TESTS
# ===========================================

echo ""
echo "--- AP Restart Tests ---"

# Test: ap restart --help (restart is listed but may not be fully implemented)
run_expect_success "ap restart succeeds (not fully implemented message OK)" \
    $NCCLI ap restart

# ===========================================
# INTEGRATION WITH DHCP TESTS
# ===========================================

echo ""
echo "--- DHCP Integration Tests ---"

# Test: dhcp --help (AP typically needs DHCP for clients)
run_expect_output "dhcp --help shows subcommands" \
    "start" \
    $NCCLI dhcp --help

run_expect_output "dhcp --help shows range options" \
    "range" \
    $NCCLI dhcp start --help

# Test: dhcp status
run_expect_success "dhcp status succeeds" \
    $NCCLI dhcp status

# ===========================================
# COUNTRY CODE TESTS
# ===========================================

echo ""
echo "--- Country Code Tests ---"

# Test: Valid country codes (US, GB, DE, etc.)
test_start "valid country codes accepted"
valid_codes=true
for country in US GB DE FR JP; do
    output=$($NCCLI ap start wlan_nonexistent --ssid "TestAP" --country "$country" 2>&1)
    if echo "$output" | grep -qi "invalid.*country"; then
        valid_codes=false
        break
    fi
done
if $valid_codes; then
    test_pass "valid country codes accepted"
else
    test_fail "valid country codes accepted" "Valid country code was rejected"
fi

# Test: Invalid country code
run_expect_failure "ap start with invalid country code fails" \
    $NCCLI ap start wlan0 --ssid "TestAP" --country "XX" --ip "10.0.0.1/24"

run_expect_failure "ap start with numeric country code fails" \
    $NCCLI ap start wlan0 --ssid "TestAP" --country "123" --ip "10.0.0.1/24"

# ===========================================
# SUMMARY
# ===========================================

echo ""
echo "Access Point command tests completed"
print_summary
