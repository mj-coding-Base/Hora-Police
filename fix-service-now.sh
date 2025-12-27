#!/bin/bash
# Quick service file fix - run this on the server

echo "=== Fixing Hora-Police Service File ==="

# Stop service
sudo systemctl stop hora-police 2>/dev/null || true

# Backup existing
if [ -f /etc/systemd/system/hora-police.service ]; then
    sudo cp /etc/systemd/system/hora-police.service /etc/systemd/system/hora-police.service.backup
fi

# Create fixed service file
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
EOF

# Reload
sudo systemctl daemon-reload

# Verify
echo ""
echo "=== Service File Fixed ==="
echo "Type: $(sudo grep '^Type=' /etc/systemd/system/hora-police.service)"
echo "PrivateTmp: $(sudo grep '^PrivateTmp=' /etc/systemd/system/hora-police.service || echo 'removed (good)')"
echo ""
echo "✅ Service file updated. Now install binary and start service."

