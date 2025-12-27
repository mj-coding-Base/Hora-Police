# Quick Fix for Current Issues

## Immediate Actions Required

### 1. Fix Service File (Status 226/NAMESPACE)

Run these commands:

```bash
# Stop the service
sudo systemctl stop hora-police

# Edit service file
sudo nano /etc/systemd/system/hora-police.service
```

**Replace the security section with:**
```ini
# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=full
ProtectHome=true
# Allow read access to /proc and /sys for process monitoring
ReadOnlyPaths=/proc /sys
ReadWritePaths=/var/lib/hora-police /etc/hora-police /var/log/hora-police
BindReadOnlyPaths=/proc /sys
```

**Save and exit (Ctrl+X, Y, Enter)**

```bash
# Reload systemd
sudo systemctl daemon-reload

# Try starting again
sudo systemctl start hora-police
sudo systemctl status hora-police
```

### 2. Fix Build Memory Issues

```bash
cd /srv/Hora-Police

# Build with limited parallelism (2 jobs instead of all CPUs)
cargo build --release -j2

# If still fails, try without LTO
RUSTFLAGS="-C opt-level=3" cargo build --release -j2

# Install
sudo cp target/release/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
```

### 3. Address Zombie Processes

```bash
# Find which processes are creating zombies
ps -eo ppid,stat | awk '$2 ~ /Z/ {print $1}' | sort | uniq -c | sort -rn | head -5

# Check what those parent PIDs are
ps -fp <PARENT_PID>

# If it's PM2, restart it
pm2 restart all

# If it's a systemd service, restart it
sudo systemctl restart <service-name>

# If unsure and safe to reboot, schedule reboot
sudo reboot
```

### 4. Complete Fix Script

Run this complete fix:

```bash
#!/bin/bash
set -e

echo "=== Fixing Hora-Police Service ==="

# 1. Stop service
echo "Stopping service..."
sudo systemctl stop hora-police || true

# 2. Fix service file
echo "Updating service file..."
sudo sed -i 's/ProtectSystem=strict/ProtectSystem=full/' /etc/systemd/system/hora-police.service
sudo sed -i '/ReadWritePaths=/a ReadOnlyPaths=/proc /sys\nBindReadOnlyPaths=/proc /sys' /etc/systemd/system/hora-police.service

# 3. Reload systemd
echo "Reloading systemd..."
sudo systemctl daemon-reload

# 4. Build with limited jobs
echo "Building application..."
cd /srv/Hora-Police
cargo build --release -j2 || RUSTFLAGS="-C opt-level=3" cargo build --release -j2

# 5. Install binary
echo "Installing binary..."
sudo cp target/release/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police

# 6. Start service
echo "Starting service..."
sudo systemctl start hora-police

# 7. Check status
echo "Service status:"
sudo systemctl status hora-police --no-pager -l

echo "=== Done ==="
echo "View logs with: sudo journalctl -u hora-police -f"
```

Save as `fix.sh`, make executable, and run:
```bash
chmod +x fix.sh
./fix.sh
```

