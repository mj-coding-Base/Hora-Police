# VPS Deployment Guide for Hora-Police

Complete guide for deploying Hora-Police on Ubuntu VPS.

## Quick Deployment

### Automated Deployment (Recommended)

```bash
cd /srv/Hora-Police
chmod +x deploy-vps.sh
./deploy-vps.sh
```

This script will:
1. Check prerequisites (Rust, system packages)
2. Pull latest code
3. Build the application
4. Install binary
5. Create directories and config
6. Install systemd service
7. Start and verify service

### Manual Deployment

If you prefer manual steps:

```bash
# 1. Install prerequisites
sudo apt-get update
sudo apt-get install -y build-essential libsqlite3-dev pkg-config

# 2. Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env
rustup default stable

# 3. Navigate to project
cd /srv/Hora-Police
git pull

# 4. Build (use -j1 for low memory systems)
source $HOME/.cargo/env
cargo build --release -j1

# 5. Install binary
sudo cp target/release/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police

# 6. Create directories
sudo mkdir -p /etc/hora-police /var/lib/hora-police/quarantine /var/log/hora-police

# 7. Create config
sudo cp config.toml.example /etc/hora-police/config.toml
# Edit config if needed: sudo nano /etc/hora-police/config.toml

# 8. Install service
sudo cp hora-police.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable hora-police
sudo systemctl start hora-police

# 9. Verify
sudo systemctl status hora-police
```

## Build Options

### Low Memory Systems (< 2GB available)

```bash
# Use single job build
cargo build --release -j1

# Or build debug version (uses less memory during compilation)
cargo build -j1
sudo cp target/debug/hora-police /usr/local/bin/hora-police
```

### Normal Systems (2GB+ available)

```bash
# Use 2-4 parallel jobs
cargo build --release -j2
```

## Troubleshooting

### Build Fails with OOM (Out of Memory)

1. **Reduce build jobs:**
   ```bash
   cargo build --release -j1
   ```

2. **Build debug version:**
   ```bash
   cargo build -j1
   sudo cp target/debug/hora-police /usr/local/bin/hora-police
   ```

3. **Add swap space:**
   ```bash
   sudo fallocate -l 4G /swapfile
   sudo chmod 600 /swapfile
   sudo mkswap /swapfile
   sudo swapon /swapfile
   echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab
   ```

4. **Build on different machine:**
   - Build on local machine or cloud instance with more RAM
   - Transfer binary via SCP: `scp target/release/hora-police deploy@server:/tmp/`
   - Install: `sudo cp /tmp/hora-police /usr/local/bin/hora-police`

### Service Fails to Start

1. **Check logs:**
   ```bash
   sudo journalctl -u hora-police -n 50
   ```

2. **Test binary manually:**
   ```bash
   sudo /usr/local/bin/hora-police /etc/hora-police/config.toml
   ```

3. **Check service file:**
   ```bash
   sudo cat /etc/systemd/system/hora-police.service
   ```

4. **Common issues:**
   - **NAMESPACE error**: Service file needs `ReadOnlyPaths=/proc /sys` and `ProtectSystem=full` (not `strict`)
   - **Binary not found**: Ensure binary is at `/usr/local/bin/hora-police` and is executable
   - **Config not found**: Ensure config exists at `/etc/hora-police/config.toml`

### Service File Issues

If you see `status=226/NAMESPACE`:

```bash
# Fix service file
sudo nano /etc/systemd/system/hora-police.service

# Ensure it has:
# Type=simple (not notify)
# ProtectSystem=full (not strict)
# ReadOnlyPaths=/proc /sys
# No PrivateTmp line

# Then reload:
sudo systemctl daemon-reload
sudo systemctl restart hora-police
```

## Post-Deployment Verification

### 1. Check Service Status

```bash
sudo systemctl status hora-police
```

Should show: `Active: active (running)`

### 2. Check Logs

```bash
sudo journalctl -u hora-police -f
```

Should show startup messages and no errors.

### 3. Verify Binary

```bash
/usr/local/bin/hora-police --help
```

Should show help text.

### 4. Check Database

```bash
ls -lh /var/lib/hora-police/intelligence.db
```

Database should be created after first run.

### 5. Test Configuration

```bash
sudo /usr/local/bin/hora-police /etc/hora-police/config.toml --dry-run
```

## Configuration

### Edit Configuration

```bash
sudo nano /etc/hora-police/config.toml
```

### Key Settings

- `cpu_threshold`: CPU usage threshold (default: 20.0)
- `auto_kill`: Enable automatic process killing (default: true)
- `threat_confidence_threshold`: Minimum confidence to kill (default: 0.7)
- `file_scanning.enabled`: Enable file scanning (default: true)
- `file_scanning.aggressive_cleanup`: Delete malware origins (default: true)

### Telegram Notifications (Optional)

Add to config:

```toml
[telegram]
bot_token = "YOUR_BOT_TOKEN"
chat_id = "YOUR_CHAT_ID"
daily_report_time = "09:00"
```

## Maintenance

### View Logs

```bash
# Follow logs
sudo journalctl -u hora-police -f

# Last 100 lines
sudo journalctl -u hora-police -n 100

# Since boot
sudo journalctl -u hora-police -b
```

### Restart Service

```bash
sudo systemctl restart hora-police
```

### Stop Service

```bash
sudo systemctl stop hora-police
```

### Update Application

```bash
cd /srv/Hora-Police
git pull
./deploy-vps.sh
# Or manually rebuild and restart
```

## System Requirements

- **OS**: Ubuntu 20.04+ (tested on 24.04)
- **RAM**: Minimum 1GB (2GB+ recommended for building)
- **Disk**: 500MB for application + database
- **CPU**: Any x86_64 processor

## Security Notes

- Service runs as `root` (required for process monitoring and killing)
- Service file includes security hardening (NoNewPrivileges, ProtectSystem, etc.)
- Database and logs stored in `/var/lib/hora-police` and `/var/log/hora-police`
- Quarantined files in `/var/lib/hora-police/quarantine`

## Support

For issues:
1. Check logs: `sudo journalctl -u hora-police -n 50`
2. Verify service file: `sudo cat /etc/systemd/system/hora-police.service`
3. Test binary: `sudo /usr/local/bin/hora-police --help`
4. Check system resources: `free -h`, `df -h`

