#!/bin/bash
# Complete installation script for pre-built binary

set -e

BINARY_SOURCE="/tmp/hora-police"

echo "=== Hora-Police Binary Installation ==="

# Check if binary exists
if [ ! -f "$BINARY_SOURCE" ]; then
    echo "❌ Binary not found at $BINARY_SOURCE"
    echo ""
    echo "Please transfer binary first:"
    echo "  From Windows (WSL): scp target/release/hora-police deploy@mail-server:/tmp/hora-police"
    echo "  Or use WinSCP/FileZilla to upload to /tmp/hora-police"
    exit 1
fi

echo "✅ Binary found: $BINARY_SOURCE"
ls -lh "$BINARY_SOURCE"

# Stop service
echo ""
echo "[1/6] Stopping service..."
sudo systemctl stop hora-police 2>/dev/null || true

# Install binary
echo "[2/6] Installing binary..."
sudo cp "$BINARY_SOURCE" /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
echo "  ✓ Binary installed to /usr/local/bin/hora-police"

# Verify binary
echo "[3/6] Verifying binary..."
if /usr/local/bin/hora-police --help >/dev/null 2>&1; then
    echo "  ✓ Binary works"
else
    echo "  ⚠️  Binary test failed, but continuing..."
fi

# Fix service file
echo "[4/6] Fixing service file..."
cd /srv/Hora-Police 2>/dev/null || cd ~/Hora-Police 2>/dev/null || {
    echo "  ⚠️  Cannot find project directory, fixing service manually..."
    sudo tee /etc/systemd/system/hora-police.service > /dev/null << 'SERVICEEOF'
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
CPUQuota=15%
MemoryMax=128M
TasksMax=1024
NoNewPrivileges=true
ProtectSystem=full
ProtectHome=true
ReadOnlyPaths=/proc /sys
ReadWritePaths=/var/lib/hora-police /etc/hora-police /var/log/hora-police

[Install]
WantedBy=multi-user.target
SERVICEEOF
    echo "  ✓ Service file created"
}

if [ -f "fix-service-complete.sh" ]; then
    chmod +x fix-service-complete.sh
    ./fix-service-complete.sh
else
    echo "  ⚠️  fix-service-complete.sh not found, using manual fix..."
    sudo tee /etc/systemd/system/hora-police.service > /dev/null << 'SERVICEEOF'
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
CPUQuota=15%
MemoryMax=128M
TasksMax=1024
NoNewPrivileges=true
ProtectSystem=full
ProtectHome=true
ReadOnlyPaths=/proc /sys
ReadWritePaths=/var/lib/hora-police /etc/hora-police /var/log/hora-police

[Install]
WantedBy=multi-user.target
SERVICEEOF
fi

# Reload systemd
echo "[5/6] Reloading systemd..."
sudo systemctl daemon-reload

# Start service
echo "[6/6] Starting service..."
sudo systemctl start hora-police
sleep 3

# Check status
echo ""
echo "=== Service Status ==="
sudo systemctl status hora-police --no-pager -l | head -20

echo ""
if systemctl is-active --quiet hora-police; then
    echo "✅ SUCCESS! Service is running."
    echo ""
    echo "Next steps:"
    echo "  - View logs: sudo journalctl -u hora-police -f"
    echo "  - Check status: sudo systemctl status hora-police"
    echo "  - Verify config: sudo cat /etc/hora-police/config.toml"
else
    echo "❌ Service failed to start"
    echo ""
    echo "Troubleshooting:"
    echo "  1. Check logs: sudo journalctl -u hora-police -n 50"
    echo "  2. Test binary: sudo /usr/local/bin/hora-police /etc/hora-police/config.toml"
    echo "  3. Check service file: sudo cat /etc/systemd/system/hora-police.service"
    exit 1
fi

