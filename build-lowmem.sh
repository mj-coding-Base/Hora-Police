#!/usr/bin/env bash
# Low-memory build script for systems with limited RAM
set -euo pipefail

echo "🛡️  Building Hora-Police with low-memory profile..."

# Load cargo env if available
source "$HOME/.cargo/env" || true

# Ensure stable toolchain
rustup default stable || true

# Clean previous builds
echo "🧹 Cleaning previous builds..."
cargo clean

# Try debug build first (uses least memory)
echo "🔨 Attempting debug build (lowest memory usage)..."
if cargo build -j1 2>&1 | tee /tmp/build.log; then
    echo "✅ Debug build successful!"
    echo "📦 Installing debug binary..."
    sudo cp target/debug/hora-police /usr/local/bin/hora-police
    sudo chmod +x /usr/local/bin/hora-police
    echo "✅ Debug binary installed at /usr/local/bin/hora-police"
    echo "⚠️  Note: Debug binary is larger and slower than release"
    echo "   You can rebuild release later when system is stable"
    exit 0
fi

# If debug build fails, try lowmem profile
echo "⚠️  Debug build failed, trying lowmem profile..."
if cargo build --profile lowmem -j1 2>&1 | tee -a /tmp/build.log; then
    echo "✅ Lowmem build successful!"
    echo "📦 Installing lowmem binary..."
    sudo cp target/lowmem/hora-police /usr/local/bin/hora-police
    sudo chmod +x /usr/local/bin/hora-police
    echo "✅ Lowmem binary installed"
    exit 0
fi

# If both fail, try minimal release
echo "⚠️  Lowmem build failed, trying minimal release build..."
RUSTFLAGS="-C opt-level=1" cargo build --release -j1 || {
    echo "❌ All build attempts failed"
    echo "📋 Build log saved to /tmp/build.log"
    echo ""
    echo "Alternatives:"
    echo "1. Build on a machine with more RAM"
    echo "2. Check memory limits: ./scripts/check-memory-limits.sh"
    echo "3. Add more swap space"
    exit 1
}

echo "✅ Minimal release build successful!"
sudo cp target/release/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
echo "✅ Binary installed"

