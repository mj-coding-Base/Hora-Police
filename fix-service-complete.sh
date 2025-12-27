#!/bin/bash
# Complete service file fix - removes all problematic settings

set -e

echo "=== Fixing Hora-Police Service File ==="

# Stop service
sudo systemctl stop hora-police 2>/dev/null || true

# Create minimal working service file
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

# Security hardening (minimal to avoid namespace issues)
NoNewPrivileges=true
ProtectSystem=full
ProtectHome=true
ReadOnlyPaths=/proc /sys
ReadWritePaths=/var/lib/hora-police /etc/hora-police /var/log/hora-police

[Install]
WantedBy=multi-user.target
EOF

# Reload
sudo systemctl daemon-reload

# Verify file
echo ""
echo "=== Service File Contents ==="
sudo cat /etc/systemd/system/hora-police.service

echo ""
echo "=== Key Settings ==="
echo "Type: $(sudo grep '^Type=' /etc/systemd/system/hora-police.service)"
echo "ProtectSystem: $(sudo grep '^ProtectSystem=' /etc/systemd/system/hora-police.service || echo 'not set')"
echo "PrivateTmp: $(sudo grep '^PrivateTmp=' /etc/systemd/system/hora-police.service || echo 'removed')"

echo ""
echo "✅ Service file updated"
echo ""
echo "Next steps:"
echo "1. Ensure binary exists: ls -lh /usr/local/bin/hora-police"
echo "2. Start service: sudo systemctl start hora-police"
echo "3. Check status: sudo systemctl status hora-police"

