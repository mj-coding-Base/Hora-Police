#!/usr/bin/env bash
# Hora-Police Update Script
# Performs complete update: pull code, build, install, restart with safety features
# Usage: ./scripts/update.sh [--dry-run] [--force] [--branch=BRANCH]
set -euo pipefail

# Configuration
REPO_DIR="/srv/Hora-Police"
BINARY_PATH="/usr/local/bin/hora-police"
BACKUP_DIR="/var/lib/hora-police/backups"
LOG_FILE="/var/log/hora-police/update.log"
SERVICE_NAME="hora-police"
MAX_BACKUPS=5

# Options
DRY_RUN=false
FORCE=false
BRANCH=""
BACKUP_FILE=""
ROLLBACK_NEEDED=false

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Logging function
log() {
    local level="$1"
    shift
    local message="$*"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[${timestamp}] [${level}] ${message}" | tee -a "${LOG_FILE}"
}

log_info() {
    log "INFO" "$@"
    echo -e "${GREEN}ℹ${NC} $*"
}

log_warn() {
    log "WARN" "$@"
    echo -e "${YELLOW}⚠${NC} $*"
}

log_error() {
    log "ERROR" "$@"
    echo -e "${RED}❌${NC} $*" >&2
}

# Parse arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --force)
                FORCE=true
                shift
                ;;
            --branch=*)
                BRANCH="${1#*=}"
                shift
                ;;
            --branch)
                BRANCH="$2"
                shift 2
                ;;
            *)
                log_error "Unknown option: $1"
                echo "Usage: $0 [--dry-run] [--force] [--branch=BRANCH]"
                exit 1
                ;;
        esac
    done
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    local missing=()
    
    if ! command -v git >/dev/null 2>&1; then
        missing+=("git")
    fi
    
    if ! command -v cargo >/dev/null 2>&1; then
        missing+=("cargo")
    fi
    
    if ! command -v systemctl >/dev/null 2>&1; then
        missing+=("systemctl")
    fi
    
    if [ ${#missing[@]} -gt 0 ]; then
        log_error "Missing prerequisites: ${missing[*]}"
        exit 1
    fi
    
    # Check repo directory
    if [ ! -d "${REPO_DIR}" ]; then
        log_error "Repository directory not found: ${REPO_DIR}"
        exit 1
    fi
    
    # Ensure log directory exists
    sudo mkdir -p "$(dirname "${LOG_FILE}")"
    sudo touch "${LOG_FILE}"
    sudo chmod 644 "${LOG_FILE}"
    
    log_info "Prerequisites check passed"
}

# Get current version from installed binary
get_current_version() {
    if [ ! -f "${BINARY_PATH}" ]; then
        echo "not-installed"
        return
    fi
    
    if [ ! -x "${BINARY_PATH}" ]; then
        echo "not-executable"
        return
    fi
    
    # Try to get version from binary
    local version_output
    version_output=$("${BINARY_PATH}" --version 2>/dev/null || echo "")
    
    if [ -z "${version_output}" ]; then
        # Fallback: use file modification time or hash
        echo "unknown-$(stat -c %Y "${BINARY_PATH}" 2>/dev/null || echo "0")"
    else
        # Extract version number
        echo "${version_output}" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1 || echo "unknown"
    fi
}

# Get latest version from git/Cargo.toml
get_latest_version() {
    cd "${REPO_DIR}"
    
    # Try git describe first (if tags exist)
    local git_version
    git_version=$(git describe --tags --always 2>/dev/null || echo "")
    
    if [ -n "${git_version}" ] && [[ "${git_version}" =~ ^v?[0-9]+\.[0-9]+\.[0-9]+ ]]; then
        echo "${git_version#v}"
        return
    fi
    
    # Fallback: parse Cargo.toml
    if [ -f "Cargo.toml" ]; then
        local cargo_version
        cargo_version=$(grep -E '^version\s*=' Cargo.toml | head -1 | sed -E 's/.*version\s*=\s*"([^"]+)".*/\1/' || echo "")
        if [ -n "${cargo_version}" ]; then
            # Append commit hash for uniqueness
            local commit_hash
            commit_hash=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
            echo "${cargo_version}-${commit_hash}"
            return
        fi
    fi
    
    # Last resort: use commit hash
    local commit_hash
    commit_hash=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
    echo "git-${commit_hash}"
}

# Backup current binary
backup_binary() {
    if [ ! -f "${BINARY_PATH}" ]; then
        log_warn "No existing binary to backup"
        return
    fi
    
    log_info "Creating backup..."
    
    # Ensure backup directory exists
    sudo mkdir -p "${BACKUP_DIR}"
    sudo chmod 755 "${BACKUP_DIR}"
    
    # Create backup with timestamp
    local timestamp
    timestamp=$(date '+%Y%m%d-%H%M%S')
    BACKUP_FILE="${BACKUP_DIR}/hora-police-${timestamp}"
    
    sudo cp "${BINARY_PATH}" "${BACKUP_FILE}"
    sudo chmod 644 "${BACKUP_FILE}"
    
    log_info "Backup created: ${BACKUP_FILE}"
    
    # Cleanup old backups (keep last MAX_BACKUPS)
    cleanup_old_backups
}

# Cleanup old backups
cleanup_old_backups() {
    local backup_count
    backup_count=$(find "${BACKUP_DIR}" -name "hora-police-*" -type f 2>/dev/null | wc -l)
    
    if [ "${backup_count}" -gt "${MAX_BACKUPS}" ]; then
        log_info "Cleaning up old backups (keeping last ${MAX_BACKUPS})..."
        find "${BACKUP_DIR}" -name "hora-police-*" -type f -printf '%T@ %p\n' 2>/dev/null | \
            sort -rn | \
            tail -n +$((MAX_BACKUPS + 1)) | \
            cut -d' ' -f2- | \
            xargs -r sudo rm -f
    fi
}

# Rollback to backup
rollback() {
    if [ -z "${BACKUP_FILE}" ] || [ ! -f "${BACKUP_FILE}" ]; then
        log_error "No backup available for rollback"
        return 1
    fi
    
    log_warn "Rolling back to previous version..."
    
    # Stop service
    sudo systemctl stop "${SERVICE_NAME}" 2>/dev/null || true
    
    # Restore backup
    sudo cp "${BACKUP_FILE}" "${BINARY_PATH}"
    sudo chmod 755 "${BINARY_PATH}"
    sudo chown root:root "${BINARY_PATH}"
    
    # Restart service
    sudo systemctl daemon-reload
    sudo systemctl start "${SERVICE_NAME}" || true
    
    log_warn "Rollback completed. Service restarted with previous version."
    return 0
}

# Update code from git
update_code() {
    log_info "Updating code from git..."
    
    cd "${REPO_DIR}"
    
    # Fetch latest changes
    git fetch origin || {
        log_error "Failed to fetch from git"
        return 1
    }
    
    # Checkout specified branch or use current
    if [ -n "${BRANCH}" ]; then
        log_info "Switching to branch: ${BRANCH}"
        git checkout "${BRANCH}" || {
            log_error "Failed to checkout branch: ${BRANCH}"
            return 1
        }
    fi
    
    # Pull latest changes
    git pull origin "$(git branch --show-current)" || {
        log_error "Failed to pull latest changes"
        return 1
    }
    
    log_info "Code updated successfully"
}

# Build binary
build_binary() {
    log_info "Building binary..."
    
    cd "${REPO_DIR}"
    
    # Make build script executable
    chmod +x build-lowmem.sh 2>/dev/null || true
    
    # Run build
    if ! ./build-lowmem.sh; then
        log_error "Build failed"
        return 1
    fi
    
    # Verify binary was created
    if [ ! -f "target/release/hora-police" ]; then
        log_error "Build completed but binary not found"
        return 1
    fi
    
    log_info "Build completed successfully"
}

# Install binary
install_binary() {
    log_info "Installing binary..."
    
    cd "${REPO_DIR}"
    
    # Copy to /tmp for installation script
    sudo cp target/release/hora-police /tmp/hora-police
    sudo chmod 755 /tmp/hora-police
    
    # Make install script executable
    chmod +x scripts/install-binary.sh 2>/dev/null || true
    
    # Run installation
    if ! sudo ./scripts/install-binary.sh; then
        log_error "Installation failed"
        return 1
    fi
    
    log_info "Binary installed successfully"
}

# Verify update
verify_update() {
    log_info "Verifying update..."
    
    # Check binary exists and is executable
    if [ ! -f "${BINARY_PATH}" ] || [ ! -x "${BINARY_PATH}" ]; then
        log_error "Binary verification failed: file missing or not executable"
        return 1
    fi
    
    # Check service is running
    sleep 2
    if ! sudo systemctl is-active --quiet "${SERVICE_NAME}"; then
        log_error "Service verification failed: service is not running"
        return 1
    fi
    
    # Check service status
    local status_output
    status_output=$(sudo systemctl status "${SERVICE_NAME}" --no-pager -l 2>&1 || true)
    
    if echo "${status_output}" | grep -qiE "failed|error|EXEC|NAMESPACE"; then
        log_error "Service verification failed: errors detected in status"
        echo "${status_output}" | head -20
        return 1
    fi
    
    log_info "Update verification passed"
    return 0
}

# Main update workflow
main() {
    parse_args "$@"
    
    log_info "=== Hora-Police Update Script ==="
    log_info "Dry-run: ${DRY_RUN}, Force: ${FORCE}, Branch: ${BRANCH:-current}"
    
    # Check prerequisites
    check_prerequisites
    
    # Get versions
    local current_version
    local latest_version
    current_version=$(get_current_version)
    latest_version=$(get_latest_version)
    
    log_info "Current version: ${current_version}"
    log_info "Latest version: ${latest_version}"
    
    # Version comparison
    if [ "${current_version}" = "${latest_version}" ] && [ "${FORCE}" = false ]; then
        log_info "Already up-to-date. Use --force to update anyway."
        exit 2
    fi
    
    if [ "${DRY_RUN}" = true ]; then
        log_info "=== DRY-RUN MODE ==="
        log_info "Would update from ${current_version} to ${latest_version}"
        log_info "Would backup current binary"
        log_info "Would pull latest code"
        log_info "Would build new binary"
        log_info "Would install and restart service"
        exit 3
    fi
    
    # Set up error trap for rollback
    trap 'if [ $? -ne 0 ] && [ "${ROLLBACK_NEEDED}" = true ]; then rollback; fi' ERR
    
    # Backup
    backup_binary
    ROLLBACK_NEEDED=true
    
    # Update code
    update_code || {
        log_error "Code update failed, rolling back..."
        rollback
        exit 1
    }
    
    # Build
    build_binary || {
        log_error "Build failed, rolling back..."
        rollback
        exit 1
    }
    
    # Install
    install_binary || {
        log_error "Installation failed, rolling back..."
        rollback
        exit 1
    }
    
    # Verify
    if ! verify_update; then
        log_error "Verification failed, rolling back..."
        rollback
        exit 4
    fi
    
    # Success
    ROLLBACK_NEEDED=false
    trap - ERR
    
    log_info "=== Update completed successfully ==="
    log_info "Updated from ${current_version} to ${latest_version}"
    
    # Show final status
    echo ""
    echo "=== Service Status ==="
    sudo systemctl status "${SERVICE_NAME}" --no-pager -l | head -15 || true
    
    exit 0
}

# Run main function
main "$@"

