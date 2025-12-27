# Complete Solution for Build OOM and Service Issues

## Problem Summary
1. Build killed (OOM) - Even debug build fails
2. Service 226/NAMESPACE - Service file not properly updated
3. 2631 zombie processes - System resource issue

## Solution: Build Elsewhere + Fix Service

Since local build keeps failing, we'll build on a different machine and transfer the binary.

### Option A: Build on Your Local Machine (Windows with WSL)

If you have WSL or can use Docker on Windows:

```bash
# In WSL or Docker container
cd /mnt/f/Personal_Projects/Hora-Police  # Adjust path to your project

# Build release
cargo build --release

# Transfer to server
scp target/release/hora-police deploy@mail-server:/tmp/hora-police
```

### Option B: Build on Cloud Instance (Recommended)

Use a temporary cloud instance with more RAM:

```bash
# On cloud instance (e.g., AWS EC2, DigitalOcean, etc.)
git clone <your-repo-url> Hora-Police
cd Hora-Police
cargo build --release

# Transfer to your server
scp target/release/hora-police deploy@mail-server:/tmp/hora-police
```

### Option C: Use GitHub Actions / CI

If you have CI/CD, build there and download the artifact.

## On Your Server: Complete Fix

Once you have the binary on the server:

```bash
# 1. Stop service
sudo systemctl stop hora-police

# 2. Install binary
sudo cp /tmp/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police

# 3. Verify binary works
sudo /usr/local/bin/hora-police --help

# 4. Fix service file completely
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

# Security hardening (minimal to avoid namespace issues)
NoNewPrivileges=true
ProtectSystem=full
ProtectHome=true
ReadOnlyPaths=/proc /sys
ReadWritePaths=/var/lib/hora-police /etc/hora-police /var/log/hora-police

[Install]
WantedBy=multi-user.target
EOF

# 5. Reload and start
sudo systemctl daemon-reload
sudo systemctl start hora-police
sudo systemctl status hora-police
```

## Alternative: Minimal Service File (If Still Failing)

If the service still fails, use this minimal version:

```bash
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

# Resource limits only
CPUQuota=15%
MemoryMax=128M
TasksMax=1024

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl start hora-police
```

## Verify Service File Was Updated

```bash
# Check service file
sudo cat /etc/systemd/system/hora-police.service | grep -E "Type=|PrivateTmp|ProtectSystem"

# Should show:
# Type=simple
# ProtectSystem=full
# (No PrivateTmp line)
```

## If You Must Build Locally

Try these extreme measures:

```bash
# 1. Kill unnecessary processes to free memory
# Check what's using memory
ps aux --sort=-%mem | head -20

# 2. Increase ulimit
ulimit -v unlimited
ulimit -m unlimited

# 3. Build with absolute minimum
cd /srv/Hora-Police
RUSTFLAGS="-C opt-level=0" cargo build -j1 --target-dir /tmp/hora-build

# 4. If that works, install
sudo cp /tmp/hora-build/debug/hora-police /usr/local/bin/hora-police
```

## Quick Fix Script

Save this as `complete-fix.sh` and run it:

```bash
#!/bin/bash
set -e

echo "=== Complete Fix for Hora-Police ==="

# Check if binary exists
if [ ! -f "/tmp/hora-police" ]; then
    echo "❌ Binary not found at /tmp/hora-police"
    echo "Please build on another machine and transfer it first:"
    echo "  scp target/release/hora-police deploy@mail-server:/tmp/hora-police"
    exit 1
fi

# Stop service
echo "[1/5] Stopping service..."
sudo systemctl stop hora-police 2>/dev/null || true

# Install binary
echo "[2/5] Installing binary..."
sudo cp /tmp/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police

# Verify binary
echo "[3/5] Verifying binary..."
if ! /usr/local/bin/hora-police --help >/dev/null 2>&1; then
    echo "⚠️  Binary test failed, but continuing..."
fi

# Fix service file
echo "[4/5] Fixing service file..."
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

# Reload and start
echo "[5/5] Starting service..."
sudo systemctl daemon-reload
sudo systemctl start hora-police
sleep 2

# Check status
echo ""
echo "=== Service Status ==="
sudo systemctl status hora-police --no-pager -l | head -20

if systemctl is-active --quiet hora-police; then
    echo ""
    echo "✅ Service is running!"
    echo "View logs: sudo journalctl -u hora-police -f"
else
    echo ""
    echo "❌ Service failed to start"
    echo "Check logs: sudo journalctl -u hora-police -n 50"
fi
```

