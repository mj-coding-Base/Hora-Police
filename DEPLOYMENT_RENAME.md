# 🚀 Hora-Police Deployment Guide

The application has been renamed from "Sentinel" to **"Hora-Police"** with enhanced aggressive cleanup capabilities.

## 📦 What Changed

### Application Name
- **Old**: `sentinel-daemon`
- **New**: `hora-police`

### Binary Location
- **Old**: `/usr/local/bin/sentinel-daemon`
- **New**: `/usr/local/bin/hora-police`

### Configuration Path
- **Old**: `/etc/sentinel/config.toml`
- **New**: `/etc/hora-police/config.toml`

### Data Directory
- **Old**: `/var/lib/sentinel/`
- **New**: `/var/lib/hora-police/`

### Service Name
- **Old**: `sentinel.service`
- **New**: `hora-police.service`

## 🔧 Fresh Installation

If installing for the first time:

```bash
cd /srv/Hora-Police

# Set Rust toolchain
rustup default stable
source $HOME/.cargo/env

# Build
cargo build --release

# Install binary
sudo cp target/release/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police

# Setup directories
sudo mkdir -p /etc/hora-police /var/lib/hora-police /var/lib/hora-police/quarantine
sudo cp config.toml.example /etc/hora-police/config.toml
sudo chown -R root:root /etc/hora-police /var/lib/hora-police
sudo chmod 644 /etc/hora-police/config.toml
sudo chmod 755 /var/lib/hora-police
sudo chmod 700 /var/lib/hora-police/quarantine

# Install service
sudo cp hora-police.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable hora-police
sudo systemctl start hora-police

# Verify
sudo systemctl status hora-police
```

## 🔄 Migration from Sentinel

If you have an existing Sentinel installation:

```bash
# Stop old service
sudo systemctl stop sentinel
sudo systemctl disable sentinel

# Build new binary
cd /srv/Hora-Police
rustup default stable
source $HOME/.cargo/env
cargo build --release

# Install new binary
sudo cp target/release/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police

# Migrate configuration
sudo mkdir -p /etc/hora-police
sudo cp /etc/sentinel/config.toml /etc/hora-police/config.toml
# Edit config to update paths:
sudo nano /etc/hora-police/config.toml
# Change: database_path = "/var/lib/hora-police/intelligence.db"
# Change: quarantine_path = "/var/lib/hora-police/quarantine"

# Migrate database (optional - keeps history)
sudo mkdir -p /var/lib/hora-police
sudo cp /var/lib/sentinel/intelligence.db /var/lib/hora-police/intelligence.db
sudo chown -R root:root /var/lib/hora-police

# Install new service
sudo cp hora-police.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable hora-police
sudo systemctl start hora-police

# Remove old service (optional)
sudo rm /etc/systemd/system/sentinel.service
sudo systemctl daemon-reload

# Verify
sudo systemctl status hora-police
```

## ✨ New Features

### Aggressive Cleanup Mode

Hora-Police now includes aggressive cleanup that:
- Deletes parent directories containing only malware
- Removes related suspicious files
- Cleans cron jobs referencing malware
- Operates with full administrative authority

Enable in config:

```toml
[file_scanning]
aggressive_cleanup = true  # NEW: Delete malware origins
```

## 📋 Configuration Example

```toml
# Hora-Police Configuration
cpu_threshold = 20.0
duration_minutes = 5
real_time_alerts = false
auto_kill = true
learning_mode = true
database_path = "/var/lib/hora-police/intelligence.db"
polling_interval_ms = 5000
threat_confidence_threshold = 0.7

[telegram]
bot_token = "YOUR_BOT_TOKEN"
chat_id = "@mjpavithra"
daily_report_time = "09:00"

[file_scanning]
enabled = true
scan_interval_minutes = 15
scan_paths = ["/home", "/tmp", "/var/tmp"]
quarantine_path = "/var/lib/hora-police/quarantine"
auto_delete = false
kill_processes_using_file = true
aggressive_cleanup = true  # NEW!
```

## 🔍 Verification

```bash
# Check service
sudo systemctl status hora-police

# Check logs
sudo journalctl -u hora-police -f

# Check binary
which hora-police
/usr/local/bin/hora-police --version  # If version flag exists

# Check config
sudo cat /etc/hora-police/config.toml
```

## 📝 Log Messages

You should see:
```
🚀 Hora-Police Anti-Malware Daemon starting...
✅ Configuration loaded from: /etc/hora-police/config.toml
✅ Database initialized at: /var/lib/hora-police/intelligence.db
✅ File scanner initialized (scanning X paths)
🛡️ Hora-Police daemon initialized. Starting monitoring...
🚀 Hora-Police daemon running. Monitoring started.
```

## 🛠️ Troubleshooting

**Service not found?**
```bash
# Check if service file exists
ls -la /etc/systemd/system/hora-police.service

# Reload systemd
sudo systemctl daemon-reload
```

**Binary not found?**
```bash
# Check if binary exists
ls -la /usr/local/bin/hora-police

# Rebuild if needed
cargo build --release
sudo cp target/release/hora-police /usr/local/bin/
```

**Config errors?**
```bash
# Test config syntax
sudo /usr/local/bin/hora-police /etc/hora-police/config.toml
```

---

**Hora-Police is now ready to protect your system with aggressive cleanup!** 🛡️

