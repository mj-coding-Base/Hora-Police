# Hora-Police: Build and Run Guide

This guide provides step-by-step instructions for building and running Hora-Police on Ubuntu/Linux systems.

## Prerequisites

### System Requirements
- Ubuntu 20.04+ or compatible Linux distribution
- Root or sudo access
- At least 500MB free disk space
- At least 128MB free RAM
- Rust toolchain (will be installed if missing)

### Required System Packages

```bash
# Update package list
sudo apt update

# Install build essentials and dependencies
sudo apt install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    ca-certificates \
    curl \
    gnupg \
    lsb-release
```

## Step 1: Install Rust (if not already installed)

```bash
# Install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Load Rust environment
source $HOME/.cargo/env

# Set stable as default toolchain
rustup default stable

# Verify installation
rustc --version
cargo --version
```

## Step 2: Build the Application

### Option A: Using the Build Script (Recommended)

```bash
# Navigate to project directory
cd /path/to/Hora-Police

# Make build script executable
chmod +x build.sh

# Run build script (builds, strips, and installs)
./build.sh
```

The build script will:
- Fetch dependencies
- Build optimized release binary
- Strip debug symbols
- Install to `/usr/local/bin/hora-police`

### Option B: Manual Build

```bash
# Navigate to project directory
cd /path/to/Hora-Police

# Set environment variables for optimization
export PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig:/usr/lib/pkgconfig

# Build in release mode with optimizations
RUSTFLAGS="-C lto -C codegen-units=1 -C opt-level=z" cargo build --release -j$(nproc)

# Strip binary (optional, reduces size)
strip target/release/hora-police

# Install binary manually
sudo cp target/release/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
```

## Step 3: Setup Directories and Configuration

```bash
# Create required directories
sudo mkdir -p /etc/hora-police
sudo mkdir -p /var/lib/hora-police/{quarantine,rollbacks}
sudo mkdir -p /etc/hora-police/keys
sudo mkdir -p /var/log/hora-police

# Set permissions
sudo chown -R root:root /etc/hora-police /var/lib/hora-police
sudo chmod 755 /var/lib/hora-police
sudo chmod 700 /var/lib/hora-police/quarantine
sudo chmod 700 /etc/hora-police/keys

# Copy example configuration
sudo cp config.examples/hostinger_kvm4.toml /etc/hora-police/config.toml

# Edit configuration (required)
sudo nano /etc/hora-police/config.toml
```

### Configuration Essentials

At minimum, review these settings in `/etc/hora-police/config.toml`:

- **Telegram** (optional but recommended):
  ```toml
  [telegram]
  bot_token = "YOUR_BOT_TOKEN_HERE"
  chat_id = "@your_username"
  ```

- **Safety settings** (start with these for first run):
  ```toml
  dry_run = true          # No destructive actions
  audit_only = true      # Only log, don't kill
  auto_kill = false      # Disable automatic killing
  ```

- **File scanning paths** (adjust to your needs):
  ```toml
  [file_scanning]
  scan_paths = [
      "/home",
      "/tmp",
      "/var/tmp",
      "/var/www",
      "/srv",
  ]
  ```

## Step 4: Configure System Settings

### Increase Inotify Watchers (for file monitoring)

```bash
# Create sysctl configuration
echo "fs.inotify.max_user_watches=524288" | sudo tee /etc/sysctl.d/60-inotify.conf

# Apply immediately
sudo sysctl --system
```

### Setup tmpfiles (auto-create directories on boot)

```bash
# Copy tmpfiles configuration
sudo cp etc/tmpfiles.d/hora-police.conf /etc/tmpfiles.d/

# Apply immediately
sudo systemd-tmpfiles --create /etc/tmpfiles.d/hora-police.conf
```

## Step 5: Run the Application

### Option A: Run as systemd Service (Recommended for Production)

```bash
# Copy systemd service file
sudo cp hora-police.service /etc/systemd/system/

# Reload systemd
sudo systemctl daemon-reload

# Enable service (start on boot)
sudo systemctl enable hora-police

# Start service
sudo systemctl start hora-police

# Check status
sudo systemctl status hora-police

# View logs
sudo journalctl -u hora-police -f
```

### Option B: Run Manually (for Testing)

```bash
# Run with default config
sudo /usr/local/bin/hora-police

# Run with custom config
sudo /usr/local/bin/hora-police --config /etc/hora-police/config.toml

# Run in dry-run mode (CLI override)
sudo /usr/local/bin/hora-police --dry-run

# Run with telemetry probe endpoint
sudo /usr/local/bin/hora-police --probe
```

### Option C: Run in Foreground (for Debugging)

```bash
# Run directly (will show output in terminal)
sudo /usr/local/bin/hora-police /etc/hora-police/config.toml

# Press Ctrl+C to stop
```

## Step 6: Verify Installation

### Quick Verification

```bash
# Check if binary exists and is executable
ls -lh /usr/local/bin/hora-police

# Check service status
sudo systemctl status hora-police

# Check if process is running
ps aux | grep hora-police

# Check logs
sudo journalctl -u hora-police -n 50
```

### Comprehensive Verification

```bash
# Run verification script
chmod +x scripts/verify-deployment.sh
sudo ./scripts/verify-deployment.sh
```

The verification script checks:
- ✅ Systemd service status
- ✅ Resource limits
- ✅ Database WAL mode
- ✅ Inotify configuration
- ✅ Zombie processes
- ✅ Binary and permissions
- ✅ Directory structure
- ✅ Configuration validity
- ✅ Logging
- ✅ Watchdog status

## Step 7: Monitor and Maintain

### View Logs

```bash
# Follow logs in real-time
sudo journalctl -u hora-police -f

# View last 100 lines
sudo journalctl -u hora-police -n 100

# View logs from today
sudo journalctl -u hora-police --since today

# View errors only
sudo journalctl -u hora-police -p err
```

### Check Database

```bash
# Check database integrity
sqlite3 /var/lib/hora-police/intelligence.db "PRAGMA integrity_check;"

# View recent kill actions
sqlite3 /var/lib/hora-police/intelligence.db \
  "SELECT pid, binary_path, reason, confidence, timestamp FROM kill_actions ORDER BY timestamp DESC LIMIT 10;"

# View detected malware files
sqlite3 /var/lib/hora-police/intelligence.db \
  "SELECT file_path, signature_name, detected_at FROM malware_files ORDER BY detected_at DESC LIMIT 10;"
```

### Check Resource Usage

```bash
# Check CPU and memory usage
ps aux | grep hora-police

# Check systemd resource limits
systemctl show hora-police --property=CPUQuota --property=MemoryMax --property=TasksMax
```

## Troubleshooting

### Build Errors

**Error: `pkg-config` not found**
```bash
sudo apt install -y pkg-config
```

**Error: OpenSSL not found**
```bash
sudo apt install -y libssl-dev
export PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig:/usr/lib/pkgconfig
```

**Error: SQLite not found**
```bash
sudo apt install -y libsqlite3-dev
```

### Runtime Errors

**Service fails to start**
```bash
# Check logs for errors
sudo journalctl -u hora-police -n 50

# Common issues:
# - Config file syntax error
# - Missing directories
# - Permission issues
```

**Permission denied**
```bash
# Ensure binary is executable
sudo chmod +x /usr/local/bin/hora-police

# Check directory permissions
sudo chown -R root:root /etc/hora-police /var/lib/hora-police
```

**Database errors**
```bash
# Check database file permissions
ls -la /var/lib/hora-police/intelligence.db

# Recreate database if corrupted (WARNING: loses data)
sudo rm /var/lib/hora-police/intelligence.db
sudo systemctl restart hora-police
```

## Quick Reference Commands

```bash
# Start service
sudo systemctl start hora-police

# Stop service
sudo systemctl stop hora-police

# Restart service
sudo systemctl restart hora-police

# Check status
sudo systemctl status hora-police

# View logs
sudo journalctl -u hora-police -f

# Reload config (after editing config.toml)
sudo systemctl restart hora-police

# Disable service (prevent auto-start on boot)
sudo systemctl disable hora-police

# Enable service (allow auto-start on boot)
sudo systemctl enable hora-police
```

## Next Steps

After successful installation:

1. **Review Configuration**: Edit `/etc/hora-police/config.toml` to match your environment
2. **Start in Audit Mode**: Keep `dry_run = true` and `audit_only = true` for initial testing
3. **Monitor for 3-7 Days**: Watch logs and verify no false positives
4. **Transition to Canary Mode**: Set `canary_mode = true` and `auto_kill = true`
5. **Enable Full Enforcement**: After canary period, disable `dry_run` and `audit_only`

For detailed operational procedures, see:
- `HOSTINGER_KVM4_RUNBOOK.md` - Operational runbook
- `DEPLOYMENT_HOSTINGER.md` - Detailed deployment guide

