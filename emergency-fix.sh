#!/bin/bash
set -e

echo "=== Emergency Fix for Hora-Police ==="

# 1. Stop service
echo "[1/6] Stopping service..."
sudo systemctl stop hora-police 2>/dev/null || true

# 2. Fix service file
echo "[2/6] Fixing service file..."
sudo tee /etc/systemd/system/hora-police.service > /dev/null << 'EOF'
[Unit]
Description=Hora-Police Anti-Malware Daemon
After=network.target

[Service]
Type=simple
User=root
ExecStart=/usr/local/bin/hora-police /etc/hora-police/config.toml
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

# Resource limits
CPUQuota=15%
MemoryMax=128M
TasksMax=1024

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=full
ProtectHome=true
ReadOnlyPaths=/proc /sys
ReadWritePaths=/var/lib/hora-police /etc/hora-police /var/log/hora-police

[Install]
WantedBy=multi-user.target
EOF

# 3. Reload systemd
echo "[3/6] Reloading systemd..."
sudo systemctl daemon-reload

# 4. Build with minimal resources
echo "[4/6] Building (this may take 10-20 minutes with -j1)..."
cd /srv/Hora-Police
cargo clean
echo "  Building with single job, no LTO..."
RUSTFLAGS="-C opt-level=3" cargo build --release -j1 || {
    echo "  First attempt failed, trying even simpler..."
    cargo build --release -j1
}

# 5. Install
echo "[5/6] Installing binary..."
if [ -f "target/release/hora-police" ]; then
    sudo cp target/release/hora-police /usr/local/bin/hora-police
    sudo chmod +x /usr/local/bin/hora-police
    echo "  ✓ Binary installed successfully"
    ls -lh /usr/local/bin/hora-police
else
    echo "  ✗ Build failed - binary not found"
    echo "  Try: cargo build --release -j1 (without RUSTFLAGS)"
    exit 1
fi

# 6. Start service
echo "[6/6] Starting service..."
sudo systemctl start hora-police
sleep 3

# Check status
echo ""
echo "=== Service Status ==="
sudo systemctl status hora-police --no-pager -l | head -20

echo ""
echo "=== Recent Logs ==="
sudo journalctl -u hora-police -n 10 --no-pager

echo ""
if systemctl is-active --quiet hora-police; then
    echo "✅ Service is running!"
    echo "View logs: sudo journalctl -u hora-police -f"
else
    echo "❌ Service failed to start"
    echo "Check logs: sudo journalctl -u hora-police -n 50"
fi

