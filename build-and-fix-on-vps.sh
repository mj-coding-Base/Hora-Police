#!/usr/bin/env bash
# Build binary on VPS and fix service
# Run this script on the VPS as deploy user

set -euo pipefail

REPO_DIR="/srv/Hora-Police"
BINARY="/usr/local/bin/hora-police"
SERVICE_UNIT="/etc/systemd/system/hora-police.service"

echo "=== Hora-Police Build and Fix Script ==="
echo ""

# 1. Check if we're in the right directory
if [ ! -d "${REPO_DIR}" ]; then
    echo "❌ Repository not found at ${REPO_DIR}"
    exit 1
fi

cd "${REPO_DIR}"

# 2. Check Rust installation
echo "[1/7] Checking Rust installation..."
if ! command -v cargo >/dev/null 2>&1; then
    echo "❌ Rust/Cargo not found"
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    rustup default stable
    echo "✅ Rust installed"
else
    echo "✅ Rust found: $(cargo --version)"
    source "$HOME/.cargo/env" || true
fi

# 3. Create required directories
echo ""
echo "[2/7] Creating required directories..."
sudo mkdir -p /etc/hora-police
sudo mkdir -p /var/lib/hora-police/quarantine
sudo mkdir -p /var/log/hora-police
sudo chown -R root:root /etc/hora-police /var/lib/hora-police /var/log/hora-police
sudo chmod 0755 /etc/hora-police /var/lib/hora-police /var/log/hora-police
sudo chmod 0700 /var/lib/hora-police/quarantine
echo "✅ Directories created"

# 4. Ensure config exists
echo ""
echo "[3/7] Checking configuration..."
if [ ! -f "/etc/hora-police/config.toml" ]; then
    if [ -f "${REPO_DIR}/config.toml.example" ]; then
        sudo cp "${REPO_DIR}/config.toml.example" /etc/hora-police/config.toml
        sudo chmod 0644 /etc/hora-police/config.toml
        echo "✅ Config created from example"
    else
        echo "⚠️  No config.toml.example found"
    fi
else
    echo "✅ Config exists"
fi

# 5. Build binary (with memory optimization)
echo ""
echo "[4/7] Building binary..."
echo "This may take 10-20 minutes depending on system resources..."
echo "Using single job to minimize memory usage..."

# Stop service if running
sudo systemctl stop hora-police || true

# Clean previous build artifacts to save space
if [ -d "target" ]; then
    echo "Cleaning previous build..."
    cargo clean || true
fi

# Build with minimal memory usage
echo "Building release binary..."
RUSTFLAGS="-C opt-level=3" cargo build --release -j1

# Check if build succeeded
if [ ! -f "target/release/hora-police" ]; then
    echo "❌ Build failed - binary not created"
    echo "Check build output above for errors"
    exit 1
fi

echo "✅ Build successful"
ls -lh target/release/hora-police

# 6. Install binary
echo ""
echo "[5/7] Installing binary..."
sudo cp target/release/hora-police "${BINARY}"
sudo chmod +x "${BINARY}"
sudo chown root:root "${BINARY}"

# Verify installation
if [ -f "${BINARY}" ]; then
    echo "✅ Binary installed at ${BINARY}"
    file "${BINARY}" || true
    
    # Test execution
    if sudo "${BINARY}" --help >/dev/null 2>&1; then
        echo "✅ Binary executes successfully"
    else
        echo "⚠️  Binary installed but --help test failed (may need config)"
    fi
else
    echo "❌ Installation failed"
    exit 1
fi

# 7. Install tmpfiles.d config
echo ""
echo "[6/7] Installing tmpfiles.d configuration..."
if [ -f "${REPO_DIR}/etc/tmpfiles.d/hora-police.conf" ]; then
    sudo cp "${REPO_DIR}/etc/tmpfiles.d/hora-police.conf" /etc/tmpfiles.d/
    sudo systemd-tmpfiles --create /etc/tmpfiles.d/hora-police.conf || true
    echo "✅ tmpfiles.d config installed"
fi

# 8. Fix systemd service
echo ""
echo "[7/7] Fixing systemd service..."

# Backup existing service file
if [ -f "${SERVICE_UNIT}" ]; then
    sudo cp "${SERVICE_UNIT}" "${SERVICE_UNIT}.bak-$(date -u +%Y%m%dT%H%M%SZ)" || true
fi

# Install correct service file
sudo tee "${SERVICE_UNIT}" > /dev/null <<'UNIT'
[Unit]
Description=Hora-Police Anti-Malware Daemon
After=network.target
StartLimitIntervalSec=300
StartLimitBurst=5

[Service]
Type=simple
User=root
ExecStart=/usr/local/bin/hora-police /etc/hora-police/config.toml
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal
CPUQuota=15%
MemoryMax=128M
TasksMax=1024
NoNewPrivileges=true
PrivateTmp=false
ProtectSystem=strict
ProtectHome=true
ReadOnlyPaths=/proc /sys
ReadWritePaths=/var/lib/hora-police /etc/hora-police /var/log/hora-police

[Install]
WantedBy=multi-user.target
UNIT

# Reload systemd
sudo systemctl daemon-reload
sudo systemctl reset-failed hora-police || true

# Enable service
sudo systemctl enable hora-police

# Start service
echo ""
echo "=== Starting service ==="
sudo systemctl start hora-police
sleep 3

# Check status
echo ""
echo "=== Service Status ==="
sudo systemctl status -l hora-police --no-pager || true

# Check for errors
echo ""
echo "=== Checking for Errors ==="
ERRORS=$(sudo journalctl -u hora-police -n 50 --no-pager | grep -iE 'NAMESPACE|EXEC|Failed.*EXEC|error.*226|error.*203' || true)
if [ -n "${ERRORS}" ]; then
    echo "⚠️  Errors found:"
    echo "${ERRORS}"
else
    echo "✅ No errors found in recent journal"
fi

# Final verification
echo ""
echo "=== Final Verification ==="
if sudo systemctl is-active --quiet hora-police; then
    echo "✅ Service is running"
    echo ""
    echo "=== Summary ==="
    echo "✅ Binary built and installed"
    echo "✅ Directories created"
    echo "✅ Service configured and started"
    echo ""
    echo "Check status with: sudo systemctl status hora-police"
    echo "View logs with: sudo journalctl -u hora-police -f"
else
    echo "❌ Service is not running"
    echo "Check logs: sudo journalctl -u hora-police -n 50"
    exit 1
fi


