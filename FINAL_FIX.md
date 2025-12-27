# Final Fix for Build OOM and Service Issues

## Immediate Solution

Your build is being killed due to OOM. Here's the fastest way to get Hora-Police running:

### Step 1: Build Debug Version (Uses Less Memory)

```bash
cd /srv/Hora-Police
git pull

# Build debug version (uses ~50% less memory)
cargo build -j1

# This should complete successfully
# Install debug binary
sudo cp target/debug/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
```

### Step 2: Fix Service File (Remove PrivateTmp)

```bash
# Stop service
sudo systemctl stop hora-police

# Update service file (remove PrivateTmp)
sudo sed -i 's/PrivateTmp=true/# PrivateTmp removed - was causing mount namespace issues/' /etc/systemd/system/hora-police.service

# Or manually edit:
sudo nano /etc/systemd/system/hora-police.service
# Comment out or remove the line: PrivateTmp=true

# Reload
sudo systemctl daemon-reload
```

### Step 3: Start Service

```bash
# Start service
sudo systemctl start hora-police

# Check status
sudo systemctl status hora-police

# View logs
sudo journalctl -u hora-police -f
```

## Alternative: Use Automated Scripts

```bash
cd /srv/Hora-Police
git pull

# Make scripts executable
chmod +x build-lowmem.sh scripts/check-memory-limits.sh

# Check memory situation
./scripts/check-memory-limits.sh

# Try lowmem build (tries debug first, then lowmem, then minimal release)
./build-lowmem.sh
```

## If Debug Build Also Fails

### Option A: Build on Your Local Machine

On your Windows machine (if you have WSL or can use Docker):

```bash
# In WSL or Docker
cd /path/to/Hora-Police
cargo build --release

# Transfer to server
scp target/release/hora-police deploy@mail-server:/tmp/hora-police

# On server
sudo cp /tmp/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
```

### Option B: Check What's Killing the Build

```bash
# Check OOM killer logs
dmesg | grep -i "out of memory" | tail -10

# Check what process is using memory
ps aux --sort=-%mem | head -20

# Check user limits
ulimit -a

# Check if there's a memory limit
cat /sys/fs/cgroup/memory/memory.limit_in_bytes 2>/dev/null || echo "No cgroup limit"
```

## Service File Fix (Complete)

The service file should look like this:

```ini
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
# PrivateTmp removed - was causing mount namespace issues
ProtectSystem=full
ProtectHome=true
ReadOnlyPaths=/proc /sys
ReadWritePaths=/var/lib/hora-police /etc/hora-police /var/log/hora-police
```

**Key changes:**
- `Type=simple` (not `notify`)
- `PrivateTmp` removed (causes namespace issues)
- `ProtectSystem=full` (not `strict`)
- `ReadOnlyPaths=/proc /sys` added

## Complete Fix Command Sequence

```bash
# 1. Pull latest changes
cd /srv/Hora-Police
git pull

# 2. Build debug version
cargo build -j1

# 3. Install binary
sudo cp target/debug/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police

# 4. Fix service file
sudo systemctl stop hora-police
sudo sed -i '/^PrivateTmp=true$/d' /etc/systemd/system/hora-police.service
sudo systemctl daemon-reload

# 5. Start service
sudo systemctl start hora-police
sudo systemctl status hora-police
```

## Verify Everything Works

```bash
# Check service
sudo systemctl status hora-police

# Check logs
sudo journalctl -u hora-police -n 30

# Test binary manually
sudo /usr/local/bin/hora-police --help

# Check if process is running
ps aux | grep hora-police
```

## Next Steps After Getting It Running

1. **Fix zombie processes** - Address the 2631 zombies
2. **Rebuild release later** - Once system is stable, rebuild release version
3. **Monitor** - Watch logs and ensure no false positives

## If Nothing Works

As a last resort, you can:
1. Build on a different machine (cloud instance, local machine, etc.)
2. Transfer the binary
3. Install and run

See `BUILD_ALTERNATIVES.md` for detailed instructions.

