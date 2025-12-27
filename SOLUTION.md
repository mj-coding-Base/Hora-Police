# Complete Solution - Build OOM and Service Issues

## The Problem
1. Build killed (OOM) - Even debug build fails
2. Service 226/NAMESPACE - Service file needs fixing
3. No binary exists - Build never completes

## Solution: Build on Different Machine + Fix Service

Since local build keeps failing, we'll build elsewhere and transfer the binary.

---

## PART 1: Build Binary on Your Windows Machine

### Option A: Using WSL (Recommended)

```bash
# Open WSL terminal on Windows
cd /mnt/f/Personal_Projects/Hora-Police

# Build release
cargo build --release

# Binary will be at: target/release/hora-police
# Transfer to server (from WSL):
scp target/release/hora-police deploy@mail-server:/tmp/hora-police
```

### Option B: Using Docker (If WSL not available)

```bash
# On Windows PowerShell
docker run -it -v F:\Personal_Projects\Hora-Police:/build rust:1.92 bash

# Inside container:
cd /build
cargo build --release
exit

# Copy binary from container
docker cp <container-id>:/build/target/release/hora-police ./hora-police

# Transfer to server
scp hora-police deploy@mail-server:/tmp/hora-police
```

### Option C: Build on Temporary Cloud Instance

1. Create temporary VM (DigitalOcean/AWS/Linode) with 2GB+ RAM
2. SSH into it
3. Run:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env
git clone <your-repo> Hora-Police
cd Hora-Police
cargo build --release
scp target/release/hora-police deploy@mail-server:/tmp/hora-police
```

---

## PART 2: On Your Server - Complete Fix

Run these commands **in order**:

```bash
# 1. Verify binary was transferred
ls -lh /tmp/hora-police

# 2. Stop service
sudo systemctl stop hora-police

# 3. Install binary
sudo cp /tmp/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police

# 4. Test binary works
sudo /usr/local/bin/hora-police --help

# 5. Fix service file (use the script)
cd /srv/Hora-Police
git pull
chmod +x fix-service-complete.sh
./fix-service-complete.sh

# 6. Start service
sudo systemctl start hora-police

# 7. Check status
sudo systemctl status hora-police

# 8. View logs
sudo journalctl -u hora-police -f
```

---

## PART 3: Manual Service File Fix (If Script Doesn't Work)

If the script doesn't work, manually fix:

```bash
# Stop service
sudo systemctl stop hora-police

# Backup current file
sudo cp /etc/systemd/system/hora-police.service /etc/systemd/system/hora-police.service.backup

# Create correct service file
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

# Reload and start
sudo systemctl daemon-reload
sudo systemctl start hora-police
sudo systemctl status hora-police
```

---

## Verification

After installation, verify everything:

```bash
# 1. Check binary exists and works
ls -lh /usr/local/bin/hora-police
/usr/local/bin/hora-police --help

# 2. Check service file is correct
sudo grep -E "Type=|PrivateTmp|ProtectSystem" /etc/systemd/system/hora-police.service
# Should show:
# Type=simple
# ProtectSystem=full
# (No PrivateTmp line)

# 3. Check service status
sudo systemctl status hora-police
# Should show: Active: active (running)

# 4. Check logs
sudo journalctl -u hora-police -n 20
# Should show startup messages, no errors
```

---

## If Service Still Fails

Try minimal service file (no security restrictions):

```bash
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

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl start hora-police
```

---

## Quick Reference

**Build on Windows (WSL):**
```bash
cd /mnt/f/Personal_Projects/Hora-Police
cargo build --release
scp target/release/hora-police deploy@mail-server:/tmp/hora-police
```

**Install on Server:**
```bash
sudo cp /tmp/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
cd /srv/Hora-Police && ./fix-service-complete.sh
sudo systemctl start hora-police
```

