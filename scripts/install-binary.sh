#!/usr/bin/env bash
# Install Hora-Police binary and systemd service
# Verifies binary exists, installs it, configures systemd, and starts the service
set -euo pipefail

BINARY_SOURCE="/tmp/hora-police"
BINARY_DEST="/usr/local/bin/hora-police"
REPO_DIR="/srv/Hora-Police"
SERVICE_SOURCE="${REPO_DIR}/systemd/hora-police.service"
SERVICE_DEST="/etc/systemd/system/hora-police.service"
TMPFILES_SOURCE="${REPO_DIR}/etc/tmpfiles.d/hora-police.conf"
TMPFILES_DEST="/etc/tmpfiles.d/hora-police.conf"

echo "=== Hora-Police Binary Installation ==="

# Verify binary exists
if [ ! -f "${BINARY_SOURCE}" ]; then
    echo "❌ ERROR: Binary not found at ${BINARY_SOURCE}"
    echo ""
    echo "Please ensure binary exists at /tmp/hora-police:"
    echo "  From build: cp target/release/hora-police /tmp/hora-police"
    echo "  From remote: scp target/release/hora-police deploy@<VPS>:/tmp/hora-police"
    exit 1
fi

echo "✅ Binary found: ${BINARY_SOURCE}"
ls -lh "${BINARY_SOURCE}"

# Verify service unit file exists
if [ ! -f "${SERVICE_SOURCE}" ]; then
    echo "❌ ERROR: Service unit file not found at ${SERVICE_SOURCE}"
    exit 1
fi

# Verify tmpfiles config exists
if [ ! -f "${TMPFILES_SOURCE}" ]; then
    echo "❌ ERROR: tmpfiles config not found at ${TMPFILES_SOURCE}"
    exit 1
fi

# 1. Install binary
echo ""
echo "[1/6] Installing binary..."
sudo cp "${BINARY_SOURCE}" "${BINARY_DEST}"
sudo chown root:root "${BINARY_DEST}"
sudo chmod 755 "${BINARY_DEST}"
echo "  ✓ Binary installed to ${BINARY_DEST}"

# 2. Install service unit file
echo "[2/6] Installing systemd service unit..."
sudo cp "${SERVICE_SOURCE}" "${SERVICE_DEST}"
sudo chmod 644 "${SERVICE_DEST}"
echo "  ✓ Service unit installed to ${SERVICE_DEST}"

# 3. Install tmpfiles config
echo "[3/6] Installing tmpfiles.d configuration..."
sudo cp "${TMPFILES_SOURCE}" "${TMPFILES_DEST}"
sudo chmod 644 "${TMPFILES_DEST}"
echo "  ✓ tmpfiles.d config installed to ${TMPFILES_DEST}"

# 4. Create directories via tmpfiles
echo "[4/6] Creating directories via systemd-tmpfiles..."
sudo systemd-tmpfiles --create "${TMPFILES_DEST}" || {
    echo "⚠️  systemd-tmpfiles --create failed, creating directories manually..."
    sudo mkdir -p /var/lib/hora-police/quarantine
    sudo mkdir -p /etc/hora-police
    sudo mkdir -p /var/log/hora-police
    sudo chown -R root:root /var/lib/hora-police /etc/hora-police /var/log/hora-police
    sudo chmod 0755 /var/lib/hora-police /etc/hora-police /var/log/hora-police
    sudo chmod 0700 /var/lib/hora-police/quarantine
}
echo "  ✓ Directories created"

# 5. Reload systemd and enable service
echo "[5/6] Reloading systemd daemon..."
sudo systemctl daemon-reload
sudo systemctl reset-failed hora-police 2>/dev/null || true
sudo systemctl enable hora-police
echo "  ✓ systemd reloaded, service enabled"

# 6. Start service
echo "[6/6] Starting hora-police service..."
sudo systemctl start hora-police
sleep 2

# Capture and print status
echo ""
echo "=== systemctl status hora-police ==="
sudo systemctl status hora-police --no-pager -l || true

echo ""
echo "=== journalctl -u hora-police (last 50 lines) ==="
sudo journalctl -u hora-police -n 50 --no-pager || true

# Final verification
echo ""
if sudo systemctl is-active --quiet hora-police; then
    echo "✅ SUCCESS! Service is running."
    exit 0
else
    echo "❌ Service failed to start"
    echo ""
    echo "Check logs: sudo journalctl -u hora-police -n 100"
    exit 1
fi
