# Hora-Police Hostinger KVM4 Runbook

## Overview

This runbook provides step-by-step procedures for deploying, operating, and troubleshooting Hora-Police on Hostinger KVM4 VPS environments hosting Next.js and Nest.js applications.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Installation](#installation)
3. [Initial Setup (Audit Mode)](#initial-setup-audit-mode)
4. [Canary Mode Transition](#canary-mode-transition)
5. [Full Enforcement](#full-enforcement)
6. [Monitoring](#monitoring)
7. [Rollback Procedures](#rollback-procedures)
8. [Troubleshooting](#troubleshooting)

## Prerequisites

### System Requirements
- Ubuntu 20.04+ or 22.04+
- Root or sudo access
- Rust toolchain (will be installed if missing)
- At least 128MB free RAM
- SQLite3 installed

### Application Requirements
- PM2 (if using PM2 for process management)
- systemd (standard on Ubuntu)
- Nginx (if using reverse proxy)

### Pre-Deployment Checklist
- [ ] Backup all application data
- [ ] Document current PM2/systemd services
- [ ] Note all application paths and ports
- [ ] Ensure Telegram bot token is ready (optional)

## Installation

### Step 1: Clone and Build

```bash
cd /srv
git clone <repository-url> Hora-Police
cd Hora-Police

# Install Rust if needed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup default stable

# Build the application
cargo build --release
```

### Step 2: Install Binary

```bash
sudo cp target/release/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
```

### Step 3: Setup Directories

```bash
sudo mkdir -p /etc/hora-police
sudo mkdir -p /var/lib/hora-police/{quarantine,rollbacks}
sudo mkdir -p /etc/hora-police/keys
sudo chown -R root:root /etc/hora-police /var/lib/hora-police
sudo chmod 755 /var/lib/hora-police
sudo chmod 700 /var/lib/hora-police/quarantine
sudo chmod 700 /etc/hora-police/keys
```

### Step 4: Create Configuration

```bash
sudo cp config.examples/hostinger_kvm4.toml /etc/hora-police/config.toml
sudo nano /etc/hora-police/config.toml
```

**Important**: Update the following:
- `telegram.bot_token` (if using Telegram)
- `telegram.chat_id` (e.g., "@mjpavithra")
- Review `whitelist.manual_patterns` for your specific apps
- Verify `file_scanning.scan_paths` match your deployment structure

### Step 5: Install systemd Service

```bash
sudo cp hora-police.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable hora-police
```

## Initial Setup (Audit Mode)

**Duration: Day 0-3**

### Configuration Settings

Ensure these settings in `/etc/hora-police/config.toml`:

```toml
dry_run = true
audit_only = true
canary_mode = false
auto_kill = false
aggressive_cleanup = false
real_time_alerts = false
```

### Start Service

```bash
sudo systemctl start hora-police
sudo systemctl status hora-police
```

### Verify Operation

```bash
# Check logs
sudo journalctl -u hora-police -f

# Check telemetry (if probe enabled)
curl http://127.0.0.1:9999

# Verify database
sqlite3 /var/lib/hora-police/intelligence.db "SELECT COUNT(*) FROM process_history;"
```

### What to Monitor

1. **False Positives**: Check logs for legitimate processes flagged
2. **Whitelist Coverage**: Verify all apps are auto-detected
3. **Resource Usage**: Ensure CPU < 1%, RAM < 60MB
4. **Detection Accuracy**: Review suspicious process detections

### Adjust Whitelist

If false positives occur:

```bash
sudo nano /etc/hora-police/config.toml
# Add patterns to whitelist.manual_patterns
sudo systemctl restart hora-police
```

## Canary Mode Transition

**Duration: Day 4-7**

### Configuration Changes

```bash
sudo nano /etc/hora-police/config.toml
```

Update:
```toml
dry_run = false
audit_only = false
canary_mode = true
auto_kill = true  # Enable but with high thresholds
aggressive_cleanup = false  # Still disabled
real_time_alerts = true  # Enable to monitor
```

### Restart Service

```bash
sudo systemctl restart hora-police
```

### Monitor Closely

- Watch Telegram alerts (if configured)
- Check logs every hour for first 24 hours
- Verify no legitimate processes are killed
- Monitor application uptime

### Rollback if Issues

If legitimate processes are killed:

```bash
sudo systemctl stop hora-police
sudo nano /etc/hora-police/config.toml
# Revert to audit_only = true
sudo systemctl start hora-police
```

## Full Enforcement

**Duration: Day 8+**

### Configuration Changes

```bash
sudo nano /etc/hora-police/config.toml
```

Update:
```toml
canary_mode = false
aggressive_cleanup = true  # Enable aggressive cleanup
```

### Restart Service

```bash
sudo systemctl restart hora-police
```

## Monitoring

### Daily Checks

```bash
# Service status
sudo systemctl status hora-police

# Recent detections
sqlite3 /var/lib/hora-police/intelligence.db \
  "SELECT pid, binary_path, reason, confidence, timestamp FROM kill_actions ORDER BY timestamp DESC LIMIT 10;"

# Resource usage
ps aux | grep hora-police
```

### Weekly Review

```bash
# Daily summary from database
sqlite3 /var/lib/hora-police/intelligence.db \
  "SELECT DATE(timestamp) as date, COUNT(*) as kills FROM kill_actions GROUP BY date ORDER BY date DESC LIMIT 7;"

# Malware files detected
sqlite3 /var/lib/hora-police/intelligence.db \
  "SELECT file_path, signature_name, detected_at FROM malware_files ORDER BY detected_at DESC LIMIT 20;"
```

### Telegram Reports

If configured, daily reports are sent automatically at the configured time (default: 09:00).

## Rollback Procedures

### Rollback from Kill Action

1. **Find Rollback Manifest**:
```bash
ls -lt /var/lib/hora-police/rollbacks/*.rollback.sh | head -1
```

2. **Review Manifest**:
```bash
cat /var/lib/hora-police/rollbacks/<latest>.rollback.json
```

3. **Execute Rollback**:
```bash
sudo bash /var/lib/hora-police/rollbacks/<latest>.rollback.sh
```

### Rollback from File Deletion

1. **List Quarantined Files**:
```bash
ls -la /var/lib/hora-police/quarantine/
```

2. **Restore File**:
```bash
sudo cp /var/lib/hora-police/quarantine/<file> <original-location>
```

3. **Restore from Rollback Manifest** (if available):
```bash
sudo bash /var/lib/hora-police/rollbacks/<malware-rollback>.rollback.sh
```

### Rollback from Cron Modification

1. **Find Backup**:
```bash
ls -lt /etc/cron.d/*.backup.* | head -1
```

2. **Restore**:
```bash
sudo cp <backup-file> <original-cron-file>
```

### Emergency Stop

```bash
sudo systemctl stop hora-police
```

## Troubleshooting

### Service Won't Start

```bash
# Check logs
sudo journalctl -u hora-police -n 50

# Common issues:
# - Config file syntax error
# - Database path permissions
# - Missing directories
```

### High CPU Usage

```bash
# Check if auto-tune is working
sudo journalctl -u hora-police | grep "System environment detected"

# Manually adjust polling_interval_ms in config
sudo nano /etc/hora-police/config.toml
```

### False Positives

1. **Add to Whitelist**:
```bash
sudo nano /etc/hora-police/config.toml
# Add pattern to whitelist.manual_patterns
```

2. **Restart Service**:
```bash
sudo systemctl restart hora-police
```

### Database Issues

```bash
# Check database integrity
sqlite3 /var/lib/hora-police/intelligence.db "PRAGMA integrity_check;"

# Vacuum database
sqlite3 /var/lib/hora-police/intelligence.db "VACUUM;"

# Archive old records (via service or manually)
sqlite3 /var/lib/hora-police/intelligence.db \
  "DELETE FROM process_history WHERE timestamp < datetime('now', '-30 days');"
```

### PM2/systemd Detection Issues

```bash
# Verify PM2 is running
pm2 ls

# Verify systemd units
systemctl list-units --type=service | grep -E "node|next|nest"

# Check logs for detection messages
sudo journalctl -u hora-police | grep -E "PM2|systemd|Nginx"
```

### File Watcher Not Working

```bash
# Check if inotify is available
ls -la /proc/sys/fs/inotify/

# Check logs
sudo journalctl -u hora-police | grep "inotify"

# Fallback to scheduled scans (already implemented)
```

## Performance Tuning

### Auto-Tune Override

If auto-detection fails:

```toml
[auto_tune]
enabled = true
vcpu_override = 4  # Set manually
ram_override_mb = 8192  # Set manually
```

### Manual Threshold Adjustment

```toml
cpu_threshold = 25.0  # Increase for less sensitivity
duration_minutes = 10  # Increase duration before flagging
threat_confidence_threshold = 0.8  # Increase for higher confidence required
```

## Security Considerations

1. **Rollback Key**: Stored in `/etc/hora-police/keys/rollback.key` - keep secure
2. **Quarantine Directory**: Only root should have access
3. **Database**: Contains sensitive process information
4. **Logs**: May contain command lines and paths - review before sharing

## Support

For issues or questions:
1. Check logs: `sudo journalctl -u hora-police`
2. Review configuration: `/etc/hora-police/config.toml`
3. Check database: `/var/lib/hora-police/intelligence.db`
4. Review rollback manifests: `/var/lib/hora-police/rollbacks/`

