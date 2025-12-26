#!/bin/bash
# Build script for Sentinel Anti-Malware Daemon

set -e

echo "🛡️  Building Sentinel Anti-Malware Daemon..."

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust/Cargo not found. Please install Rust:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Check for required system libraries
echo "📦 Checking dependencies..."
if ! pkg-config --exists sqlite3 2>/dev/null; then
    echo "⚠️  SQLite3 development libraries not found."
    echo "   Install with: sudo apt-get install libsqlite3-dev"
    exit 1
fi

# Build in release mode
echo "🔨 Building release binary..."
cargo build --release

if [ $? -eq 0 ]; then
    echo "✅ Build successful!"
    echo "📦 Binary location: target/release/sentinel-daemon"
    echo ""
    echo "To install:"
    echo "  sudo cp target/release/sentinel-daemon /usr/local/bin/"
    echo "  sudo chmod +x /usr/local/bin/sentinel-daemon"
else
    echo "❌ Build failed. Check errors above."
    exit 1
fi

