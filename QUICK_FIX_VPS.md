# Quick Fix: Build and Run on VPS

## Problem
Binary is missing at `/usr/local/bin/hora-police`, causing 203/EXEC errors.

## Solution: Build on VPS

Run this single command on your VPS:

```bash
cd /srv/Hora-Police
git pull
chmod +x build-and-fix-on-vps.sh
./build-and-fix-on-vps.sh
```

This script will:
1. ✅ Check/install Rust if needed
2. ✅ Create all required directories
3. ✅ Build the binary (takes 10-20 minutes)
4. ✅ Install binary with correct permissions
5. ✅ Install tmpfiles.d configuration
6. ✅ Fix systemd service file
7. ✅ Start and verify service

## What to Expect

- **Build time**: 10-20 minutes (single job to save memory)
- **Memory usage**: Minimal (uses -j1 to avoid OOM)
- **Final result**: Service running and active

## Verification

After script completes:

```bash
# Check service status
sudo systemctl status hora-police --no-pager

# Should show: Active: active (running)

# Check for errors
sudo journalctl -u hora-police -n 50 --no-pager | grep -iE 'EXEC|error' || echo "No errors"

# View logs
sudo journalctl -u hora-police -f
```

## If Build Fails with OOM

If you get "Killed" during build:

1. **Add swap** (if not already present):
   ```bash
   free -h  # Check current swap
   sudo fallocate -l 2G /swapfile
   sudo chmod 600 /swapfile
   sudo mkswap /swapfile
   sudo swapon /swapfile
   ```

2. **Try again**:
   ```bash
   ./build-and-fix-on-vps.sh
   ```

## Manual Steps (if script fails)

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

# 2. Build
cd /srv/Hora-Police
RUSTFLAGS="-C opt-level=3" cargo build --release -j1

# 3. Install
sudo cp target/release/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police

# 4. Fix service
./fix-service-directories.sh
```

