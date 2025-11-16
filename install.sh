#!/bin/bash
#
# netctl Installation Script
# Installs netctl network management tool and all required components
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

print_info "Starting netctl installation..."

# Check for required tools
print_info "Checking for required build tools..."

if ! command -v cargo &> /dev/null; then
    print_error "cargo not found. Please install Rust: https://rustup.rs/"
    exit 1
fi

if ! command -v systemctl &> /dev/null; then
    print_error "systemctl not found. This system doesn't appear to use systemd."
    exit 1
fi

print_success "Required build tools found"

# Get the script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# Configuration
PREFIX="${PREFIX:-/usr}"
BINDIR="${PREFIX}/bin"
SYSTEMD_DIR="/lib/systemd/system"
MAN1_DIR="${PREFIX}/share/man/man1"
MAN5_DIR="${PREFIX}/share/man/man5"
MAN7_DIR="${PREFIX}/share/man/man7"
DOC_DIR="${PREFIX}/share/doc/netctl"
EXAMPLES_DIR="${DOC_DIR}/examples"
CONFIG_DIR="/etc/netctl"
PLUGINS_DIR="${CONFIG_DIR}/plugins"
CONNECTIONS_DIR="${CONFIG_DIR}/connections"
RUN_DIR="/run/netctl"
STATE_DIR="/var/lib/netctl"

print_info "Installation directories:"
print_info "  Binaries:     ${BINDIR}"
print_info "  Systemd:      ${SYSTEMD_DIR}"
print_info "  Man pages:    ${PREFIX}/share/man"
print_info "  Config:       ${CONFIG_DIR}"
print_info "  Documentation: ${DOC_DIR}"

# Build the project
print_info "Building netctl..."
cargo build --release

if [ ! -f "target/release/netctl" ]; then
    print_error "Build failed: netctl binary not found"
    exit 1
fi

print_success "Build completed successfully"

# Install binaries
print_info "Installing binaries to ${BINDIR}..."
install -D -m 755 target/release/netctl "${BINDIR}/netctl"
install -D -m 755 target/release/nm-converter "${BINDIR}/nm-converter"
install -D -m 755 target/release/libnccli "${BINDIR}/libnccli"
print_success "Binaries installed"

# Install systemd service files
print_info "Installing systemd service files to ${SYSTEMD_DIR}..."
install -D -m 644 systemd/netctl.service "${SYSTEMD_DIR}/netctl.service"
install -D -m 644 systemd/netctl@.service "${SYSTEMD_DIR}/netctl@.service"
install -D -m 644 systemd/netctl-auto@.service "${SYSTEMD_DIR}/netctl-auto@.service"
print_success "Systemd service files installed"

# Create configuration directories
print_info "Creating configuration directories..."
mkdir -p "${CONFIG_DIR}"
mkdir -p "${PLUGINS_DIR}"
mkdir -p "${CONNECTIONS_DIR}"
mkdir -p "${STATE_DIR}"
print_success "Configuration directories created"

# Create runtime directory (will be recreated by tmpfiles.d on reboot)
print_info "Creating runtime directory..."
mkdir -p "${RUN_DIR}"
chmod 755 "${RUN_DIR}"
print_success "Runtime directory created"

# Install man pages
print_info "Installing man pages..."
if [ -d "docs" ]; then
    mkdir -p "${MAN1_DIR}"
    mkdir -p "${MAN5_DIR}"
    mkdir -p "${MAN7_DIR}"

    [ -f "docs/netctl.1" ] && install -D -m 644 docs/netctl.1 "${MAN1_DIR}/netctl.1"
    [ -f "docs/nm-converter.1" ] && install -D -m 644 docs/nm-converter.1 "${MAN1_DIR}/nm-converter.1"
    [ -f "docs/libnccli.1" ] && install -D -m 644 docs/libnccli.1 "${MAN1_DIR}/libnccli.1"
    [ -f "docs/netctl.nctl.5" ] && install -D -m 644 docs/netctl.nctl.5 "${MAN5_DIR}/netctl.nctl.5"
    [ -f "docs/netctl-plugin.7" ] && install -D -m 644 docs/netctl-plugin.7 "${MAN7_DIR}/netctl-plugin.7"

    print_success "Man pages installed"
else
    print_warning "docs directory not found, skipping man page installation"
fi

# Install documentation and examples
print_info "Installing documentation and examples..."
mkdir -p "${DOC_DIR}"
mkdir -p "${EXAMPLES_DIR}"

# Install README and documentation
[ -f "README.md" ] && install -D -m 644 README.md "${DOC_DIR}/README.md"
[ -f "LICENSE" ] && install -D -m 644 LICENSE "${DOC_DIR}/LICENSE"
[ -f "LICENSE-APACHE" ] && install -D -m 644 LICENSE-APACHE "${DOC_DIR}/LICENSE-APACHE"
[ -f "LICENSE-MIT" ] && install -D -m 644 LICENSE-MIT "${DOC_DIR}/LICENSE-MIT"

# Install example configurations
if [ -d "config/examples" ]; then
    cp -r config/examples/*.nctl "${EXAMPLES_DIR}/" 2>/dev/null || true
    cp -r config/examples/*.nmconnection "${EXAMPLES_DIR}/" 2>/dev/null || true
    cp -r config/examples/*.toml "${EXAMPLES_DIR}/" 2>/dev/null || true
    print_success "Examples installed"
else
    print_warning "config/examples directory not found, skipping example installation"
fi

# Create tmpfiles.d configuration for runtime directory
print_info "Creating tmpfiles.d configuration..."
cat > /etc/tmpfiles.d/netctl.conf << 'EOF'
# netctl runtime directory
d /run/netctl 0755 root root -
EOF
print_success "tmpfiles.d configuration created"

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

# Print installation summary
echo ""
print_success "=========================================="
print_success "netctl installation completed successfully!"
print_success "=========================================="
echo ""
print_info "Installed components:"
print_info "  - netctl binary (${BINDIR}/netctl)"
print_info "  - nm-converter binary (${BINDIR}/nm-converter)"
print_info "  - libnccli binary (${BINDIR}/libnccli)"
print_info "  - Systemd services:"
print_info "    * netctl.service (main daemon)"
print_info "    * netctl@.service (connection template)"
print_info "    * netctl-auto@.service (auto-connection)"
print_info "  - Configuration directory: ${CONFIG_DIR}"
print_info "  - Examples: ${EXAMPLES_DIR}"
echo ""
print_info "Next steps:"
print_info "  1. Review example configurations in ${EXAMPLES_DIR}"
print_info "  2. Create connection profiles in ${CONNECTIONS_DIR}"
print_info "  3. Start the netctl service:"
print_info "     sudo systemctl start netctl"
print_info "  4. Enable netctl to start on boot:"
print_info "     sudo systemctl enable netctl"
print_info "  5. Check status:"
print_info "     sudo systemctl status netctl"
echo ""
print_info "For more information, see:"
print_info "  - man netctl"
print_info "  - man netctl.nctl"
print_info "  - ${DOC_DIR}/README.md"
echo ""
print_warning "Note: If you were using NetworkManager, you may want to disable it:"
print_warning "  sudo systemctl stop NetworkManager"
print_warning "  sudo systemctl disable NetworkManager"
echo ""
