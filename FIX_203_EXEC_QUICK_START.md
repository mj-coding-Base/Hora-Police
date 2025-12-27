# Quick Start: Fix 203/EXEC Error

## Problem
Service fails with `status=203/EXEC` - systemd cannot execute the binary.

## Solution (3 Steps)

### Step 1: Diagnose (On VPS)

```bash
cd /srv/Hora-Police
git pull
chmod +x diagnose-binary.sh
./diagnose-binary.sh
```

This will tell you exactly what's wrong with the binary.

### Step 2: Copy Binary (From WSL on Windows)

```bash
cd /mnt/f/Personal_Projects/Hora-Police
chmod +x copy-binary-from-wsl.sh
./copy-binary-from-wsl.sh
```

This script will:
- Build the binary if needed
- Copy it to VPS (62.72.13.136)
- Install with correct permissions
- Verify installation

### Step 3: Fix Service (On VPS)

```bash
cd /srv/Hora-Police
chmod +x fix-service-directories.sh
./fix-service-directories.sh
```

This will:
- Create all required directories
- Install tmpfiles.d config
- Fix systemd unit file
- Start and verify service

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

If scripts don't work, manual steps:

**From WSL**:
```bash
cd /mnt/f/Personal_Projects/Hora-Police
cargo build --release
scp target/release/hora-police deploy@62.72.13.136:/tmp/
```

**On VPS**:
```bash
sudo mv /tmp/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
cd /srv/Hora-Police
./fix-service-directories.sh
```

