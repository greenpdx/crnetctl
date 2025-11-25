#!/bin/bash
# test_helpers.sh - Common test utilities for nccli tests

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

# Path to binaries
NCCLI="${NCCLI:-./target/debug/nccli}"
NETCTLD="${NETCTLD:-./target/debug/netctld}"

# Check if daemon is running
check_daemon() {
    if ! pgrep -x "netctld" > /dev/null 2>&1; then
        echo -e "${RED}ERROR: netctld daemon is not running${NC}"
        echo "Start it with: sudo $NETCTLD"
        return 1
    fi
    return 0
}

# Check if running as root
# NOTE: nccli handles privilege checks internally and will print
# a clear error message if root is required for an operation.
# Tests don't need to pre-check - they'll fail with nccli's error.
check_root() {
    if [ "$EUID" -ne 0 ]; then
        return 1
    fi
    return 0
}

# Log test start
test_start() {
    local test_name="$1"
    echo -e "${BLUE}[TEST]${NC} $test_name"
}

# Log test pass
test_pass() {
    local test_name="$1"
    echo -e "${GREEN}[PASS]${NC} $test_name"
    ((TESTS_PASSED++))
}

# Log test fail
test_fail() {
    local test_name="$1"
    local reason="${2:-}"
    echo -e "${RED}[FAIL]${NC} $test_name"
    if [ -n "$reason" ]; then
        echo -e "       Reason: $reason"
    fi
    ((TESTS_FAILED++))
}

# Log test skip
test_skip() {
    local test_name="$1"
    local reason="${2:-}"
    echo -e "${YELLOW}[SKIP]${NC} $test_name"
    if [ -n "$reason" ]; then
        echo -e "       Reason: $reason"
    fi
    ((TESTS_SKIPPED++))
}

# Run a command and check exit code
run_expect_success() {
    local test_name="$1"
    shift
    local cmd="$@"

    test_start "$test_name"

    local output
    local exit_code
    output=$($cmd 2>&1)
    exit_code=$?

    if [ $exit_code -eq 0 ]; then
        test_pass "$test_name"
        echo "$output"
        return 0
    else
        test_fail "$test_name" "Exit code: $exit_code"
        echo "$output"
        return 1
    fi
}

# Run a command and check exit code (expect failure)
run_expect_failure() {
    local test_name="$1"
    shift
    local cmd="$@"

    test_start "$test_name"

    local output
    local exit_code
    output=$($cmd 2>&1)
    exit_code=$?

    if [ $exit_code -ne 0 ]; then
        test_pass "$test_name"
        return 0
    else
        test_fail "$test_name" "Expected failure but got success"
        echo "$output"
        return 1
    fi
}

# Run a command and check output contains string
run_expect_output() {
    local test_name="$1"
    local expected="$2"
    shift 2
    local cmd="$@"

    test_start "$test_name"

    local output
    local exit_code
    output=$($cmd 2>&1)
    exit_code=$?

    if [ $exit_code -ne 0 ]; then
        test_fail "$test_name" "Command failed with exit code: $exit_code"
        echo "$output"
        return 1
    fi

    if echo "$output" | grep -q "$expected"; then
        test_pass "$test_name"
        return 0
    else
        test_fail "$test_name" "Output did not contain: $expected"
        echo "Actual output:"
        echo "$output"
        return 1
    fi
}

# Run a command and check output matches regex
run_expect_regex() {
    local test_name="$1"
    local pattern="$2"
    shift 2
    local cmd="$@"

    test_start "$test_name"

    local output
    local exit_code
    output=$($cmd 2>&1)
    exit_code=$?

    if [ $exit_code -ne 0 ]; then
        test_fail "$test_name" "Command failed with exit code: $exit_code"
        echo "$output"
        return 1
    fi

    if echo "$output" | grep -qE "$pattern"; then
        test_pass "$test_name"
        return 0
    else
        test_fail "$test_name" "Output did not match pattern: $pattern"
        echo "Actual output:"
        echo "$output"
        return 1
    fi
}

# Run command with timeout
run_with_timeout() {
    local timeout_secs="$1"
    shift
    timeout "$timeout_secs" "$@"
}

# Print test summary
print_summary() {
    echo ""
    echo "============================================"
    echo "TEST SUMMARY"
    echo "============================================"
    echo -e "${GREEN}Passed:${NC}  $TESTS_PASSED"
    echo -e "${RED}Failed:${NC}  $TESTS_FAILED"
    echo -e "${YELLOW}Skipped:${NC} $TESTS_SKIPPED"
    echo "============================================"

    if [ $TESTS_FAILED -gt 0 ]; then
        return 1
    fi
    return 0
}

# Create a temporary test config file
create_temp_config() {
    local name="$1"
    local content="$2"
    local temp_file=$(mktemp /tmp/nccli_test_XXXXXX.nctl)
    echo "$content" > "$temp_file"
    echo "$temp_file"
}

# Clean up temporary files
cleanup_temp_files() {
    rm -f /tmp/nccli_test_*.nctl
}

# Check if interface exists
interface_exists() {
    local iface="$1"
    ip link show "$iface" > /dev/null 2>&1
}

# Get first available ethernet interface
get_ethernet_interface() {
    ip -o link show | grep -v "lo:" | grep -v "wl" | head -1 | awk -F': ' '{print $2}'
}

# Get first available wifi interface
get_wifi_interface() {
    ip -o link show | grep "wl" | head -1 | awk -F': ' '{print $2}'
}
