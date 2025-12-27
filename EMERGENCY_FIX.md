# Emergency Fix for Build OOM and Service Issues

## Problem Summary
1. Build killed (SIGKILL) - Out of Memory
2. Binary doesn't exist - Build failed
3. Service still 226/NAMESPACE - Service file not updated or Type=notify issue

## Complete Fix Procedure

### Step 1: Fix Service File (Critical - Do This First)

```bash
# Stop service
sudo systemctl stop hora-police

# Backup current service file
sudo cp /etc/systemd/system/hora-police.service /etc/systemd/system/hora-police.service.backup

# Edit service file
sudo nano /etc/systemd/system/hora-police.service
```

**Replace the entire [Service] section with this:**

```ini
[Service]
Type=simple
User=root
ExecStart=/usr/local/bin/hora-police /etc/hora-police/config.toml
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

# Resource limits (safe defaults for KVM4)
CPUQuota=15%
MemoryMax=128M
TasksMax=1024

# Security hardening (relaxed for process monitoring)
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=full
ProtectHome=true
ReadOnlyPaths=/proc /sys
ReadWritePaths=/var/lib/hora-police /etc/hora-police /var/log/hora-police
```

**Key changes:**
- Changed `Type=notify` to `Type=simple` (avoids namespace issues)
- Removed `WatchdogSec` and `NotifyAccess` (not needed with Type=simple)
- Changed `ProtectSystem=strict` to `ProtectSystem=full`
- Added `ReadOnlyPaths=/proc /sys`

**Save and exit (Ctrl+X, Y, Enter)**

```bash
# Reload systemd
sudo systemctl daemon-reload
```

### Step 2: Build with Minimal Memory Usage

```bash
cd /srv/Hora-Police

# Clean previous build artifacts
cargo clean

# Build WITHOUT LTO (uses much less memory)
# Use only 1 job to minimize memory pressure
RUSTFLAGS="-C opt-level=3" cargo build --release -j1

# If that still fails, try even simpler:
cargo build --release -j1 --config 'build.jobs=1'
```

**If build still fails, try this minimal approach:**

```bash
# Set memory limit for cargo
ulimit -v 2097152  # 2GB virtual memory limit

# Build with single job, no LTO
RUSTFLAGS="-C opt-level=2" cargo build --release -j1
```

### Step 3: Install Binary

```bash
# Check if binary was created
ls -lh target/release/hora-police

# If it exists, install it
sudo cp target/release/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police

# Verify
/usr/local/bin/hora-police --help || echo "Binary exists"
```

### Step 4: Test Binary Manually

```bash
# Test if binary runs (will fail without config, but should start)
sudo /usr/local/bin/hora-police /etc/hora-police/config.toml &
sleep 2
ps aux | grep hora-police
sudo pkill hora-police
```

### Step 5: Start Service

```bash
# Start service
sudo systemctl start hora-police

# Wait a moment
sleep 3

# Check status
sudo systemctl status hora-police

# View logs
sudo journalctl -u hora-police -n 30 --no-pager
```

## Alternative: Pre-built Binary Approach

If building continues to fail, you can:

1. **Build on a machine with more memory**
2. **Transfer the binary to the server:**

```bash
# On build machine
cargo build --release
scp target/release/hora-police deploy@mail-server:/tmp/

# On server
sudo cp /tmp/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
```

## Complete One-Liner Fix Script

Run this complete fix:

```bash
#!/bin/bash
set -e

echo "=== Emergency Fix for Hora-Police ==="

# 1. Stop service
echo "[1/6] Stopping service..."
sudo systemctl stop hora-police 2>/dev/null || true

# 2. Fix service file
echo "[2/6] Fixing service file..."
sudo tee /etc/systemd/system/hora-police.service > /dev/null << 'EOF'
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

# Resource limits
CPUQuota=15%
MemoryMax=128M
TasksMax=1024

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=full
ProtectHome=true
ReadOnlyPaths=/proc /sys
ReadWritePaths=/var/lib/hora-police /etc/hora-police /var/log/hora-police

[Install]
WantedBy=multi-user.target
EOF

# 3. Reload systemd
echo "[3/6] Reloading systemd..."
sudo systemctl daemon-reload

# 4. Build with minimal resources
echo "[4/6] Building (this may take a while with -j1)..."
cd /srv/Hora-Police
cargo clean
RUSTFLAGS="-C opt-level=3" cargo build --release -j1 || {
    echo "Build failed, trying even simpler..."
    cargo build --release -j1
}

# 5. Install
echo "[5/6] Installing binary..."
if [ -f "target/release/hora-police" ]; then
    sudo cp target/release/hora-police /usr/local/bin/hora-police
    sudo chmod +x /usr/local/bin/hora-police
    echo "  ✓ Binary installed"
else
    echo "  ✗ Build failed - binary not found"
    exit 1
fi

# 6. Start service
echo "[6/6] Starting service..."
sudo systemctl start hora-police
sleep 3

# Check status
echo ""
echo "=== Service Status ==="
sudo systemctl status hora-police --no-pager -l | head -20

echo ""
echo "=== Logs (last 10 lines) ==="
sudo journalctl -u hora-police -n 10 --no-pager

echo ""
echo "=== Done ==="
```

Save as `emergency-fix.sh`, make executable, and run:
```bash
chmod +x emergency-fix.sh
./emergency-fix.sh
```

## If Build Still Fails

### Option 1: Increase Swap

```bash
# Check current swap
free -h

# Create 4GB swap file
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile

# Make permanent
echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab

# Verify
free -h

# Try build again
cd /srv/Hora-Police
RUSTFLAGS="-C opt-level=3" cargo build --release -j1
```

### Option 2: Build on Different Machine

Build on a machine with more RAM, then transfer:

```bash
# On build machine (with more RAM)
cd /path/to/Hora-Police
cargo build --release

# Transfer to server
scp target/release/hora-police deploy@mail-server:/tmp/

# On server
sudo cp /tmp/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
```

### Option 3: Use Debug Build (Faster, Larger)

```bash
# Debug build uses less memory during compilation
cargo build -j1

# Install debug binary (larger but works)
sudo cp target/debug/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
```

## Verify Service File Was Updated

```bash
# Check current service file
sudo cat /etc/systemd/system/hora-police.service | grep -E "Type=|ProtectSystem="

# Should show:
# Type=simple
# ProtectSystem=full
```

If it still shows `Type=notify` or `ProtectSystem=strict`, the file wasn't updated properly.

