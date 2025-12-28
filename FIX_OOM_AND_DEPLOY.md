# Fix OOM Issues and Deploy

## Problem
Your VPS is running out of memory. Even `git pull`, `rustup`, and `cargo clean` are being killed.

## Solution: Add Swap First, Then Deploy

### Step 1: Add Swap Space (CRITICAL)

```bash
# Check current swap
free -h

# Add 4GB swap file
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile

# Make permanent
echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab

# Verify swap is active
free -h
# Should show Swap: 4.0Gi
```

### Step 2: Create Install Script Manually (if git pull fails)

If `git pull` keeps failing, create the install script manually:

```bash
cat > /tmp/install-binary.sh << 'EOF'
#!/usr/bin/env bash
set -euo pipefail

BINARY_SOURCE="/tmp/hora-police"
BINARY_DEST="/usr/local/bin/hora-police"
REPO_DIR="/srv/Hora-Police"

echo "=== Hora-Police Binary Installation ==="

if [ ! -f "${BINARY_SOURCE}" ]; then
    echo "❌ Binary not found at ${BINARY_SOURCE}"
    exit 1
fi

echo "✅ Binary found: ${BINARY_SOURCE}"

# Stop service
sudo systemctl stop hora-police 2>/dev/null || true

# Install binary
sudo cp "${BINARY_SOURCE}" "${BINARY_DEST}"
sudo chown root:root "${BINARY_DEST}"
sudo chmod 755 "${BINARY_DEST}"

# Verify binary
if sudo "${BINARY_DEST}" --help >/dev/null 2>&1; then
    echo "✅ Binary executes successfully"
else
    echo "⚠️  Binary test failed (may need config file)"
fi

# Ensure directories
sudo mkdir -p /etc/hora-police
sudo mkdir -p /var/lib/hora-police/quarantine
sudo mkdir -p /var/lib/hora-police/rollbacks
sudo mkdir -p /var/log/hora-police
sudo chown -R root:root /etc/hora-police /var/lib/hora-police /var/log/hora-police
sudo chmod 0755 /etc/hora-police /var/lib/hora-police /var/log/hora-police
sudo chmod 0700 /var/lib/hora-police/quarantine

# Install tmpfiles.d if exists
if [ -f "${REPO_DIR}/etc/tmpfiles.d/hora-police.conf" ]; then
    sudo cp "${REPO_DIR}/etc/tmpfiles.d/hora-police.conf" /etc/tmpfiles.d/hora-police.conf
    sudo systemd-tmpfiles --create /etc/tmpfiles.d/hora-police.conf || true
fi

# Reload systemd
sudo systemctl daemon-reload
sudo systemctl reset-failed hora-police || true

# Start service
sudo systemctl start hora-police
sleep 3

# Check status
sudo systemctl status hora-police --no-pager -l | head -30 || true

if sudo systemctl is-active --quiet hora-police; then
    echo "✅ SUCCESS! Service is running."
else
    echo "❌ Service failed to start"
    sudo journalctl -u hora-police -n 50
    exit 1
fi
EOF

chmod +x /tmp/install-binary.sh
```

### Step 3: Try Git Pull Again (with swap)

```bash
cd /srv/Hora-Police
git pull
```

If it still fails, you can manually apply the code fixes (see below).

### Step 4: Build with Swap

```bash
cd /srv/Hora-Police

# Make build script executable
chmod +x build-lowmem.sh

# Build (should work now with swap)
./build-lowmem.sh
```

### Step 5: Install Binary

```bash
# Copy binary to /tmp
cp target/release/hora-police /tmp/hora-police

# Install using script
/tmp/install-binary.sh
```

## Alternative: Build Elsewhere and Transfer

If building on VPS still fails, build on your local machine and transfer:

### On Your Local Machine (Windows/Linux):

```bash
# Build
cd /path/to/Hora-Police
cargo build --release

# Transfer to VPS
scp target/release/hora-police deploy@mail-server:/tmp/hora-police
```

### On VPS:

```bash
# Install
/tmp/install-binary.sh
```

## Manual Code Fixes (if git pull fails)

If you can't pull the fixes, apply them manually:

### Fix 1: `src/file_quarantine.rs` line 94

```bash
cd /srv/Hora-Police
sed -i 's/let monitor = ProcessMonitor::new();/let mut monitor = ProcessMonitor::new();/' src/file_quarantine.rs
```

### Fix 2: `src/process_monitor.rs`

Add helper function and fix Uid calls:

```bash
cd /srv/Hora-Police

# Add import
sed -i 's/use sysinfo::{Pid, System, Process, User};/use sysinfo::{Pid, System, Process, User, Uid};/' src/process_monitor.rs

# Add helper function after imports (around line 6)
cat >> /tmp/uid_helper.txt << 'HELPER'
fn uid_to_u32(uid_opt: Option<&Uid>) -> u32 {
    uid_opt.map(|u| u.as_()).unwrap_or(0u32)
}
HELPER

# Insert after line 5 (after use statements)
sed -i '5r /tmp/uid_helper.txt' src/process_monitor.rs

# Replace as_raw() calls
sed -i 's/process.user_id().map(|u| u.as_raw()).unwrap_or(0u32)/uid_to_u32(process.user_id())/g' src/process_monitor.rs
```

### Fix 3: `src/kill_engine.rs` - Complex, better to pull

This fix is complex. If git pull fails, try building with the first two fixes only, or transfer a pre-built binary.

## Quick One-Liner Fix

```bash
# Add swap, create install script, and prepare for build
sudo fallocate -l 4G /swapfile && sudo chmod 600 /swapfile && sudo mkswap /swapfile && sudo swapon /swapfile && echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab && free -h
```

