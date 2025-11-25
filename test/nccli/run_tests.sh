#!/bin/bash
# run_tests.sh - Main test runner for nccli tests
#
# Usage:
#   ./run_tests.sh           # Run all tests
#   ./run_tests.sh general   # Run only general tests
#   ./run_tests.sh device    # Run only device tests
#   ./run_tests.sh connection # Run only connection tests
#   ./run_tests.sh vpn       # Run only VPN tests
#   ./run_tests.sh network   # Run only network service tests
#   ./run_tests.sh wifi      # Run only WiFi tests
#   ./run_tests.sh ap        # Run only Access Point tests
#   ./run_tests.sh dhcpc     # Run only DHCP client tests

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Change to project root
cd "$PROJECT_ROOT"

# Source helpers for colors and check functions
source "$SCRIPT_DIR/test_helpers.sh"

# Override NCCLI path to use project binary
export NCCLI="$PROJECT_ROOT/target/debug/nccli"
export NETCTLD="$PROJECT_ROOT/target/debug/netctld"

echo "============================================"
echo "nccli Integration Test Suite"
echo "============================================"
echo ""
echo "Project root: $PROJECT_ROOT"
echo "nccli binary: $NCCLI"
echo "netctld binary: $NETCTLD"
echo ""

# Check prerequisites
echo "Checking prerequisites..."

# Check if binaries exist
if [ ! -x "$NCCLI" ]; then
    echo -e "${RED}ERROR: nccli binary not found at $NCCLI${NC}"
    echo "Run: cargo build"
    exit 1
fi

if [ ! -x "$NETCTLD" ]; then
    echo -e "${RED}ERROR: netctld binary not found at $NETCTLD${NC}"
    echo "Run: cargo build"
    exit 1
fi

echo -e "${GREEN}✓${NC} Binaries found"

# Check if daemon is running
if ! check_daemon; then
    echo ""
    echo "To start the daemon, run in another terminal:"
    echo "  sudo $NETCTLD"
    echo ""
    exit 1
fi

echo -e "${GREEN}✓${NC} netctld daemon is running"

# Check root (info only - most nccli tests don't need root)
# nccli communicates with netctld daemon via D-Bus, so the daemon handles
# privileged operations. Root is only needed for:
# - Starting/stopping netctld itself
# - Running crdhcpc daemon directly
if check_root; then
    echo -e "${GREEN}✓${NC} Running as root"
else
    echo -e "${BLUE}i${NC} Running as non-root (nccli uses D-Bus to netctld for privileged ops)"
fi

echo ""

# Determine which tests to run
TEST_CATEGORY="${1:-all}"

run_test_file() {
    local test_file="$1"
    local test_name="$2"

    if [ -f "$test_file" ]; then
        echo ""
        echo "============================================"
        echo "Running $test_name tests..."
        echo "============================================"
        chmod +x "$test_file"
        bash "$test_file"
    else
        echo -e "${RED}ERROR: Test file not found: $test_file${NC}"
        return 1
    fi
}

# Run tests based on category
case "$TEST_CATEGORY" in
    all)
        run_test_file "$SCRIPT_DIR/test_general.sh" "General"
        run_test_file "$SCRIPT_DIR/test_device.sh" "Device"
        run_test_file "$SCRIPT_DIR/test_connection.sh" "Connection"
        run_test_file "$SCRIPT_DIR/test_vpn.sh" "VPN"
        run_test_file "$SCRIPT_DIR/test_network.sh" "Network Services"
        run_test_file "$SCRIPT_DIR/test_wifi.sh" "WiFi"
        run_test_file "$SCRIPT_DIR/test_ap.sh" "Access Point"
        run_test_file "$SCRIPT_DIR/test_dhcp_client.sh" "DHCP Client"
        run_test_file "$SCRIPT_DIR/test_dbus.sh" "D-Bus Communication"
        ;;
    general)
        run_test_file "$SCRIPT_DIR/test_general.sh" "General"
        ;;
    device)
        run_test_file "$SCRIPT_DIR/test_device.sh" "Device"
        ;;
    connection)
        run_test_file "$SCRIPT_DIR/test_connection.sh" "Connection"
        ;;
    vpn)
        run_test_file "$SCRIPT_DIR/test_vpn.sh" "VPN"
        ;;
    network)
        run_test_file "$SCRIPT_DIR/test_network.sh" "Network Services"
        ;;
    dbus)
        run_test_file "$SCRIPT_DIR/test_dbus.sh" "D-Bus Communication"
        ;;
    wifi)
        run_test_file "$SCRIPT_DIR/test_wifi.sh" "WiFi"
        ;;
    ap)
        run_test_file "$SCRIPT_DIR/test_ap.sh" "Access Point"
        ;;
    dhcpc)
        run_test_file "$SCRIPT_DIR/test_dhcp_client.sh" "DHCP Client"
        ;;
    *)
        echo "Unknown test category: $TEST_CATEGORY"
        echo ""
        echo "Available categories:"
        echo "  all        - Run all tests (default)"
        echo "  general    - General command tests"
        echo "  device     - Device command tests"
        echo "  connection - Connection command tests"
        echo "  vpn        - VPN command tests"
        echo "  network    - DHCP, DNS, Route tests"
        echo "  wifi       - WiFi command tests"
        echo "  ap         - Access Point tests"
        echo "  dhcpc      - DHCP client tests (crdhcpc integration)"
        echo "  dbus       - D-Bus communication tests"
        exit 1
        ;;
esac

# Print final summary
echo ""
echo "============================================"
echo "ALL TESTS COMPLETED"
echo "============================================"
print_summary

# Cleanup
cleanup_temp_files

# Exit with appropriate code
if [ $TESTS_FAILED -gt 0 ]; then
    exit 1
fi
exit 0
