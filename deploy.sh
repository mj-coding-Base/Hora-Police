#!/usr/bin/env bash
# Hora-Police Deployment Script
# Idempotent deployment script for Ubuntu VPS
#
# Usage:
#   ./deploy.sh --repo=https://github.com/mj-coding-Base/Hora-Police --branch=main [OPTIONS]
#
# Options:
#   --repo=URL          GitHub repository URL (default: from GIT_REPO env var)
#   --branch=BRANCH     Git branch to deploy (default: main, or GIT_BRANCH env var)
#   --force             Force update even if local changes exist
#   --rollback          Rollback to previous version
#   --dry-run           Print actions without executing
#   --skip-build        Use pre-copied /tmp/hora-police binary
#   --debug             Increase logging verbosity
#
# Environment Variables:
#   GIT_REPO            Repository URL (overrides --repo)
#   GIT_BRANCH          Branch name (overrides --branch)
#   BUILD_HOST          Remote host for fallback build (user@host)
#   SWAP_SIZE_GB        Swap file size in GB (default: 4)
#   INSTALL_PATH        Binary installation path (default: /usr/local/bin/hora-police)
#   PREBUILT_URL        URL to download prebuilt binary if build fails
#
# Examples:
#   ./deploy.sh --repo=https://github.com/mj-coding-Base/Hora-Police --branch=main
#   BUILD_HOST=user@build-server ./deploy.sh
#   ./deploy.sh --skip-build  # Use pre-built binary at /tmp/hora-police
#   ./deploy.sh --rollback    # Restore previous version

set -euo pipefail

# Configuration
REPO_DIR="/srv/Hora-Police"
LOG_FILE="/tmp/hora-police-deploy.log"
BINARY_SOURCE="/tmp/hora-police"
BINARY_DEST="${INSTALL_PATH:-/usr/local/bin/hora-police}"
SERVICE_UNIT="/etc/systemd/system/hora-police.service"
TMPFILES_CONF="/etc/tmpfiles.d/hora-police.conf"
SWAP_SIZE_GB="${SWAP_SIZE_GB:-4}"
SWAP_FILE="/swapfile-hora-police"

# State tracking
DRY_RUN=false
SKIP_BUILD=false
FORCE=false
ROLLBACK=false
DEBUG=false
SWAP_CREATED=false
BINARY_BACKUP=""
SERVICE_BACKUP=""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Logging functions
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOG_FILE"
}

log_info() {
    log "INFO: $*"
    echo -e "${GREEN}✓${NC} $*"
}

log_warn() {
    log "WARN: $*"
    echo -e "${YELLOW}⚠${NC} $*"
}

log_error() {
    log "ERROR: $*"
    echo -e "${RED}✗${NC} $*" >&2
}

log_debug() {
    if [ "$DEBUG" = true ]; then
        log "DEBUG: $*"
        echo "  [DEBUG] $*"
    fi
}

# Error handling
error_exit() {
    log_error "$1"
    exit "${2:-1}"
}

# Execute command (respects dry-run)
run_cmd() {
    local cmd="$*"
    log_debug "Executing: $cmd"
    if [ "$DRY_RUN" = true ]; then
        echo "  [DRY-RUN] $cmd"
        return 0
    fi
    eval "$cmd"
}

# Parse command-line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --repo=*)
                GIT_REPO="${1#*=}"
                shift
                ;;
            --branch=*)
                GIT_BRANCH="${1#*=}"
                shift
                ;;
            --force)
                FORCE=true
                shift
                ;;
            --rollback)
                ROLLBACK=true
                shift
                ;;
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --skip-build)
                SKIP_BUILD=true
                shift
                ;;
            --debug)
                DEBUG=true
                shift
                ;;
            *)
                error_exit "Unknown option: $1" 1
                ;;
        esac
    done
}

# Check if running as root (for certain operations)
check_sudo() {
    if ! sudo -n true 2>/dev/null; then
        log_warn "Some operations require sudo. You may be prompted for password."
    fi
}

# Install build dependencies
install_dependencies() {
    log_info "Checking build dependencies..."
    
    local deps=(
        "build-essential"
        "pkg-config"
        "libssl-dev"
        "libsqlite3-dev"
        "lld"
        "curl"
        "ca-certificates"
    )
    
    local missing=()
    for dep in "${deps[@]}"; do
        if ! dpkg -l | grep -q "^ii.*$dep"; then
            missing+=("$dep")
        fi
    done
    
    if [ ${#missing[@]} -gt 0 ]; then
        log_info "Installing missing dependencies: ${missing[*]}"
        run_cmd "sudo apt-get update -qq"
        run_cmd "sudo apt-get install -y ${missing[*]}"
    else
        log_info "All build dependencies are installed"
    fi
}

# Setup Rust toolchain
setup_rust() {
    log_info "Checking Rust toolchain..."
    
    if command -v cargo >/dev/null 2>&1; then
        log_info "Rust is installed: $(cargo --version)"
        if [ -f "$HOME/.cargo/env" ]; then
            source "$HOME/.cargo/env"
        fi
        return 0
    fi
    
    log_info "Installing Rust via rustup..."
    run_cmd "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
    
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
        run_cmd "rustup default stable"
        log_info "Rust installed: $(cargo --version)"
    else
        error_exit "Failed to install Rust" 1
    fi
}

# Check available memory
get_memory_gb() {
    local mem_kb
    mem_kb=$(grep MemAvailable /proc/meminfo | awk '{print $2}')
    echo $((mem_kb / 1024 / 1024))
}

# Create swap file
create_swap() {
    local mem_gb
    mem_gb=$(get_memory_gb)
    
    if [ "$mem_gb" -lt 4 ]; then
        log_info "Low memory detected (${mem_gb}GB). Creating ${SWAP_SIZE_GB}GB swap file..."
        
        if [ -f "$SWAP_FILE" ]; then
            log_info "Swap file already exists"
            return 0
        fi
        
        run_cmd "sudo fallocate -l ${SWAP_SIZE_GB}G $SWAP_FILE || sudo dd if=/dev/zero of=$SWAP_FILE bs=1M count=$((SWAP_SIZE_GB * 1024))"
        run_cmd "sudo chmod 600 $SWAP_FILE"
        run_cmd "sudo mkswap $SWAP_FILE"
        run_cmd "sudo swapon $SWAP_FILE"
        
        SWAP_CREATED=true
        log_info "Swap file created and enabled"
    else
        log_info "Sufficient memory available (${mem_gb}GB)"
    fi
}

# Remove swap file (if we created it)
remove_swap() {
    if [ "$SWAP_CREATED" = true ]; then
        log_info "Removing temporary swap file..."
        run_cmd "sudo swapoff $SWAP_FILE 2>/dev/null || true"
        run_cmd "sudo rm -f $SWAP_FILE"
        log_info "Swap file removed"
    fi
}

# Clone or update repository
update_repository() {
    local repo="${GIT_REPO:-https://github.com/mj-coding-Base/Hora-Police}"
    local branch="${GIT_BRANCH:-main}"
    
    log_info "Updating repository: $repo (branch: $branch)"
    
    if [ -d "$REPO_DIR/.git" ]; then
        log_info "Repository exists, updating..."
        cd "$REPO_DIR"
        
        if [ "$FORCE" = false ]; then
            # Check for local changes
            if ! git diff-index --quiet HEAD -- 2>/dev/null; then
                log_warn "Local changes detected. Use --force to overwrite."
                if [ "$DRY_RUN" = false ]; then
                    error_exit "Repository has local changes. Use --force to proceed." 1
                fi
            fi
        fi
        
        run_cmd "git fetch origin"
        run_cmd "git reset --hard origin/$branch"
        log_info "Repository updated to $branch"
    else
        log_info "Cloning repository..."
        run_cmd "sudo mkdir -p $(dirname "$REPO_DIR")"
        run_cmd "sudo git clone -b $branch $repo $REPO_DIR"
        run_cmd "sudo chown -R $USER:$USER $REPO_DIR"
        log_info "Repository cloned"
    fi
    
    cd "$REPO_DIR"
}

# Build binary locally
build_local() {
    log_info "Building binary locally..."
    cd "$REPO_DIR"
    
    # Source cargo env
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    fi
    
    # Try build-lowmem.sh first
    if [ -f "./build-lowmem.sh" ]; then
        log_info "Using build-lowmem.sh script"
        chmod +x ./build-lowmem.sh
        run_cmd "./build-lowmem.sh"
    else
        log_info "Building with cargo (low-memory flags)..."
        export RUSTFLAGS="-C codegen-units=1 -C opt-level=1 -C linker=lld"
        export CARGO_BUILD_JOBS=1
        run_cmd "cargo build --release -j1 --locked"
    fi
    
    # Verify binary exists
    if [ -f "target/release/hora-police" ]; then
        log_info "Build successful"
        run_cmd "cp target/release/hora-police $BINARY_SOURCE"
        return 0
    else
        return 1
    fi
}

# Build on remote host
build_remote() {
    local build_host="${BUILD_HOST:-}"
    
    if [ -z "$build_host" ]; then
        return 1
    fi
    
    log_info "Attempting remote build on $build_host..."
    cd "$REPO_DIR"
    
    # Transfer source to remote host
    run_cmd "tar czf /tmp/hora-police-src.tar.gz --exclude=target --exclude=.git ."
    run_cmd "scp /tmp/hora-police-src.tar.gz $build_host:/tmp/"
    
    # Build on remote
    run_cmd "ssh $build_host 'cd /tmp && tar xzf hora-police-src.tar.gz && cd hora-police-src && source \$HOME/.cargo/env && cargo build --release -j1 && cp target/release/hora-police /tmp/hora-police'"
    
    # Download binary
    run_cmd "scp $build_host:/tmp/hora-police $BINARY_SOURCE"
    run_cmd "rm -f /tmp/hora-police-src.tar.gz"
    
    log_info "Remote build successful"
    return 0
}

# Download prebuilt binary
download_prebuilt() {
    local url="${PREBUILT_URL:-}"
    
    if [ -z "$url" ]; then
        return 1
    fi
    
    log_info "Downloading prebuilt binary from $url..."
    run_cmd "curl -fL -o $BINARY_SOURCE $url"
    
    if [ -f "$BINARY_SOURCE" ]; then
        run_cmd "chmod +x $BINARY_SOURCE"
        log_info "Prebuilt binary downloaded"
        return 0
    else
        return 1
    fi
}

# Build with fallbacks
build_binary() {
    if [ "$SKIP_BUILD" = true ]; then
        log_info "Skipping build, using pre-copied binary"
        if [ ! -f "$BINARY_SOURCE" ]; then
            error_exit "Binary not found at $BINARY_SOURCE" 1
        fi
        return 0
    fi
    
    log_info "Building binary (with fallbacks)..."
    
    # Try local build first
    if build_local; then
        return 0
    fi
    
    log_warn "Local build failed, trying fallbacks..."
    
    # Try remote build
    if build_remote; then
        return 0
    fi
    
    # Try download prebuilt
    if download_prebuilt; then
        return 0
    fi
    
    error_exit "All build methods failed. Check logs at $LOG_FILE" 1
}

# Backup existing binary
backup_binary() {
    if [ -f "$BINARY_DEST" ]; then
        BINARY_BACKUP="${BINARY_DEST}.backup.$(date +%Y%m%d_%H%M%S)"
        log_info "Backing up existing binary to $BINARY_BACKUP"
        run_cmd "sudo cp $BINARY_DEST $BINARY_BACKUP"
    fi
}

# Install binary
install_binary() {
    log_info "Installing binary to $BINARY_DEST..."
    
    if [ ! -f "$BINARY_SOURCE" ]; then
        error_exit "Binary not found at $BINARY_SOURCE" 1
    fi
    
    backup_binary
    
    # Atomic install
    run_cmd "sudo install -m755 $BINARY_SOURCE $BINARY_DEST"
    
    # Verify
    if [ -f "$BINARY_DEST" ] && [ -x "$BINARY_DEST" ]; then
        log_info "Binary installed successfully"
        log_debug "Binary info: $(file $BINARY_DEST)"
    else
        error_exit "Binary installation failed" 1
    fi
}

# Setup directories and permissions
setup_directories() {
    log_info "Setting up directories and permissions..."
    
    local dirs=(
        "/etc/hora-police:755"
        "/var/lib/hora-police:755"
        "/var/lib/hora-police/quarantine:700"
        "/var/log/hora-police:755"
    )
    
    for dir_spec in "${dirs[@]}"; do
        local dir="${dir_spec%%:*}"
        local mode="${dir_spec##*:}"
        run_cmd "sudo mkdir -p $dir"
        run_cmd "sudo chown root:root $dir"
        run_cmd "sudo chmod $mode $dir"
    done
    
    log_info "Directories created with correct permissions"
}

# Install service files
install_service_files() {
    log_info "Installing service files..."
    cd "$REPO_DIR"
    
    # Backup existing service file
    if [ -f "$SERVICE_UNIT" ]; then
        SERVICE_BACKUP="${SERVICE_UNIT}.backup.$(date +%Y%m%d_%H%M%S)"
        log_info "Backing up existing service file to $SERVICE_BACKUP"
        run_cmd "sudo cp $SERVICE_UNIT $SERVICE_BACKUP"
    fi
    
    # Install service unit
    if [ -f "hora-police.service" ]; then
        run_cmd "sudo cp hora-police.service $SERVICE_UNIT"
        log_info "Service unit installed"
    else
        log_warn "hora-police.service not found in repo, skipping"
    fi
    
    # Install tmpfiles.d config
    if [ -f "etc/tmpfiles.d/hora-police.conf" ]; then
        run_cmd "sudo cp etc/tmpfiles.d/hora-police.conf $TMPFILES_CONF"
        run_cmd "sudo systemd-tmpfiles --create $TMPFILES_CONF"
        log_info "tmpfiles.d configuration installed"
    else
        log_warn "tmpfiles.d config not found in repo, skipping"
    fi
}

# Enable and start service
start_service() {
    log_info "Enabling and starting service..."
    
    run_cmd "sudo systemctl daemon-reload"
    run_cmd "sudo systemctl enable hora-police"
    run_cmd "sudo systemctl restart hora-police"
    
    # Wait a bit for service to start
    sleep 3
    
    # Verify service is active
    if run_cmd "systemctl is-active --quiet hora-police"; then
        log_info "Service is running"
    else
        log_error "Service failed to start"
        run_cmd "sudo systemctl status hora-police --no-pager -l | head -30"
        return 1
    fi
}

# Verify deployment
verify_deployment() {
    log_info "Verifying deployment..."
    
    local status=""
    local binary_ok=false
    local service_ok=false
    
    # Check binary
    if [ -f "$BINARY_DEST" ] && [ -x "$BINARY_DEST" ]; then
        binary_ok=true
        log_info "Binary verified: $BINARY_DEST"
    else
        log_error "Binary verification failed"
    fi
    
    # Check service
    if systemctl is-active --quiet hora-police 2>/dev/null; then
        service_ok=true
        log_info "Service is active"
    else
        log_error "Service is not active"
    fi
    
    # Capture logs
    log_info "Capturing service logs..."
    run_cmd "sudo journalctl -u hora-police -n 100 --no-pager >> $LOG_FILE 2>&1 || true"
    
    # Generate status report
    echo ""
    echo "=========================================="
    echo "Deployment Status Report"
    echo "=========================================="
    echo "Binary: $BINARY_DEST"
    echo "  Status: $([ "$binary_ok" = true ] && echo "OK" || echo "FAILED")"
    echo "  Size: $(du -h $BINARY_DEST 2>/dev/null | cut -f1 || echo "N/A")"
    echo ""
    echo "Service: hora-police"
    echo "  Status: $([ "$service_ok" = true ] && echo "ACTIVE" || echo "INACTIVE")"
    echo ""
    echo "Last 50 log lines:"
    echo "----------------------------------------"
    run_cmd "sudo journalctl -u hora-police -n 50 --no-pager || true"
    echo "=========================================="
    echo ""
    echo "Full deployment log: $LOG_FILE"
    
    if [ "$binary_ok" = true ] && [ "$service_ok" = true ]; then
        log_info "Deployment successful!"
        return 0
    else
        log_error "Deployment verification failed"
        return 1
    fi
}

# Rollback to previous version
rollback_deployment() {
    log_info "Rolling back to previous version..."
    
    # Find latest backup
    local latest_binary_backup
    latest_binary_backup=$(ls -t ${BINARY_DEST}.backup.* 2>/dev/null | head -1)
    
    local latest_service_backup
    latest_service_backup=$(ls -t ${SERVICE_UNIT}.backup.* 2>/dev/null | head -1)
    
    if [ -z "$latest_binary_backup" ]; then
        error_exit "No binary backup found for rollback" 1
    fi
    
    log_info "Restoring binary from $latest_binary_backup"
    run_cmd "sudo cp $latest_binary_backup $BINARY_DEST"
    run_cmd "sudo chmod +x $BINARY_DEST"
    
    if [ -n "$latest_service_backup" ]; then
        log_info "Restoring service file from $latest_service_backup"
        run_cmd "sudo cp $latest_service_backup $SERVICE_UNIT"
    fi
    
    run_cmd "sudo systemctl daemon-reload"
    run_cmd "sudo systemctl restart hora-police"
    
    sleep 3
    
    if systemctl is-active --quiet hora-police; then
        log_info "Rollback successful"
        return 0
    else
        error_exit "Rollback failed - service not active" 1
    fi
}

# Main deployment function
main_deploy() {
    log_info "Starting Hora-Police deployment..."
    
    check_sudo
    install_dependencies
    setup_rust
    create_swap
    
    if [ "$ROLLBACK" = true ]; then
        rollback_deployment
        return 0
    fi
    
    update_repository
    build_binary
    install_binary
    setup_directories
    install_service_files
    start_service
    
    if verify_deployment; then
        remove_swap
        log_info "Deployment completed successfully"
        return 0
    else
        log_error "Deployment completed with errors"
        return 1
    fi
}

# Main entry point
main() {
    # Initialize log file
    echo "=== Hora-Police Deployment Log ===" > "$LOG_FILE"
    echo "Started: $(date)" >> "$LOG_FILE"
    echo "" >> "$LOG_FILE"
    
    parse_args "$@"
    
    if [ "$DRY_RUN" = true ]; then
        log_info "DRY-RUN MODE: No changes will be made"
    fi
    
    if main_deploy; then
        exit 0
    else
        exit 1
    fi
}

# Run main function
main "$@"

