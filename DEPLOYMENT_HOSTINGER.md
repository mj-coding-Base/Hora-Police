# Hora-Police Deployment Guide for Hostinger KVM4

This guide provides exact commands for deploying Hora-Police on a Hostinger KVM4 VPS.

## Quick Start

```bash
# 1. Navigate to deployment directory
cd /srv

# 2. Clone repository (or upload files)
git clone <repository-url> Hora-Police
cd Hora-Police

# 3. Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup default stable

# 4. Build application
cargo build --release

# 5. Install binary
sudo cp target/release/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police

# 6. Create directories
sudo mkdir -p /etc/hora-police
sudo mkdir -p /var/lib/hora-police/{quarantine,rollbacks}
sudo mkdir -p /etc/hora-police/keys

# 7. Set permissions
sudo chown -R root:root /etc/hora-police /var/lib/hora-police
sudo chmod 755 /var/lib/hora-police
sudo chmod 700 /var/lib/hora-police/quarantine
sudo chmod 700 /etc/hora-police/keys

# 8. Copy configuration
sudo cp config.examples/hostinger_kvm4.toml /etc/hora-police/config.toml

# 9. Edit configuration (add your Telegram token if using)
sudo nano /etc/hora-police/config.toml

# 10. Install systemd service
sudo cp hora-police.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable hora-police

# 11. Start service
sudo systemctl start hora-police

# 12. Verify status
sudo systemctl status hora-police
```

## Detailed Steps

### Step 1: Prerequisites Check

```bash
# Check Ubuntu version
lsb_release -a

# Check available disk space (need at least 500MB)
df -h

# Check available RAM (need at least 128MB free)
free -h

# Check if Rust is installed
rustc --version || echo "Rust not installed, will install in next step"
```

### Step 2: Install Rust (if needed)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add to PATH
source $HOME/.cargo/env

# Set default toolchain
rustup default stable

# Verify installation
rustc --version
cargo --version
```

### Step 3: Build Application

```bash
# Navigate to project directory
cd /srv/Hora-Police

# Build in release mode
cargo build --release

# Verify binary was created
ls -lh target/release/hora-police
```

**Expected output**: Binary should be ~5-10MB in size.

### Step 4: Install Binary

```bash
# Copy to system binary directory
sudo cp target/release/hora-police /usr/local/bin/hora-police

# Make executable
sudo chmod +x /usr/local/bin/hora-police

# Verify installation
which hora-police
hora-police --help
```

### Step 5: Create Directory Structure

```bash
# Create configuration directory
sudo mkdir -p /etc/hora-police

# Create data directories
sudo mkdir -p /var/lib/hora-police/quarantine
sudo mkdir -p /var/lib/hora-police/rollbacks

# Create keys directory
sudo mkdir -p /etc/hora-police/keys

# Set ownership
sudo chown -R root:root /etc/hora-police /var/lib/hora-police

# Set permissions
sudo chmod 755 /var/lib/hora-police
sudo chmod 700 /var/lib/hora-police/quarantine
sudo chmod 700 /etc/hora-police/keys
```

### Step 6: Configure Application

```bash
# Copy example configuration
sudo cp config.examples/hostinger_kvm4.toml /etc/hora-police/config.toml

# Edit configuration
sudo nano /etc/hora-police/config.toml
```

**Required changes**:
- Set `telegram.bot_token` (if using Telegram)
- Set `telegram.chat_id` (e.g., "@mjpavithra")
- Review `whitelist.manual_patterns` for your apps
- Verify `file_scanning.scan_paths`

**Recommended initial settings** (Audit Mode):
```toml
dry_run = true
audit_only = true
auto_kill = false
aggressive_cleanup = false
```

### Step 7: Install systemd Service

```bash
# Copy service file
sudo cp hora-police.service /etc/systemd/system/

# Reload systemd
sudo systemctl daemon-reload

# Enable service (start on boot)
sudo systemctl enable hora-police

# Verify service file
sudo systemctl cat hora-police
```

### Step 8: Start and Verify

```bash
# Start service
sudo systemctl start hora-police

# Check status
sudo systemctl status hora-police

# View logs
sudo journalctl -u hora-police -f
```

**Expected log output**:
```
🚀 Hora-Police Anti-Malware Daemon starting...
✅ System environment detected: 4 vCPU, 8192MB RAM
✅ Database initialized at: /var/lib/hora-police/intelligence.db
✅ Whitelist initialized with X entries
🛡️  Hora-Police daemon initialized. Starting monitoring...
```

### Step 9: Verify Operation

```bash
# Check if process is running
ps aux | grep hora-police

# Check resource usage
top -p $(pgrep hora-police)

# Check database
sqlite3 /var/lib/hora-police/intelligence.db "SELECT COUNT(*) FROM process_history;"

# Check telemetry endpoint (if probe enabled)
curl http://127.0.0.1:9999
```

## Post-Deployment Verification

### 1. Service Health

```bash
# Service should be active and running
sudo systemctl is-active hora-police
# Expected: active

# Service should be enabled
sudo systemctl is-enabled hora-police
# Expected: enabled
```

### 2. Resource Usage

```bash
# CPU should be < 1%
ps aux | grep hora-police | awk '{print $3}'

# Memory should be < 60MB
ps aux | grep hora-police | awk '{print $6}'
```

### 3. Detection Capabilities

```bash
# Check if PM2 apps are detected
sudo journalctl -u hora-police | grep "PM2 apps"

# Check if systemd units are detected
sudo journalctl -u hora-police | grep "systemd units"

# Check if Nginx upstreams are detected
sudo journalctl -u hora-police | grep "Nginx upstreams"
```

### 4. Database Operation

```bash
# Verify database exists and is accessible
sqlite3 /var/lib/hora-police/intelligence.db "PRAGMA integrity_check;"
# Expected: ok

# Check if data is being recorded
sqlite3 /var/lib/hora-police/intelligence.db \
  "SELECT COUNT(*) FROM process_history WHERE timestamp > datetime('now', '-1 hour');"
```

## Troubleshooting Installation

### Build Fails

```bash
# Update Rust
rustup update stable

# Clean and rebuild
cargo clean
cargo build --release
```

### Service Fails to Start

```bash
# Check configuration syntax
hora-police --config /etc/hora-police/config.toml --dry-run

# Check logs
sudo journalctl -u hora-police -n 50

# Common issues:
# - Config file not found
# - Database directory permissions
# - Missing dependencies
```

### Permission Errors

```bash
# Fix ownership
sudo chown -R root:root /etc/hora-police /var/lib/hora-police

# Fix permissions
sudo chmod 755 /var/lib/hora-police
sudo chmod 700 /var/lib/hora-police/quarantine
```

## Updating Hora-Police

```bash
# 1. Stop service
sudo systemctl stop hora-police

# 2. Backup configuration
sudo cp /etc/hora-police/config.toml /etc/hora-police/config.toml.backup

# 3. Pull latest changes
cd /srv/Hora-Police
git pull

# 4. Rebuild
cargo build --release

# 5. Install new binary
sudo cp target/release/hora-police /usr/local/bin/hora-police

# 6. Restart service
sudo systemctl start hora-police
```

## Uninstallation

```bash
# 1. Stop and disable service
sudo systemctl stop hora-police
sudo systemctl disable hora-police

# 2. Remove service file
sudo rm /etc/systemd/system/hora-police.service
sudo systemctl daemon-reload

# 3. Remove binary
sudo rm /usr/local/bin/hora-police

# 4. Remove configuration (optional - keep for reference)
# sudo rm -rf /etc/hora-police

# 5. Remove data (optional - backup first!)
# sudo rm -rf /var/lib/hora-police
```

## Next Steps

After successful installation:

1. **Review HOSTINGER_KVM4_RUNBOOK.md** for operational procedures
2. **Monitor in Audit Mode** for 3 days
3. **Transition to Canary Mode** after verification
4. **Enable Full Enforcement** after canary period

## Support

For deployment issues:
1. Check logs: `sudo journalctl -u hora-police`
2. Verify configuration: `sudo cat /etc/hora-police/config.toml`
3. Check system resources: `free -h && df -h`
4. Review build output: `cargo build --release 2>&1 | tail -20`

