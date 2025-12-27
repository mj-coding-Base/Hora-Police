# Quick Start: Fix 203/EXEC Error

## Problem
Service fails with `status=203/EXEC` - systemd cannot execute the binary (usually because binary is missing).

## Solution: Build on VPS (Single Command)

**On your VPS, run**:

```bash
cd /srv/Hora-Police
git pull
chmod +x build-and-fix-on-vps.sh
./build-and-fix-on-vps.sh
```

This single script will:
- ✅ Check/install Rust if needed
- ✅ Create all required directories
- ✅ Build the binary on VPS (10-20 minutes)
- ✅ Install binary with correct permissions
- ✅ Install tmpfiles.d configuration
- ✅ Fix systemd unit file
- ✅ Start and verify service

## Alternative: Step-by-Step

### Step 1: Diagnose (Optional)

```bash
cd /srv/Hora-Police
git pull
chmod +x diagnose-binary.sh
./diagnose-binary.sh
```

This will tell you exactly what's wrong with the binary.

### Step 2: Build and Fix

```bash
cd /srv/Hora-Police
chmod +x build-and-fix-on-vps.sh
./build-and-fix-on-vps.sh
```

## Verification

```bash
# Check service status
sudo systemctl status hora-police --no-pager

# Check for errors
sudo journalctl -u hora-police -n 50 --no-pager | grep -iE 'EXEC|error' || echo "No errors"

# Should show: Active: active (running)
```

## Files Created

1. **`diagnose-binary.sh`** - Comprehensive binary diagnostics
2. **`copy-binary-from-wsl.sh`** - Automated binary transfer from WSL
3. **`fix-service-directories.sh`** - Enhanced with better 203/EXEC handling
4. **`REMEDIATION_BINARY_MISSING.md`** - Updated with VPS IP and exact commands

## Manual Alternative

If the automated script doesn't work, manual steps:

**On VPS**:
```bash
cd /srv/Hora-Police

# Install Rust if needed
source $HOME/.cargo/env || curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Build
RUSTFLAGS="-C opt-level=3" cargo build --release -j1

# Install
sudo cp target/release/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police

# Fix service
./fix-service-directories.sh
```

