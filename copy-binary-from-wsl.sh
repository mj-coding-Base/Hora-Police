#!/usr/bin/env bash
# Copy binary from WSL to VPS
# Run this script from WSL on your Windows machine

set -euo pipefail

VPS_IP="62.72.13.136"
VPS_USER="deploy"
BINARY_NAME="hora-police"
VPS_BINARY_PATH="/usr/local/bin/hora-police"
PROJECT_DIR="/mnt/f/Personal_Projects/Hora-Police"

echo "=== Hora-Police Binary Transfer Script ==="
echo "VPS: ${VPS_USER}@${VPS_IP}"
echo ""

# Check if we're in WSL
if [ ! -d "/mnt/c" ] && [ ! -d "/mnt/f" ]; then
    echo "⚠️  Warning: This script is designed for WSL"
    echo "If running on Linux, adjust PROJECT_DIR variable"
fi

# 1. Navigate to project directory
echo "[1/5] Checking project directory..."
if [ ! -d "${PROJECT_DIR}" ]; then
    echo "❌ Project directory not found: ${PROJECT_DIR}"
    echo ""
    echo "Please set PROJECT_DIR to your Hora-Police project path"
    echo "Or run from the project directory:"
    echo "  cd /path/to/Hora-Police"
    echo "  PROJECT_DIR=\$(pwd) ./copy-binary-from-wsl.sh"
    exit 1
fi

cd "${PROJECT_DIR}"
echo "✅ Project directory: ${PROJECT_DIR}"

# 2. Check if binary exists, build if not
echo ""
echo "[2/5] Checking for existing binary..."
BINARY_PATH="${PROJECT_DIR}/target/release/${BINARY_NAME}"

if [ ! -f "${BINARY_PATH}" ]; then
    echo "Binary not found, building..."
    echo "This may take 5-15 minutes..."
    
    # Check if Rust is available
    if ! command -v cargo >/dev/null 2>&1; then
        echo "❌ Rust/Cargo not found"
        echo "Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi
    
    # Build release binary
    cargo build --release
    
    if [ ! -f "${BINARY_PATH}" ]; then
        echo "❌ Build failed - binary not created"
        exit 1
    fi
    
    echo "✅ Build successful"
else
    echo "✅ Binary found: ${BINARY_PATH}"
    echo "File info:"
    ls -lh "${BINARY_PATH}"
    file "${BINARY_PATH}" || true
fi

# 3. Verify binary
echo ""
echo "[3/5] Verifying binary..."
if ! file "${BINARY_PATH}" | grep -q "ELF.*x86-64"; then
    echo "⚠️  Warning: Binary may not be x86-64 ELF"
    file "${BINARY_PATH}" || true
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Test binary locally if possible
if [ -x "${BINARY_PATH}" ]; then
    if "${BINARY_PATH}" --help >/dev/null 2>&1; then
        echo "✅ Binary is executable and responds to --help"
    else
        echo "⚠️  Binary exists but --help test failed (may be OK)"
    fi
fi

# 4. Copy to VPS
echo ""
echo "[4/5] Copying binary to VPS..."
echo "Target: ${VPS_USER}@${VPS_IP}:/tmp/${BINARY_NAME}"

scp "${BINARY_PATH}" "${VPS_USER}@${VPS_IP}:/tmp/${BINARY_NAME}"

if [ $? -eq 0 ]; then
    echo "✅ Binary copied successfully"
else
    echo "❌ SCP failed"
    echo "Check:"
    echo "  1. SSH key is set up"
    echo "  2. VPS is accessible"
    echo "  3. User has write access to /tmp"
    exit 1
fi

# 5. Install on VPS
echo ""
echo "[5/5] Installing binary on VPS..."
echo "This will:"
echo "  1. Move binary to ${VPS_BINARY_PATH}"
echo "  2. Set executable permissions"
echo "  3. Verify installation"

ssh "${VPS_USER}@${VPS_IP}" << 'ENDSSH'
set -e
BINARY_NAME="hora-police"
VPS_BINARY_PATH="/usr/local/bin/hora-police"

# Move and set permissions
sudo mv "/tmp/${BINARY_NAME}" "${VPS_BINARY_PATH}"
sudo chmod +x "${VPS_BINARY_PATH}"
sudo chown root:root "${VPS_BINARY_PATH}"

# Verify
echo ""
echo "=== Installation Verification ==="
if [ -f "${VPS_BINARY_PATH}" ]; then
    echo "✅ Binary installed at ${VPS_BINARY_PATH}"
    ls -lh "${VPS_BINARY_PATH}"
    file "${VPS_BINARY_PATH}" || true
    
    # Test execution
    if sudo "${VPS_BINARY_PATH}" --help >/dev/null 2>&1; then
        echo "✅ Binary executes successfully"
    else
        echo "⚠️  Binary installed but --help test failed"
    fi
else
    echo "❌ Installation failed"
    exit 1
fi
ENDSSH

if [ $? -eq 0 ]; then
    echo ""
    echo "=== Transfer Complete ==="
    echo "✅ Binary successfully installed on VPS"
    echo ""
    echo "Next steps:"
    echo "  1. SSH to VPS: ssh ${VPS_USER}@${VPS_IP}"
    echo "  2. Run diagnostic: cd /srv/Hora-Police && ./diagnose-binary.sh"
    echo "  3. Run fix script: ./fix-service-directories.sh"
    echo "  4. Check service: sudo systemctl status hora-police"
else
    echo ""
    echo "❌ Installation on VPS failed"
    echo "SSH to VPS and run manually:"
    echo "  sudo mv /tmp/${BINARY_NAME} ${VPS_BINARY_PATH}"
    echo "  sudo chmod +x ${VPS_BINARY_PATH}"
    exit 1
fi

