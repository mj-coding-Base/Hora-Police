#!/usr/bin/env bash
# Complete VPS deployment script for Hora-Police
set -euo pipefail

echo "🛡️  Hora-Police VPS Deployment Script"
echo "======================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if running as root for certain operations
if [ "$EUID" -ne 0 ]; then 
    SUDO="sudo"
else
    SUDO=""
fi

# Step 1: Check prerequisites
echo ""
echo "[1/8] Checking prerequisites..."
if ! command -v rustc &> /dev/null; then
    echo -e "${YELLOW}⚠️  Rust not found. Installing...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Cargo not found after Rust installation${NC}"
    exit 1
fi

# Check for required system packages
echo "Checking system packages..."
if ! dpkg -l | grep -q libsqlite3-dev; then
    echo -e "${YELLOW}Installing libsqlite3-dev...${NC}"
    $SUDO apt-get update
    $SUDO apt-get install -y libsqlite3-dev pkg-config build-essential
fi

echo -e "${GREEN}✅ Prerequisites OK${NC}"

# Step 2: Navigate to project directory
echo ""
echo "[2/8] Setting up project directory..."
PROJECT_DIR="/srv/Hora-Police"
if [ ! -d "$PROJECT_DIR" ]; then
    echo -e "${YELLOW}⚠️  Project directory not found. Please clone the repository first.${NC}"
    echo "   git clone <repo-url> $PROJECT_DIR"
    exit 1
fi

cd "$PROJECT_DIR"

# Pull latest changes
echo "Pulling latest changes..."
git pull || echo -e "${YELLOW}⚠️  Git pull failed (may not be a git repo)${NC}"

echo -e "${GREEN}✅ Project directory ready${NC}"

# Step 3: Load Rust environment
echo ""
echo "[3/8] Loading Rust environment..."
source "$HOME/.cargo/env" || true
rustup default stable || true

# Verify Rust is available
if ! cargo --version &> /dev/null; then
    echo -e "${RED}❌ Cargo not available${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Rust environment loaded${NC}"

# Step 4: Build the application
echo ""
echo "[4/8] Building Hora-Police..."
echo -e "${YELLOW}This may take 10-20 minutes with -j1 (low memory build)${NC}"

# Try to detect available memory and adjust build jobs
AVAIL_MEM=$(free -m | awk 'NR==2{printf "%.0f", $7}')
if [ "$AVAIL_MEM" -lt 2048 ]; then
    BUILD_JOBS=1
    echo -e "${YELLOW}Low memory detected (${AVAIL_MEM}MB available). Using single job build.${NC}"
else
    BUILD_JOBS=2
    echo "Using 2 parallel jobs for build"
fi

# Clean previous builds
cargo clean || true

# Build with appropriate job count
if cargo build --release -j$BUILD_JOBS; then
    echo -e "${GREEN}✅ Build successful${NC}"
else
    echo -e "${RED}❌ Build failed${NC}"
    echo "Trying debug build (uses less memory)..."
    if cargo build -j1; then
        echo -e "${YELLOW}⚠️  Debug build successful. Using debug binary.${NC}"
        BINARY_PATH="target/debug/hora-police"
    else
        echo -e "${RED}❌ Build failed completely${NC}"
        exit 1
    fi
fi

# Set binary path
if [ -z "${BINARY_PATH:-}" ]; then
    BINARY_PATH="target/release/hora-police"
fi

if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}❌ Binary not found at $BINARY_PATH${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Binary built: $BINARY_PATH${NC}"

# Step 5: Install binary
echo ""
echo "[5/8] Installing binary..."
$SUDO cp "$BINARY_PATH" /usr/local/bin/hora-police
$SUDO chmod +x /usr/local/bin/hora-police

# Verify installation
if /usr/local/bin/hora-police --help &> /dev/null; then
    echo -e "${GREEN}✅ Binary installed and verified${NC}"
else
    echo -e "${YELLOW}⚠️  Binary installed but test failed (may be expected)${NC}"
fi

# Step 6: Create directories and config
echo ""
echo "[6/8] Setting up directories and configuration..."
$SUDO mkdir -p /etc/hora-police
$SUDO mkdir -p /var/lib/hora-police
$SUDO mkdir -p /var/lib/hora-police/quarantine
$SUDO mkdir -p /var/log/hora-police

# Copy config if it doesn't exist
if [ ! -f /etc/hora-police/config.toml ]; then
    if [ -f "$PROJECT_DIR/config.toml.example" ]; then
        $SUDO cp "$PROJECT_DIR/config.toml.example" /etc/hora-police/config.toml
        echo -e "${GREEN}✅ Config file created from example${NC}"
    else
        echo -e "${YELLOW}⚠️  No config.toml.example found. Creating minimal config...${NC}"
        $SUDO tee /etc/hora-police/config.toml > /dev/null << 'EOF'
cpu_threshold = 20.0
duration_minutes = 5
real_time_alerts = false
auto_kill = true
learning_mode = true
database_path = "/var/lib/hora-police/intelligence.db"
polling_interval_ms = 5000
threat_confidence_threshold = 0.7

[file_scanning]
enabled = true
scan_interval_minutes = 15
scan_paths = ["/home", "/tmp", "/var/tmp"]
quarantine_path = "/var/lib/hora-police/quarantine"
auto_delete = false
kill_processes_using_file = true
aggressive_cleanup = true
EOF
    fi
else
    echo -e "${GREEN}✅ Config file already exists${NC}"
fi

# Set permissions
$SUDO chown -R root:root /etc/hora-police /var/lib/hora-police
$SUDO chmod 644 /etc/hora-police/config.toml
$SUDO chmod 755 /var/lib/hora-police
$SUDO chmod 700 /var/lib/hora-police/quarantine

echo -e "${GREEN}✅ Directories and config ready${NC}"

# Step 7: Install and configure systemd service
echo ""
echo "[7/8] Installing systemd service..."

# Copy service file
if [ -f "$PROJECT_DIR/hora-police.service" ]; then
    $SUDO cp "$PROJECT_DIR/hora-police.service" /etc/systemd/system/
    echo -e "${GREEN}✅ Service file copied${NC}"
else
    echo -e "${YELLOW}⚠️  Service file not found. Creating minimal service file...${NC}"
    $SUDO tee /etc/systemd/system/hora-police.service > /dev/null << 'EOF'
[Unit]
Description=Hora-Police Anti-Malware Daemon
After=network.target

[Service]
Type=simple
User=root
ExecStart=/usr/local/bin/hora-police /etc/hora-police/config.toml
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal
CPUQuota=15%
MemoryMax=128M
TasksMax=1024
NoNewPrivileges=true
ProtectSystem=full
ProtectHome=true
ReadOnlyPaths=/proc /sys
ReadWritePaths=/var/lib/hora-police /etc/hora-police /var/log/hora-police

[Install]
WantedBy=multi-user.target
EOF
fi

# Reload systemd
$SUDO systemctl daemon-reload

# Enable service
$SUDO systemctl enable hora-police

echo -e "${GREEN}✅ Service installed and enabled${NC}"

# Step 8: Start service and verify
echo ""
echo "[8/8] Starting service..."
$SUDO systemctl stop hora-police 2>/dev/null || true
$SUDO systemctl start hora-police

sleep 2

# Check status
if $SUDO systemctl is-active --quiet hora-police; then
    echo -e "${GREEN}✅ Service is running!${NC}"
    echo ""
    echo "=== Service Status ==="
    $SUDO systemctl status hora-police --no-pager -l | head -15
    echo ""
    echo -e "${GREEN}🎉 Deployment complete!${NC}"
    echo ""
    echo "Useful commands:"
    echo "  - View logs: sudo journalctl -u hora-police -f"
    echo "  - Check status: sudo systemctl status hora-police"
    echo "  - Restart: sudo systemctl restart hora-police"
    echo "  - Stop: sudo systemctl stop hora-police"
else
    echo -e "${RED}❌ Service failed to start${NC}"
    echo ""
    echo "Troubleshooting:"
    echo "  1. Check logs: sudo journalctl -u hora-police -n 50"
    echo "  2. Test binary: sudo /usr/local/bin/hora-police /etc/hora-police/config.toml"
    echo "  3. Check service file: sudo cat /etc/systemd/system/hora-police.service"
    exit 1
fi

