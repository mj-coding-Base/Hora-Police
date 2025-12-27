# Immediate Fix - Run These Commands on Server

## The Problem
- Binary not built/transferred yet
- Service file still has namespace issues
- Scripts not on server

## Solution: Fix Service File First, Then Build

### Step 1: Fix Service File (Run This Now)

```bash
# Stop service
sudo systemctl stop hora-police

# Fix service file directly
sudo tee /etc/systemd/system/hora-police.service > /dev/null << 'EOF'
[Unit]
Description=Hora-Police Anti-Malware Daemon
After=network.target

[Service]
Type=simple
User=root
ExecStart=/usr/local/bin/hora-police /etc/hora-police/config.toml
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal
CPUQuota=15%
MemoryMax=128M
TasksMax=1024
NoNewPrivileges=true
ProtectSystem=full
ProtectHome=true
ReadOnlyPaths=/proc /sys
ReadWritePaths=/var/lib/hora-police /etc/hora-police /var/log/hora-police

[Install]
WantedBy=multi-user.target
EOF

# Reload systemd
sudo systemctl daemon-reload

# Verify the fix
sudo grep -E "Type=|PrivateTmp" /etc/systemd/system/hora-police.service
# Should show: Type=simple (and no PrivateTmp line)
```

### Step 2: Build Binary on Windows (WSL)

On your Windows machine, open WSL and run:

```bash
cd /mnt/f/Personal_Projects/Hora-Police
cargo build --release
scp target/release/hora-police deploy@mail-server:/tmp/hora-police
```

### Step 3: Install Binary on Server

```bash
# Install binary
sudo cp /tmp/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police

# Test it works
sudo /usr/local/bin/hora-police --help

# Start service
sudo systemctl start hora-police
sudo systemctl status hora-police
```

## Alternative: If You Can't Build on Windows

Try building on server with even more minimal settings:

```bash
# Load Rust
source $HOME/.cargo/env

# Try building with absolute minimum
cd /srv/Hora-Police
ulimit -v unlimited
RUSTFLAGS="-C opt-level=0" cargo build --target-dir /tmp/hora-build -j1

# If successful, install
sudo cp /tmp/hora-build/debug/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
```

