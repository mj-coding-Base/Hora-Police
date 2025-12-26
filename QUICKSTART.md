# 🚀 Quick Start Guide

Get Sentinel up and running in 5 minutes.

## Prerequisites

- Ubuntu 20.04+ VPS
- Root/sudo access
- Internet connection

## Step 1: Install Dependencies

```bash
sudo apt-get update
sudo apt-get install -y build-essential libsqlite3-dev pkg-config curl
```

## Step 2: Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

## Step 3: Build Sentinel

```bash
# Clone or navigate to project directory
cd sentinel-daemon

# Make build script executable
chmod +x build.sh

# Build
./build.sh
```

## Step 4: Install Binary

```bash
sudo cp target/release/sentinel-daemon /usr/local/bin/sentinel-daemon
sudo chmod +x /usr/local/bin/sentinel-daemon
```

## Step 5: Configure

```bash
# Create directories
sudo mkdir -p /etc/sentinel
sudo mkdir -p /var/lib/sentinel

# Copy example config
sudo cp config.toml.example /etc/sentinel/config.toml

# Edit config (optional - defaults work)
sudo nano /etc/sentinel/config.toml
```

## Step 6: Install Service

```bash
# Copy service file
sudo cp sentinel.service /etc/systemd/system/

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable sentinel
sudo systemctl start sentinel
```

## Step 7: Verify

```bash
# Check status
sudo systemctl status sentinel

# View logs
sudo journalctl -u sentinel -f
```

You should see:
```
🚀 Sentinel Anti-Malware Daemon starting...
✅ Configuration loaded from: /etc/sentinel/config.toml
🛡️  Sentinel daemon initialized. Starting monitoring...
```

## Optional: Setup Telegram (5 minutes)

1. Message [@BotFather](https://t.me/BotFather) → `/newbot`
2. Get bot token
3. Message [@userinfobot](https://t.me/userinfobot) → Get chat ID
4. Edit `/etc/sentinel/config.toml`:
   ```toml
   [telegram]
   bot_token = "YOUR_TOKEN"
   chat_id = "@mjpavithra"
   ```
5. Restart: `sudo systemctl restart sentinel`

## Test It

Create a test CPU-hogging process:

```bash
# This will be detected and killed within 5 minutes
while true; do :; done &
```

Check logs:
```bash
sudo journalctl -u sentinel -f
```

You should see detection and kill actions.

## Troubleshooting

**Daemon won't start?**
```bash
# Check config syntax
sentinel-daemon /etc/sentinel/config.toml

# Check permissions
ls -la /var/lib/sentinel/
```

**Build errors?**
- See `BUILD_NOTES.md`
- Ensure all dependencies installed
- Check Rust version: `rustc --version`

**No detections?**
- Verify processes are using CPU: `top`
- Check threshold in config (default: 20% for 5 min)
- Lower threshold for testing: `cpu_threshold = 10.0`

## Next Steps

- Read `README.md` for full documentation
- Review `ARCHITECTURE.md` for system design
- Check `TELEGRAM_SETUP.md` for reporting setup
- Customize config for your environment

## Uninstall

```bash
sudo systemctl stop sentinel
sudo systemctl disable sentinel
sudo rm /etc/systemd/system/sentinel.service
sudo rm /usr/local/bin/sentinel-daemon
sudo rm -rf /etc/sentinel
sudo rm -rf /var/lib/sentinel
```

---

**That's it!** Sentinel is now protecting your VPS. 🛡️

