#!/usr/bin/env bash
# Install build dependencies for Hora-Police
set -euo pipefail

echo "=== Installing Build Dependencies ==="

# Update package list
echo "[1/4] Updating package list..."
sudo apt update

# Install essential build tools
echo "[2/4] Installing build-essential (includes gcc, make, etc.)..."
sudo apt install -y build-essential

# Install Rust-specific dependencies
echo "[3/4] Installing Rust dependencies..."
sudo apt install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    ca-certificates \
    curl

# Verify installation
echo "[4/4] Verifying installation..."
if command -v cc >/dev/null 2>&1; then
    echo "✅ C compiler (cc) found: $(which cc)"
    cc --version | head -1
else
    echo "❌ C compiler not found after installation"
    exit 1
fi

if command -v cargo >/dev/null 2>&1; then
    echo "✅ Cargo found: $(cargo --version)"
else
    echo "⚠️  Cargo not found. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    rustup default stable
    echo "✅ Rust installed: $(cargo --version)"
fi

echo ""
echo "✅ All build dependencies installed!"
echo ""
echo "You can now run:"
echo "  ./build-lowmem.sh"

