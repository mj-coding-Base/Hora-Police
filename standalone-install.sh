#!/usr/bin/env bash
# Standalone install script - can be created manually on VPS
# Doesn't require git pull or repo access
set -euo pipefail

BINARY_SOURCE="/tmp/hora-police"
BINARY_DEST="/usr/local/bin/hora-police"
REPO_DIR="/srv/Hora-Police"

echo "=== Hora-Police Binary Installation ==="

if [ ! -f "${BINARY_SOURCE}" ]; then
    echo "❌ Binary not found at ${BINARY_SOURCE}"
    echo ""
    echo "Please copy binary to /tmp/hora-police first:"
    echo "  From build: cp target/release/hora-police /tmp/hora-police"
    echo "  From remote: scp target/release/hora-police deploy@<VPS>:/tmp/hora-police"
    exit 1
fi

echo "✅ Binary found: ${BINARY_SOURCE}"
ls -lh "${BINARY_SOURCE}"

# 1. Stop service
echo ""
echo "[1/6] Stopping service..."
sudo systemctl stop hora-police 2>/dev/null || true

# 2. Install binary
echo "[2/6] Installing binary..."
sudo cp "${BINARY_SOURCE}" "${BINARY_DEST}"
sudo chown root:root "${BINARY_DEST}"
sudo chmod 755 "${BINARY_DEST}"
echo "  ✓ Binary installed to ${BINARY_DEST}"

# 3. Verify binary
echo "[3/6] Verifying binary..."
if sudo "${BINARY_DEST}" --help >/dev/null 2>&1; then
    echo "  ✓ Binary executes successfully"
    file "${BINARY_DEST}" || true
else
    echo "  ⚠️  Binary test failed (may need config file), but continuing..."
fi

# 4. Ensure directories exist
echo "[4/6] Ensuring required directories exist..."
sudo mkdir -p /etc/hora-police
sudo mkdir -p /var/lib/hora-police/quarantine
sudo mkdir -p /var/lib/hora-police/rollbacks
sudo mkdir -p /var/log/hora-police
sudo chown -R root:root /etc/hora-police /var/lib/hora-police /var/log/hora-police
sudo chmod 0755 /etc/hora-police /var/lib/hora-police /var/log/hora-police
sudo chmod 0700 /var/lib/hora-police/quarantine

# Install tmpfiles.d config if present
if [ -f "${REPO_DIR}/etc/tmpfiles.d/hora-police.conf" ]; then
    sudo cp "${REPO_DIR}/etc/tmpfiles.d/hora-police.conf" /etc/tmpfiles.d/hora-police.conf
    sudo systemd-tmpfiles --create /etc/tmpfiles.d/hora-police.conf || true
    echo "  ✓ tmpfiles.d configuration installed"
fi

# 5. Reload systemd
echo "[5/6] Reloading systemd..."
sudo systemctl daemon-reload
sudo systemctl reset-failed hora-police || true

# 6. Start service
echo "[6/6] Starting service..."
sudo systemctl start hora-police
sleep 3

# Check status
echo ""
echo "=== Service Status ==="
sudo systemctl status hora-police --no-pager -l | head -30 || true

# Check for errors
echo ""
echo "=== Checking for Errors ==="
ERRORS=$(sudo journalctl -u hora-police -n 50 --no-pager | grep -iE 'EXEC|NAMESPACE|Failed.*EXEC|error.*226|error.*203' || true)
if [ -n "${ERRORS}" ]; then
    echo "⚠️  Errors found in journal:"
    echo "${ERRORS}"
else
    echo "✅ No errors found in recent journal"
fi

# Final verification
echo ""
if sudo systemctl is-active --quiet hora-police; then
    echo "✅ SUCCESS! Service is running."
    echo ""
    echo "Next steps:"
    echo "  - View logs: sudo journalctl -u hora-police -f"
    echo "  - Check status: sudo systemctl status hora-police"
else
    echo "❌ Service failed to start"
    echo ""
    echo "Troubleshooting:"
    echo "  1. Check logs: sudo journalctl -u hora-police -n 50"
    echo "  2. Test binary: sudo ${BINARY_DEST} --help"
    exit 1
fi

