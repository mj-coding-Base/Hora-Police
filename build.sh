#!/usr/bin/env bash
set -euo pipefail

echo "🛡️  Building Hora-Police Anti-Malware Daemon..."

# Load cargo env if available
source "$HOME/.cargo/env" || true

# Ensure stable toolchain
rustup default stable || true

# Fetch dependencies
echo "📦 Fetching dependencies..."
cargo fetch

# Build with optimizations
echo "🔨 Building optimized release binary..."
RUSTFLAGS="-C lto -C codegen-units=1 -C opt-level=z" cargo build --release -j$(nproc)

# Strip binary
echo "✂️  Stripping binary..."
strip target/release/hora-police || true

# Install binary
echo "📦 Installing binary..."
sudo cp target/release/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police

echo "✅ Build and installation successful!"
echo "📦 Binary location: /usr/local/bin/hora-police"
