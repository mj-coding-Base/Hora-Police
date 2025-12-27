#!/bin/bash
# Quick fix script for Hora-Police service issues

set -e

echo "=== Fixing Hora-Police Service Issues ==="

# 1. Stop service
echo "[1/7] Stopping service..."
sudo systemctl stop hora-police 2>/dev/null || true

# 2. Fix service file - change ProtectSystem from strict to full
echo "[2/7] Updating service file..."
if grep -q "ProtectSystem=strict" /etc/systemd/system/hora-police.service; then
    sudo sed -i 's/ProtectSystem=strict/ProtectSystem=full/' /etc/systemd/system/hora-police.service
    echo "  ✓ Changed ProtectSystem=strict to ProtectSystem=full"
fi

# Add ReadOnlyPaths if not present
if ! grep -q "ReadOnlyPaths=/proc /sys" /etc/systemd/system/hora-police.service; then
    sudo sed -i '/ReadWritePaths=/a ReadOnlyPaths=/proc /sys' /etc/systemd/system/hora-police.service
    echo "  ✓ Added ReadOnlyPaths for /proc and /sys"
fi

# 3. Reload systemd
echo "[3/7] Reloading systemd..."
sudo systemctl daemon-reload

# 4. Check if binary exists
echo "[4/7] Checking binary..."
if [ ! -f "/usr/local/bin/hora-police" ]; then
    echo "  ⚠ Binary not found, attempting to build..."
    cd /srv/Hora-Police || cd ~/Hora-Police || { echo "  ✗ Cannot find project directory"; exit 1; }
    
    # Build with limited parallelism
    echo "  Building with 2 jobs (to avoid OOM)..."
    cargo build --release -j2 2>&1 | tail -20 || {
        echo "  Build failed, trying without LTO..."
        RUSTFLAGS="-C opt-level=3" cargo build --release -j2
    }
    
    # Install
    sudo cp target/release/hora-police /usr/local/bin/hora-police
    sudo chmod +x /usr/local/bin/hora-police
    echo "  ✓ Binary installed"
else
    echo "  ✓ Binary exists"
fi

# 5. Check configuration
echo "[5/7] Checking configuration..."
if [ ! -f "/etc/hora-police/config.toml" ]; then
    echo "  ⚠ Config file not found, creating from example..."
    sudo mkdir -p /etc/hora-police
    if [ -f "/srv/Hora-Police/config.examples/hostinger_kvm4.toml" ]; then
        sudo cp /srv/Hora-Police/config.examples/hostinger_kvm4.toml /etc/hora-police/config.toml
    elif [ -f "~/Hora-Police/config.examples/hostinger_kvm4.toml" ]; then
        sudo cp ~/Hora-Police/config.examples/hostinger_kvm4.toml /etc/hora-police/config.toml
    fi
    echo "  ✓ Config file created (please edit it)"
else
    echo "  ✓ Config file exists"
fi

# 6. Test binary manually (non-blocking)
echo "[6/7] Testing binary (timeout 5s)..."
timeout 5 sudo /usr/local/bin/hora-police /etc/hora-police/config.toml 2>&1 | head -5 || {
    echo "  ⚠ Binary test timed out or failed (this is OK if it starts)"
}

# 7. Start service
echo "[7/7] Starting service..."
sudo systemctl start hora-police
sleep 2

# Check status
echo ""
echo "=== Service Status ==="
sudo systemctl status hora-police --no-pager -l | head -15

echo ""
echo "=== Next Steps ==="
echo "1. View logs: sudo journalctl -u hora-police -f"
echo "2. Check status: sudo systemctl status hora-police"
echo "3. If still failing, check: sudo journalctl -u hora-police -n 50"

