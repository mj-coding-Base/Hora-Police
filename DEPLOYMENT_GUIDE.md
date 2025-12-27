# Hora-Police Deployment Guide

## Quick Fix: Build and Deploy on VPS

### Step 1: Fix Compile Errors (Already Applied)

The following fixes have been applied:
- ✅ Fixed `monitor` mutability in `file_quarantine.rs`
- ✅ Fixed `Uid.as_raw()` → `Uid.as_()` in `process_monitor.rs` with helper function
- ✅ Fixed recursive async in `kill_engine.rs` using `tokio::spawn`

### Step 2: Build on VPS

```bash
cd /srv/Hora-Police
git pull
chmod +x build-lowmem.sh
./build-lowmem.sh
```

This will:
- Build with `opt-level=2`, `codegen-units=1`, `-j1` (single job)
- Disable LTO to reduce memory usage
- Produce binary at `target/release/hora-police`

**Expected time**: 10-20 minutes

### Step 3: Install Binary

```bash
# Copy binary to /tmp
cp target/release/hora-police /tmp/hora-police

# Run install script
chmod +x scripts/install-binary.sh
./scripts/install-binary.sh
```

The install script will:
- Stop service
- Copy binary to `/usr/local/bin/hora-police` with correct permissions
- Ensure all required directories exist
- Install tmpfiles.d configuration
- Reload systemd
- Start service and verify

### Step 4: Verify Deployment

```bash
# Check service status
sudo systemctl status hora-police --no-pager

# Should show: Active: active (running)

# Check for errors
sudo journalctl -u hora-police -n 50 --no-pager | grep -iE 'EXEC|error' || echo "No errors"

# Verify binary
file /usr/local/bin/hora-police
ldd /usr/local/bin/hora-police

# Verify directories
ls -la /etc/hora-police /var/lib/hora-police /var/log/hora-police
```

## Verification Checklist

- [ ] Binary exists: `test -f /usr/local/bin/hora-police && echo "OK"`
- [ ] Binary is executable: `test -x /usr/local/bin/hora-police && echo "OK"`
- [ ] Binary architecture matches: `file /usr/local/bin/hora-police | grep -q "x86-64" && echo "OK"`
- [ ] Service is active: `systemctl is-active --quiet hora-police && echo "OK"`
- [ ] No EXEC errors: `journalctl -u hora-police -n 50 | grep -qi "EXEC" || echo "OK"`
- [ ] Directories exist: `test -d /etc/hora-police && test -d /var/lib/hora-police && test -d /var/log/hora-police && echo "OK"`
- [ ] tmpfiles.d installed: `test -f /etc/tmpfiles.d/hora-police.conf && echo "OK"`

## Troubleshooting

### Build Fails with OOM

Add swap space:
```bash
sudo fallocate -l 2G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

Then retry build.

### Service Fails with 203/EXEC

1. Verify binary exists: `ls -l /usr/local/bin/hora-police`
2. Test binary: `sudo /usr/local/bin/hora-police --help`
3. Check permissions: `stat /usr/local/bin/hora-police`
4. Check architecture: `file /usr/local/bin/hora-police`

### Service Fails with 226/NAMESPACE

1. Verify directories exist: `ls -la /var/log/hora-police`
2. Check tmpfiles.d: `sudo systemd-tmpfiles --create /etc/tmpfiles.d/hora-police.conf`
3. Check service file: `sudo systemctl cat hora-police`

## Code Changes Summary

### 1. `src/file_quarantine.rs`
- Changed `let monitor = ProcessMonitor::new();` to `let mut monitor = ProcessMonitor::new();`
- Fixes: `error[E0596]: cannot borrow monitor as mutable`

### 2. `src/process_monitor.rs`
- Added `uid_to_u32()` helper function using `Uid.as_()` (sysinfo 0.30+ API)
- Replaced `u.as_raw()` with `uid_to_u32(process.user_id())` in two places
- Fixes: `error[E0599]: no method named as_raw found`

### 3. `src/kill_engine.rs`
- Changed recursive `self.kill_process().await` to `tokio::spawn` with cloned data
- Prevents infinite future size in recursive async function
- Fixes: `error[E0733]: recursion in an async fn requires boxing`

### 4. `build-lowmem.sh`
- Updated to use `RUSTFLAGS="-C opt-level=2 -C codegen-units=1"`
- Uses `-j1` for single job build
- Uses `--locked` for deterministic builds

### 5. `scripts/install-binary.sh`
- New script for safe binary deployment
- Stops service, installs binary, ensures directories, reloads systemd, starts service
