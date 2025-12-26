# 🚀 Deployment Guide: Sentinel on Ubuntu KVM

Complete step-by-step guide for deploying Sentinel Anti-Malware Daemon on Ubuntu KVM.

## 📋 Prerequisites

### System Requirements
- **OS**: Ubuntu 20.04 LTS or Ubuntu 22.04 LTS (recommended)
- **Architecture**: x86_64 (amd64)
- **RAM**: Minimum 512MB free (daemon uses ~40MB)
- **Disk**: 100MB free for binary and database
- **Access**: Root or sudo privileges
- **Network**: Internet access for building and Telegram (optional)

### Verify Your System

```bash
# Check Ubuntu version
lsb_release -a

# Check architecture
uname -m  # Should show: x86_64

# Check available memory
free -h

# Check disk space
df -h
```

## 🔧 Step 1: System Preparation

### 1.1 Update System Packages

```bash
sudo apt-get update
sudo apt-get upgrade -y
```

### 1.2 Install Build Dependencies

```bash
sudo apt-get install -y \
    build-essential \
    libsqlite3-dev \
    pkg-config \
    curl \
    git \
    ca-certificates
```

### 1.3 Install Rust Toolchain

```bash
# Download and run Rust installer
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add Rust to PATH for current session
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

**Expected output:**
```
rustc 1.70.0 or higher
cargo 1.70.0 or higher
```

### 1.4 (Optional) Add Rust to Permanent PATH

If you want Rust available after logout:

```bash
echo 'source $HOME/.cargo/env' >> ~/.bashrc
source ~/.bashrc
```

## 📦 Step 2: Get Sentinel Source Code

### Option A: If You Have the Source Files

```bash
# Navigate to your project directory
cd /path/to/sentinel-daemon
```

### Option B: If You Need to Transfer Files

If you're transferring from another machine:

```bash
# On your local machine, create a tarball
tar -czf sentinel-daemon.tar.gz sentinel-daemon/

# Transfer to KVM (using SCP, SFTP, or your preferred method)
scp sentinel-daemon.tar.gz user@your-kvm-ip:/tmp/

# On KVM, extract
cd /tmp
tar -xzf sentinel-daemon.tar.gz
cd sentinel-daemon
```

## 🔨 Step 3: Build Sentinel

### 3.1 Navigate to Project Directory

```bash
cd /path/to/sentinel-daemon
```

### 3.2 Verify Project Structure

```bash
ls -la
# Should see: Cargo.toml, src/, build.sh, etc.
```

### 3.3 Build the Binary

```bash
# Make build script executable
chmod +x build.sh

# Run build script
./build.sh
```

**OR build manually:**

```bash
# Build in release mode (optimized)
cargo build --release
```

**Expected output:**
```
   Compiling sentinel-daemon v0.1.0
   ...
   Finished release [optimized] target(s) in Xm Ys
```

**Build time**: Typically 5-15 minutes on KVM (depends on CPU)

### 3.4 Verify Binary

```bash
# Check binary exists
ls -lh target/release/sentinel-daemon

# Test binary (will show help/error if config missing - that's OK)
sudo ./target/release/sentinel-daemon --help 2>&1 || echo "Binary exists"
```

## 📁 Step 4: Install Sentinel

### 4.1 Create System Directories

```bash
# Create config directory
sudo mkdir -p /etc/sentinel

# Create data directory for database
sudo mkdir -p /var/lib/sentinel

# Set permissions
sudo chown root:root /etc/sentinel
sudo chown root:root /var/lib/sentinel
sudo chmod 755 /etc/sentinel
sudo chmod 755 /var/lib/sentinel
```

### 4.2 Install Binary

```bash
# Copy binary to system path
sudo cp target/release/sentinel-daemon /usr/local/bin/sentinel-daemon

# Set permissions
sudo chmod +x /usr/local/bin/sentinel-daemon

# Verify installation
which sentinel-daemon
sentinel-daemon --version 2>&1 || echo "Binary installed"
```

### 4.3 Install Configuration

```bash
# Copy example config
sudo cp config.toml.example /etc/sentinel/config.toml

# Set permissions (read-only for security)
sudo chmod 644 /etc/sentinel/config.toml
sudo chown root:root /etc/sentinel/config.toml
```

### 4.4 Edit Configuration (Optional)

```bash
# Edit config file
sudo nano /etc/sentinel/config.toml
```

**Default configuration works, but you may want to adjust:**

```toml
# For testing, you might lower thresholds:
cpu_threshold = 15.0          # Lower for more sensitive detection
duration_minutes = 3           # Shorter duration for testing

# Enable real-time alerts (if Telegram configured):
real_time_alerts = false       # Set to true for immediate notifications

# Database path (default is fine):
database_path = "/var/lib/sentinel/intelligence.db"
```

**Save and exit**: `Ctrl+X`, then `Y`, then `Enter`

## 🤖 Step 5: Install systemd Service

### 5.1 Copy Service File

```bash
# Copy service file
sudo cp sentinel.service /etc/systemd/system/sentinel.service

# Set permissions
sudo chmod 644 /etc/systemd/system/sentinel.service
```

### 5.2 Review Service File

```bash
# View service file
cat /etc/systemd/system/sentinel.service
```

**Verify the paths are correct:**
- `ExecStart` should point to `/usr/local/bin/sentinel-daemon`
- Config path should be `/etc/sentinel/config.toml`

### 5.3 Reload systemd

```bash
# Reload systemd to recognize new service
sudo systemctl daemon-reload
```

### 5.4 Enable Service (Start on Boot)

```bash
# Enable service to start on boot
sudo systemctl enable sentinel
```

**Expected output:**
```
Created symlink /etc/systemd/system/multi-user.target.wants/sentinel.service
```

## 🚀 Step 6: Start and Verify

### 6.1 Start Sentinel

```bash
# Start the service
sudo systemctl start sentinel
```

### 6.2 Check Status

```bash
# Check service status
sudo systemctl status sentinel
```

**Expected output:**
```
● sentinel.service - Sentinel Anti-Malware Daemon
     Loaded: loaded (/etc/systemd/system/sentinel.service; enabled)
     Active: active (running) since ...
```

### 6.3 View Logs

```bash
# View recent logs
sudo journalctl -u sentinel -n 50

# Follow logs in real-time
sudo journalctl -u sentinel -f
```

**Look for:**
```
🚀 Sentinel Anti-Malware Daemon starting...
✅ Configuration loaded from: /etc/sentinel/config.toml
🛡️  Sentinel daemon initialized. Starting monitoring...
```

### 6.4 Verify Process

```bash
# Check if daemon is running
ps aux | grep sentinel-daemon | grep -v grep

# Check resource usage
top -p $(pgrep sentinel-daemon)
```

**Expected:**
- Process running as root
- Low CPU usage (<1%)
- Memory usage ~30-40MB

## 📱 Step 7: Configure Telegram (Optional)

### 7.1 Create Telegram Bot

1. Open Telegram app or web
2. Search for **@BotFather**
3. Send `/newbot`
4. Follow prompts:
   - Bot name: `My Sentinel Bot`
   - Username: `my_sentinel_bot` (must end in `bot`)
5. **Copy the bot token** (looks like: `123456789:ABCdef...`)

### 7.2 Get Your Chat ID

**Option A: Use Username**
- If your Telegram username is `mjpavithra`, use `@mjpavithra`

**Option B: Get Numeric ID**
1. Search for **@userinfobot**
2. Start conversation
3. Copy your numeric ID

### 7.3 Update Configuration

```bash
# Edit config
sudo nano /etc/sentinel/config.toml
```

**Add/update Telegram section:**

```toml
[telegram]
bot_token = "YOUR_BOT_TOKEN_HERE"
chat_id = "@mjpavithra"  # or your numeric ID
daily_report_time = "09:00"  # 24-hour format
```

**Save and exit**

### 7.4 Restart Sentinel

```bash
# Restart to apply Telegram config
sudo systemctl restart sentinel

# Verify Telegram connection in logs
sudo journalctl -u sentinel -f
```

## ✅ Step 8: Verification and Testing

### 8.1 Test CPU Abuse Detection

```bash
# Create a CPU-hogging process (will be killed within 5 minutes)
while true; do :; done &

# Note the PID
echo $!

# Watch logs
sudo journalctl -u sentinel -f
```

**Expected behavior:**
- Process detected after 5 minutes of high CPU
- Process killed automatically
- Entry logged in database

### 8.2 Check Database

```bash
# Install sqlite3 if not present
sudo apt-get install -y sqlite3

# Query database
sudo sqlite3 /var/lib/sentinel/intelligence.db

# In sqlite prompt:
.tables
SELECT * FROM kill_actions ORDER BY timestamp DESC LIMIT 5;
.quit
```

### 8.3 Test Cron Monitoring

```bash
# Create a test cron entry (harmless)
echo "* * * * * echo 'test'" | sudo crontab -

# Check logs for cron detection
sudo journalctl -u sentinel -f

# Remove test cron
sudo crontab -r
```

### 8.4 Verify Daily Report (if Telegram configured)

Wait until the configured time (default 09:00) or manually trigger:

```bash
# Check if report task is scheduled
sudo journalctl -u sentinel | grep -i "daily report"
```

## 🔍 Step 9: Monitoring and Maintenance

### 9.1 Check Service Health

```bash
# Status check
sudo systemctl status sentinel

# Resource usage
ps aux | grep sentinel-daemon

# Disk usage (database)
du -h /var/lib/sentinel/intelligence.db
```

### 9.2 View Logs

```bash
# Recent logs
sudo journalctl -u sentinel -n 100

# Logs since boot
sudo journalctl -u sentinel -b

# Logs from today
sudo journalctl -u sentinel --since today

# Follow logs
sudo journalctl -u sentinel -f
```

### 9.3 Database Maintenance

```bash
# Check database size
du -h /var/lib/sentinel/intelligence.db

# Query recent activity
sudo sqlite3 /var/lib/sentinel/intelligence.db \
  "SELECT COUNT(*) FROM process_history WHERE timestamp > datetime('now', '-1 day');"

# (Optional) Clean old records (older than 30 days)
sudo sqlite3 /var/lib/sentinel/intelligence.db \
  "DELETE FROM process_history WHERE timestamp < datetime('now', '-30 days');"
```

### 9.4 Update Sentinel

When updating to a new version:

```bash
# Stop service
sudo systemctl stop sentinel

# Backup database
sudo cp /var/lib/sentinel/intelligence.db /var/lib/sentinel/intelligence.db.backup

# Rebuild binary
cd /path/to/sentinel-daemon
git pull  # or update source files
cargo build --release

# Install new binary
sudo cp target/release/sentinel-daemon /usr/local/bin/sentinel-daemon

# Start service
sudo systemctl start sentinel

# Verify
sudo systemctl status sentinel
```

## 🛠️ Troubleshooting

### Issue: Service Won't Start

```bash
# Check service status
sudo systemctl status sentinel

# Check logs for errors
sudo journalctl -u sentinel -n 50

# Test binary manually
sudo /usr/local/bin/sentinel-daemon /etc/sentinel/config.toml
```

**Common fixes:**
- Verify config file exists: `ls -la /etc/sentinel/config.toml`
- Check config syntax: `sudo sentinel-daemon /etc/sentinel/config.toml`
- Verify database directory permissions: `ls -ld /var/lib/sentinel`

### Issue: Build Fails

```bash
# Check Rust version
rustc --version

# Update Rust
rustup update

# Clean build
cargo clean
cargo build --release
```

### Issue: High CPU Usage

```bash
# Check actual usage
top -p $(pgrep sentinel-daemon)

# Increase polling interval in config
sudo nano /etc/sentinel/config.toml
# Set: polling_interval_ms = 10000  # 10 seconds instead of 5
sudo systemctl restart sentinel
```

### Issue: Database Errors

```bash
# Check database file
ls -la /var/lib/sentinel/intelligence.db

# Check permissions
sudo chown root:root /var/lib/sentinel/intelligence.db
sudo chmod 644 /var/lib/sentinel/intelligence.db

# Verify database integrity
sudo sqlite3 /var/lib/sentinel/intelligence.db "PRAGMA integrity_check;"
```

### Issue: Telegram Not Working

```bash
# Test bot token manually
curl "https://api.telegram.org/bot<YOUR_TOKEN>/getMe"

# Check network connectivity
ping api.telegram.org

# Verify config
sudo cat /etc/sentinel/config.toml | grep -A 3 telegram

# Check logs for Telegram errors
sudo journalctl -u sentinel | grep -i telegram
```

### Issue: No Detections

```bash
# Verify processes are using CPU
top

# Lower threshold for testing
sudo nano /etc/sentinel/config.toml
# Set: cpu_threshold = 10.0
# Set: duration_minutes = 1
sudo systemctl restart sentinel

# Create test process
while true; do :; done &
```

## 📊 Performance Monitoring

### Check Resource Usage

```bash
# CPU and Memory
ps aux | grep sentinel-daemon

# Detailed monitoring
sudo systemd-cgtop | grep sentinel

# Over time
watch -n 5 'ps aux | grep sentinel-daemon | grep -v grep'
```

### Expected Performance

- **CPU**: <1% average
- **Memory**: 30-40MB
- **Disk I/O**: Minimal (database writes)
- **Network**: Only Telegram API calls (if enabled)

## 🔒 Security Considerations

### File Permissions

```bash
# Verify permissions
ls -la /usr/local/bin/sentinel-daemon
ls -la /etc/sentinel/config.toml
ls -la /var/lib/sentinel/

# Should be:
# Binary: root:root, 755
# Config: root:root, 644
# Database: root:root, 644
```

### Firewall (if applicable)

```bash
# Sentinel only needs outbound HTTPS for Telegram
# No inbound ports required

# If using UFW, ensure outbound is allowed
sudo ufw status
```

## 📝 Quick Reference Commands

```bash
# Start/Stop/Restart
sudo systemctl start sentinel
sudo systemctl stop sentinel
sudo systemctl restart sentinel

# Status and Logs
sudo systemctl status sentinel
sudo journalctl -u sentinel -f

# Configuration
sudo nano /etc/sentinel/config.toml
sudo systemctl restart sentinel

# Database
sudo sqlite3 /var/lib/sentinel/intelligence.db

# Uninstall (if needed)
sudo systemctl stop sentinel
sudo systemctl disable sentinel
sudo rm /etc/systemd/system/sentinel.service
sudo rm /usr/local/bin/sentinel-daemon
sudo rm -rf /etc/sentinel
sudo rm -rf /var/lib/sentinel
```

## ✅ Deployment Checklist

- [ ] System updated and dependencies installed
- [ ] Rust toolchain installed
- [ ] Sentinel built successfully
- [ ] Binary installed to `/usr/local/bin/`
- [ ] Configuration file created at `/etc/sentinel/config.toml`
- [ ] Database directory created at `/var/lib/sentinel/`
- [ ] systemd service installed and enabled
- [ ] Service started and running
- [ ] Logs show successful startup
- [ ] Resource usage verified (<1% CPU, ~40MB RAM)
- [ ] Telegram configured (optional)
- [ ] Test detection verified
- [ ] Database accessible and logging

## 🎉 Success Indicators

You'll know Sentinel is working when:

1. ✅ Service status shows `active (running)`
2. ✅ Logs show "Starting monitoring..."
3. ✅ Process visible in `ps aux`
4. ✅ Low resource usage
5. ✅ Database file exists and grows over time
6. ✅ Test processes are detected and killed
7. ✅ Daily Telegram reports arrive (if configured)

---

**Congratulations!** Sentinel is now protecting your Ubuntu KVM. 🛡️

For additional help, see:
- `README.md` - Full documentation
- `QUICKSTART.md` - Quick setup guide
- `TELEGRAM_SETUP.md` - Telegram configuration
- `BUILD_NOTES.md` - Build troubleshooting

