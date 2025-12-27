#!/bin/bash
# Quick fix script to create required directories and fix service

set -e

echo "=== Fixing Hora-Police Service Directories ==="

# Create all required directories
sudo mkdir -p /etc/hora-police
sudo mkdir -p /var/lib/hora-police
sudo mkdir -p /var/lib/hora-police/quarantine
sudo mkdir -p /var/log/hora-police

# Set permissions
sudo chown -R root:root /etc/hora-police /var/lib/hora-police /var/log/hora-police
sudo chmod 755 /etc/hora-police
sudo chmod 755 /var/lib/hora-police
sudo chmod 700 /var/lib/hora-police/quarantine
sudo chmod 755 /var/log/hora-police

# Install tmpfiles.d if available
if [ -f "etc/tmpfiles.d/hora-police.conf" ]; then
    sudo cp etc/tmpfiles.d/hora-police.conf /etc/tmpfiles.d/
    sudo systemd-tmpfiles --create /etc/tmpfiles.d/hora-police.conf 2>/dev/null || true
    echo "✅ tmpfiles.d configuration installed"
fi

# Stop service
sudo systemctl stop hora-police 2>/dev/null || true

# Reload systemd
sudo systemctl daemon-reload

echo ""
echo "✅ Directories created and permissions set"
echo ""
echo "Next steps:"
echo "  1. Ensure binary exists: ls -lh /usr/local/bin/hora-police"
echo "  2. Ensure config exists: ls -lh /etc/hora-police/config.toml"
echo "  3. Start service: sudo systemctl start hora-police"
echo "  4. Check status: sudo systemctl status hora-police"

