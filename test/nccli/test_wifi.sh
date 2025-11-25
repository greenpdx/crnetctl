#!/bin/bash
# test_wifi.sh - Test WiFi commands
#
# Tests for nccli WiFi functionality including:
# - WiFi device listing and scanning
# - WiFi radio control
# - WiFi connection commands
# - WiFi hotspot creation
#
# Note: Many tests require a WiFi interface and/or root privileges

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test_helpers.sh"

echo "============================================"
echo "WIFI COMMAND TESTS"
echo "============================================"

# Get WiFi interface for tests
WIFI_IFACE=$(get_wifi_interface)

# ===========================================
# BASIC WIFI COMMAND TESTS
# ===========================================

echo ""
echo "--- Basic WiFi Command Tests ---"

# Test: device wifi --help
run_expect_output "device wifi --help shows subcommands" \
    "list" \
    $NCCLI device wifi --help

run_expect_output "device wifi --help shows connect" \
    "connect" \
    $NCCLI device wifi --help

run_expect_output "device wifi --help shows hotspot" \
    "hotspot" \
    $NCCLI device wifi --help

run_expect_output "device wifi --help shows radio" \
    "radio" \
    $NCCLI device wifi --help

# ===========================================
# RADIO COMMAND TESTS
# ===========================================

echo ""
echo "--- Radio Command Tests ---"

# Test: radio --help
run_expect_output "radio --help shows subcommands" \
    "wifi" \
    $NCCLI radio --help

# Test: radio all
run_expect_output "radio all shows WiFi status" \
    "WIFI" \
    $NCCLI radio all

# Test: radio all (terse mode)
run_expect_regex "radio all terse mode shows enabled/disabled" \
    "enabled|disabled" \
    $NCCLI -t radio all

# Test: radio wifi (get status)
run_expect_regex "radio wifi shows status" \
    "enabled|disabled" \
    $NCCLI radio wifi

# ===========================================
# WIFI INTERFACE TESTS (require WiFi hardware)
# ===========================================

echo ""
echo "--- WiFi Interface Tests ---"

if [ -n "$WIFI_IFACE" ]; then
    echo "Found WiFi interface: $WIFI_IFACE"

    # Test: device wifi list
    run_expect_success "device wifi list succeeds" \
        $NCCLI device wifi list

    # Test: device wifi list with specific interface
    run_expect_success "device wifi list on $WIFI_IFACE" \
        $NCCLI device wifi list "$WIFI_IFACE"

    # Test: device wifi list (terse mode)
    run_expect_success "device wifi list terse mode" \
        $NCCLI -t device wifi list

    # Test: device wifi list shows headers (non-terse)
    run_expect_output "device wifi list shows SSID header" \
        "SSID" \
        $NCCLI device wifi list

    run_expect_output "device wifi list shows SIGNAL header" \
        "SIGNAL" \
        $NCCLI device wifi list

    # Test: device wifi list with rescan
    run_expect_success "device wifi list with rescan" \
        $NCCLI device wifi list --rescan yes

    # Test: device status for WiFi interface
    run_expect_output "device status shows wifi type" \
        "wifi" \
        $NCCLI device status "$WIFI_IFACE"

    # Test: device show for WiFi interface
    run_expect_output "device show wifi interface shows GENERAL" \
        "GENERAL" \
        $NCCLI device show "$WIFI_IFACE"

else
    test_skip "device wifi list" "No WiFi interface found"
    test_skip "device wifi list on interface" "No WiFi interface found"
    test_skip "device wifi list terse" "No WiFi interface found"
    test_skip "device wifi list headers" "No WiFi interface found"
    test_skip "device wifi list rescan" "No WiFi interface found"
    test_skip "device status wifi type" "No WiFi interface found"
    test_skip "device show wifi interface" "No WiFi interface found"
fi

# ===========================================
# WIFI CONNECTION TESTS
# ===========================================

echo ""
echo "--- WiFi Connection Tests ---"

# Test: device wifi connect --help
run_expect_output "device wifi connect --help shows options" \
    "password" \
    $NCCLI device wifi connect --help

run_expect_output "device wifi connect --help shows hidden" \
    "hidden" \
    $NCCLI device wifi connect --help

# Test: connection add wifi --help
run_expect_output "connection add --help shows wifi type" \
    "wifi" \
    $NCCLI connection add --help

run_expect_output "connection add --help shows ssid" \
    "ssid" \
    $NCCLI connection add --help

# Test: connection add wifi validation (missing ssid)
run_expect_failure "connection add wifi without ssid fails" \
    $NCCLI connection add --type wifi --con-name test-wifi-no-ssid

# ===========================================
# WIFI HOTSPOT TESTS
# ===========================================

echo ""
echo "--- WiFi Hotspot Tests ---"

# Test: device wifi hotspot --help
run_expect_output "device wifi hotspot --help shows ssid option" \
    "ssid" \
    $NCCLI device wifi hotspot --help

run_expect_output "device wifi hotspot --help shows password option" \
    "password" \
    $NCCLI device wifi hotspot --help

run_expect_output "device wifi hotspot --help shows band option" \
    "band" \
    $NCCLI device wifi hotspot --help

run_expect_output "device wifi hotspot --help shows channel option" \
    "channel" \
    $NCCLI device wifi hotspot --help

# ===========================================
# RADIO CONTROL TESTS (via D-Bus to netctld)
# ===========================================

echo ""
echo "--- Radio Control Tests (via D-Bus) ---"

# Note: These tests run as normal user - netctld daemon handles privileged ops
if [ -n "$WIFI_IFACE" ]; then
    # Save current state
    ORIG_STATE=$($NCCLI -t radio wifi 2>/dev/null)

    # Test: radio wifi on
    run_expect_success "radio wifi on" \
        $NCCLI radio wifi on

    # Verify state
    run_expect_output "radio wifi is enabled after 'on'" \
        "enabled" \
        $NCCLI radio wifi

    # Test: device wifi radio on
    run_expect_success "device wifi radio on" \
        $NCCLI device wifi radio on

    # Test: device wifi radio off
    run_expect_success "device wifi radio off" \
        $NCCLI device wifi radio off

    # Restore original state
    if [ "$ORIG_STATE" = "enabled" ]; then
        $NCCLI radio wifi on > /dev/null 2>&1
    fi
else
    test_skip "radio wifi on" "No WiFi interface found"
    test_skip "radio wifi state check" "No WiFi interface found"
    test_skip "device wifi radio on" "No WiFi interface found"
    test_skip "device wifi radio off" "No WiFi interface found"
fi

# ===========================================
# WIFI VALIDATION TESTS
# ===========================================

echo ""
echo "--- WiFi Validation Tests ---"

# Test: Invalid SSID (too long - over 32 characters)
LONG_SSID="ThisSSIDIsWayTooLongForWiFiNetworks123"
run_expect_failure "connection add wifi with too long SSID fails" \
    $NCCLI connection add --type wifi --con-name test-long-ssid --ssid "$LONG_SSID"

# Test: Invalid password (too short - less than 8 chars)
run_expect_failure "connection add wifi with short password fails" \
    $NCCLI connection add --type wifi --con-name test-short-pwd --ssid "TestNetwork" --password "short"

# Test: Invalid password (too long - over 63 chars)
LONG_PASS="ThisPasswordIsWayTooLongForWPA2SecurityAndShouldFailValidation1234567890"
run_expect_failure "connection add wifi with too long password fails" \
    $NCCLI connection add --type wifi --con-name test-long-pwd --ssid "TestNetwork" --password "$LONG_PASS"

# Test: Invalid radio state
run_expect_failure "radio wifi with invalid state fails" \
    $NCCLI radio wifi invalid_state

# Test: device wifi radio with invalid state
run_expect_failure "device wifi radio with invalid state fails" \
    $NCCLI device wifi radio invalid_state

# Test: wifi connect to nonexistent interface
run_expect_failure "device wifi connect on nonexistent interface fails" \
    $NCCLI device wifi connect "TestNetwork" --ifname nonexistent_wifi_99

# ===========================================
# WIFI TERSE OUTPUT TESTS
# ===========================================

echo ""
echo "--- WiFi Terse Output Tests ---"

# Test: radio wifi terse
run_expect_success "radio wifi terse mode" \
    $NCCLI -t radio wifi

# Test: radio all terse
run_expect_success "radio all terse mode" \
    $NCCLI -t radio all

if [ -n "$WIFI_IFACE" ]; then
    # Test: device wifi list terse output format (colon-separated)
    test_start "device wifi list terse format"
    output=$($NCCLI -t device wifi list 2>&1)
    # Terse output should be colon-separated, not have headers
    if ! echo "$output" | grep -q "SSID"; then
        test_pass "device wifi list terse format"
    else
        # Empty scan results are OK too
        if [ -z "$output" ] || echo "$output" | grep -qE "^[^:]*:[^:]*:" 2>/dev/null; then
            test_pass "device wifi list terse format"
        else
            test_fail "device wifi list terse format" "Output contains headers in terse mode"
        fi
    fi
fi

# ===========================================
# SUMMARY
# ===========================================

echo ""
echo "WiFi command tests completed"
print_summary
