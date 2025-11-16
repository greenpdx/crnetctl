#!/bin/bash
#
# netctl Uninstall Script
# Removes netctl network management tool and all installed components
#

set -e  # Exit on error

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print functions
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    print_error "This script must be run as root (use sudo)"
    exit 1
fi

print_warning "=========================================="
print_warning "netctl Uninstallation"
print_warning "=========================================="
echo ""
print_warning "This will remove netctl and all its components from your system."
print_warning "Configuration files in /etc/netctl will be preserved."
echo ""
read -p "Are you sure you want to continue? (yes/no): " -r
echo ""

if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
    print_info "Uninstallation cancelled."
    exit 0
fi

print_info "Starting netctl uninstallation..."

# Configuration
PREFIX="${PREFIX:-/usr}"
BINDIR="${PREFIX}/bin"
SYSTEMD_DIR="/lib/systemd/system"
MAN1_DIR="${PREFIX}/share/man/man1"
MAN5_DIR="${PREFIX}/share/man/man5"
MAN7_DIR="${PREFIX}/share/man/man7"
DOC_DIR="${PREFIX}/share/doc/netctl"
CONFIG_DIR="/etc/netctl"
RUN_DIR="/run/netctl"
STATE_DIR="/var/lib/netctl"

# Stop and disable services
print_info "Stopping and disabling netctl services..."

if systemctl is-active --quiet netctl.service; then
    systemctl stop netctl.service || print_warning "Failed to stop netctl.service"
fi

if systemctl is-enabled --quiet netctl.service 2>/dev/null; then
    systemctl disable netctl.service || print_warning "Failed to disable netctl.service"
fi

# Stop any running connection services
for service in /etc/systemd/system/multi-user.target.wants/netctl@*.service; do
    if [ -e "$service" ]; then
        service_name=$(basename "$service")
        print_info "Stopping and disabling ${service_name}..."
        systemctl stop "$service_name" || true
        systemctl disable "$service_name" || true
    fi
done

# Stop any running auto-connect services
for service in /etc/systemd/system/multi-user.target.wants/netctl-auto@*.service; do
    if [ -e "$service" ]; then
        service_name=$(basename "$service")
        print_info "Stopping and disabling ${service_name}..."
        systemctl stop "$service_name" || true
        systemctl disable "$service_name" || true
    fi
done

print_success "Services stopped and disabled"

# Remove binaries
print_info "Removing binaries from ${BINDIR}..."
rm -f "${BINDIR}/netctl"
rm -f "${BINDIR}/nm-converter"
rm -f "${BINDIR}/libnccli"
print_success "Binaries removed"

# Remove systemd service files
print_info "Removing systemd service files from ${SYSTEMD_DIR}..."
rm -f "${SYSTEMD_DIR}/netctl.service"
rm -f "${SYSTEMD_DIR}/netctl@.service"
rm -f "${SYSTEMD_DIR}/netctl-auto@.service"
print_success "Systemd service files removed"

# Remove man pages
print_info "Removing man pages..."
rm -f "${MAN1_DIR}/netctl.1"
rm -f "${MAN1_DIR}/nm-converter.1"
rm -f "${MAN1_DIR}/libnccli.1"
rm -f "${MAN5_DIR}/netctl.nctl.5"
rm -f "${MAN7_DIR}/netctl-plugin.7"
print_success "Man pages removed"

# Remove documentation
print_info "Removing documentation..."
rm -rf "${DOC_DIR}"
print_success "Documentation removed"

# Remove tmpfiles.d configuration
print_info "Removing tmpfiles.d configuration..."
rm -f /etc/tmpfiles.d/netctl.conf
print_success "tmpfiles.d configuration removed"

# Remove runtime directory
print_info "Removing runtime directory..."
rm -rf "${RUN_DIR}"
print_success "Runtime directory removed"

# Ask about configuration and state directories
echo ""
print_warning "Configuration directory: ${CONFIG_DIR}"
print_warning "State directory: ${STATE_DIR}"
echo ""
read -p "Do you want to remove configuration and state directories? (yes/no): " -r
echo ""

if [[ $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
    print_info "Removing configuration directory..."
    rm -rf "${CONFIG_DIR}"
    print_success "Configuration directory removed"

    print_info "Removing state directory..."
    rm -rf "${STATE_DIR}"
    print_success "State directory removed"
else
    print_info "Configuration and state directories preserved:"
    print_info "  - ${CONFIG_DIR}"
    print_info "  - ${STATE_DIR}"
    print_warning "You can manually remove these directories later if needed."
fi

# Reload systemd
print_info "Reloading systemd daemon..."
systemctl daemon-reload
print_success "Systemd daemon reloaded"

# Update man database
print_info "Updating man database..."
if command -v mandb &> /dev/null; then
    mandb -q || print_warning "Failed to update man database (non-fatal)"
    print_success "Man database updated"
else
    print_warning "mandb not found, skipping man database update"
fi

# Print uninstallation summary
echo ""
print_success "=========================================="
print_success "netctl uninstallation completed!"
print_success "=========================================="
echo ""
print_info "Removed components:"
print_info "  - Binaries (netctl, nm-converter, libnccli)"
print_info "  - Systemd service files"
print_info "  - Man pages"
print_info "  - Documentation"
print_info "  - Runtime files"
echo ""

if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
    print_info "Preserved directories:"
    print_info "  - ${CONFIG_DIR}"
    print_info "  - ${STATE_DIR}"
    echo ""
fi

print_info "netctl has been successfully removed from your system."
echo ""
