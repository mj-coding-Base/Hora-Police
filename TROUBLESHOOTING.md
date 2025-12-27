# Hora-Police Troubleshooting Guide

## Critical Issues and Solutions

### Issue 1: Service Fails with Status 226/NAMESPACE

**Error:**
```
hora-police.service: Main process exited, code=exited, status=226/NAMESPACE
```

**Cause:** Systemd sandboxing is too restrictive. The service needs access to `/proc` and `/sys` for process monitoring, but `ProtectSystem=strict` blocks this.

**Solution:**

1. **Update the service file** to use `ProtectSystem=full` instead of `strict` and add required paths:

```bash
sudo nano /etc/systemd/system/hora-police.service
```

Update the security section to:
```ini
ProtectSystem=full
ReadOnlyPaths=/proc /sys
BindReadOnlyPaths=/proc /sys
```

2. **Reload and restart:**
```bash
sudo systemctl daemon-reload
sudo systemctl restart hora-police
sudo systemctl status hora-police
```

**Alternative (if still failing):** Temporarily disable strict sandboxing for testing:
```ini
ProtectSystem=false
ProtectHome=false
```

### Issue 2: Build Process Killed (OOM - Out of Memory)

**Error:**
```
Killed
./build.sh: line 14: 1412765 Killed cargo fetch
```

**Cause:** System is running out of memory, likely due to:
- 2631 zombie processes consuming resources
- Insufficient swap space
- Too many parallel build jobs

**Solution:**

1. **Fix zombie processes first** (see Issue 3 below)

2. **Build with limited parallelism:**
```bash
# Instead of -j$(nproc), use fewer jobs
cargo build --release -j2

# Or build without LTO (faster, uses less memory)
RUSTFLAGS="-C opt-level=3" cargo build --release -j2
```

3. **Add swap space** (if needed):
```bash
# Check current swap
free -h

# Create swap file (2GB example)
sudo fallocate -l 2G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile

# Make permanent
echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab
```

4. **Build in stages:**
```bash
# Fetch dependencies first
cargo fetch

# Build with limited jobs
cargo build --release -j2

# Strip binary
strip target/release/hora-police
```

### Issue 3: 2631 Zombie Processes

**Critical Issue:** High zombie count indicates parent processes not properly waiting on children.

**Immediate Actions:**

1. **Identify zombie parent processes:**
```bash
# Find parent PIDs with most zombies
ps -eo ppid,stat | awk '$2 ~ /Z/ {print $1}' | sort | uniq -c | sort -rn | head -10

# Check what these parent processes are
ps -fp <PARENT_PID>
```

2. **Restart problematic services:**
```bash
# If it's a systemd service
sudo systemctl restart <service-name>

# If it's a user service
sudo -u <user> systemctl --user restart <service-name>

# If it's PM2
pm2 restart all
# Or for specific user
sudo -u deploy pm2 restart all
```

3. **Emergency: Reboot if safe:**
```bash
# Schedule reboot during maintenance window
sudo reboot
```

4. **Prevent future zombies:**
- Ensure all services properly handle child processes
- Use Hora-Police's zombie reaper (once running)
- Monitor with: `watch -n 5 'ps aux | awk "\$8==\"Z\" {print}" | wc -l'`

### Issue 4: Binary Not Found

**Error:**
```
/usr/local/bin/hora-police: No such file or directory
```

**Solution:**

1. **Check if binary exists:**
```bash
ls -la /usr/local/bin/hora-police
```

2. **If missing, build and install:**
```bash
cd /srv/Hora-Police
./build.sh
# Or manually:
cargo build --release -j2
sudo cp target/release/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
```

### Issue 5: Configuration File Not Found

**Error:**
```
Failed to read config from /etc/hora-police/config.toml
```

**Solution:**

```bash
# Create directories
sudo mkdir -p /etc/hora-police

# Copy example config
sudo cp /srv/Hora-Police/config.examples/hostinger_kvm4.toml /etc/hora-police/config.toml

# Set permissions
sudo chmod 644 /etc/hora-police/config.toml
sudo chown root:root /etc/hora-police/config.toml
```

### Issue 6: Permission Denied

**Error:**
```
Permission denied (os error 13)
```

**Solution:**

```bash
# Fix directory permissions
sudo chown -R root:root /var/lib/hora-police /etc/hora-police
sudo chmod 755 /var/lib/hora-police
sudo chmod 700 /var/lib/hora-police/quarantine

# Ensure binary is executable
sudo chmod +x /usr/local/bin/hora-police
```

### Issue 7: Database Errors

**Error:**
```
Failed to initialize database
```

**Solution:**

```bash
# Check database file permissions
ls -la /var/lib/hora-police/intelligence.db

# Fix permissions
sudo chown root:root /var/lib/hora-police/intelligence.db
sudo chmod 644 /var/lib/hora-police/intelligence.db

# If corrupted, backup and recreate
sudo mv /var/lib/hora-police/intelligence.db /var/lib/hora-police/intelligence.db.backup
sudo systemctl restart hora-police
```

## Step-by-Step Recovery Procedure

### For Your Current Situation:

1. **Stop the failing service:**
```bash
sudo systemctl stop hora-police
sudo systemctl disable hora-police
```

2. **Fix the service file:**
```bash
sudo nano /etc/systemd/system/hora-police.service
```
Change `ProtectSystem=strict` to `ProtectSystem=full` and add:
```ini
ReadOnlyPaths=/proc /sys
BindReadOnlyPaths=/proc /sys
```

3. **Address zombie processes:**
```bash
# Find top zombie parents
ps -eo ppid,stat | awk '$2 ~ /Z/ {print $1}' | sort | uniq -c | sort -rn | head -5

# Restart those services (be careful - check what they are first)
# Example if it's PM2:
pm2 restart all
```

4. **Build with limited resources:**
```bash
cd /srv/Hora-Police

# Build with 2 jobs only
cargo build --release -j2

# If still fails, try without LTO
RUSTFLAGS="-C opt-level=3" cargo build --release -j2

# Install
sudo cp target/release/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
```

5. **Test binary manually first:**
```bash
# Test without systemd
sudo /usr/local/bin/hora-police /etc/hora-police/config.toml

# If it works, press Ctrl+C and continue
```

6. **Reload and start service:**
```bash
sudo systemctl daemon-reload
sudo systemctl enable hora-police
sudo systemctl start hora-police
sudo systemctl status hora-police
```

7. **Check logs:**
```bash
sudo journalctl -u hora-police -f
```

## Quick Diagnostic Commands

```bash
# Check service status
sudo systemctl status hora-police

# View recent logs
sudo journalctl -u hora-police -n 50

# Test binary manually
sudo /usr/local/bin/hora-police --config /etc/hora-police/config.toml

# Check system resources
free -h
df -h
ps aux | head -20

# Check zombie count
ps aux | awk '$8=="Z" {print}' | wc -l

# Check zombie parents
ps -eo ppid,stat | awk '$2 ~ /Z/ {print $1}' | sort | uniq -c | sort -rn | head -10

# Check if binary exists
ls -la /usr/local/bin/hora-police

# Check config exists
ls -la /etc/hora-police/config.toml
```

## Prevention

1. **Monitor zombie processes regularly**
2. **Ensure proper swap space** (at least 1GB)
3. **Build with limited parallelism** on low-memory systems
4. **Use the updated service file** with proper paths
5. **Start in audit mode** (`dry_run = true`) to test safely

