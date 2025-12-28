#!/usr/bin/env bash
# Check Hora-Police service status and configuration
set -euo pipefail

SERVICE_NAME="hora-police"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"

echo "=== Hora-Police Service Check ==="
echo ""

# Check if service file exists
if [ ! -f "${SERVICE_FILE}" ]; then
    echo "❌ Service file not found: ${SERVICE_FILE}"
    exit 1
fi

echo "[1/3] Parsing ExecStart from service unit..."
EXEC_START=$(sudo systemctl show "${SERVICE_NAME}" -p ExecStart --value 2>/dev/null || \
    grep "^ExecStart=" "${SERVICE_FILE}" | cut -d'=' -f2- | head -1)
if [ -z "${EXEC_START}" ]; then
    echo "  ⚠️  Could not parse ExecStart"
else
    echo "  ExecStart: ${EXEC_START}"
    
    # Extract binary path (first word before space)
    BINARY_PATH=$(echo "${EXEC_START}" | awk '{print $1}')
    echo "  Binary path: ${BINARY_PATH}"
    
    # Verify binary exists and is executable
    if [ -f "${BINARY_PATH}" ]; then
        if [ -x "${BINARY_PATH}" ]; then
            echo "  ✅ Binary exists and is executable"
            ls -lh "${BINARY_PATH}"
        else
            echo "  ❌ Binary exists but is not executable"
            echo "     Fix: sudo chmod +x ${BINARY_PATH}"
        fi
    else
        echo "  ❌ Binary not found at: ${BINARY_PATH}"
        echo "     Install binary first: sudo ./scripts/install-binary.sh"
    fi
fi

echo ""
echo "[2/3] Service unit file content:"
sudo systemctl cat "${SERVICE_NAME}" 2>/dev/null || cat "${SERVICE_FILE}"

echo ""
echo "[3/3] Last 50 journal lines:"
sudo journalctl -u "${SERVICE_NAME}" -n 50 --no-pager || echo "  ⚠️  Could not read journal"

echo ""
echo "=== Service Status ==="
sudo systemctl status "${SERVICE_NAME}" --no-pager -l | head -20 || true

