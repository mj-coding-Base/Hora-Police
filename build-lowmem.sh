#!/usr/bin/env bash
# Low-memory build script for systems with limited RAM
# Uses single job, disables LTO, lower optimization to reduce memory usage
set -euo pipefail

echo "🛡️  Building Hora-Police with low-memory profile..."

# Check for C compiler
if ! command -v cc >/dev/null 2>&1; then
    echo "❌ C compiler (cc) not found!"
    echo ""
    echo "Please install build dependencies first:"
    echo "  chmod +x install-build-deps.sh"
    echo "  ./install-build-deps.sh"
    echo ""
    echo "Or manually:"
    echo "  sudo apt update"
    echo "  sudo apt install -y build-essential pkg-config libssl-dev libsqlite3-dev"
    exit 1
fi

# Load cargo env if available
source "$HOME/.cargo/env" || true

# Ensure stable toolchain
rustup default stable || true

# Clean previous builds to save space
echo "🧹 Cleaning previous builds..."
cargo clean || true

# Build with low-memory settings: opt-level=2, codegen-units=1, single job, no LTO
echo "🔨 Building release binary with low-memory profile..."
echo "   Settings: opt-level=2, codegen-units=1, -j1, LTO disabled"
export RUSTFLAGS="-C opt-level=2 -C codegen-units=1"
cargo build --release -j1 --locked

# Check if build succeeded
if [ ! -f "target/release/hora-police" ]; then
    echo "❌ Build failed - binary not created"
    echo "Check build output above for errors"
    exit 1
fi

echo "✅ Build successful!"
ls -lh target/release/hora-police

# Note: Binary installation should be done via scripts/install-binary.sh
echo ""
echo "📦 Binary ready at: target/release/hora-police"
echo "   Install with: ./scripts/install-binary.sh"

