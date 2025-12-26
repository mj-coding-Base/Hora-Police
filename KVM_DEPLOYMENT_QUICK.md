# ⚡ Quick Deployment: Ubuntu KVM (5 Minutes)

Fast-track deployment guide for experienced users.

## Prerequisites Check

```bash
# Must have: Ubuntu 20.04+, root/sudo, internet
lsb_release -a
sudo whoami  # Should show: root
```

## Installation Commands

```bash
# 1. Install dependencies
sudo apt-get update && sudo apt-get install -y build-essential libsqlite3-dev pkg-config curl

# 2. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 3. Build (from project directory)
cd /path/to/sentinel-daemon
cargo build --release

# 4. Install
sudo mkdir -p /etc/sentinel /var/lib/sentinel
sudo cp target/release/sentinel-daemon /usr/local/bin/
sudo cp config.toml.example /etc/sentinel/config.toml
sudo cp sentinel.service /etc/systemd/system/

# 5. Start service
sudo systemctl daemon-reload
sudo systemctl enable --now sentinel

# 6. Verify
sudo systemctl status sentinel
sudo journalctl -u sentinel -f
```

## One-Liner (Copy-Paste)

```bash
sudo apt-get update && sudo apt-get install -y build-essential libsqlite3-dev pkg-config curl && \
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
source $HOME/.cargo/env && \
cd /path/to/sentinel-daemon && \
cargo build --release && \
sudo mkdir -p /etc/sentinel /var/lib/sentinel && \
sudo cp target/release/sentinel-daemon /usr/local/bin/ && \
sudo cp config.toml.example /etc/sentinel/config.toml && \
sudo cp sentinel.service /etc/systemd/system/ && \
sudo systemctl daemon-reload && \
sudo systemctl enable --now sentinel && \
echo "✅ Sentinel deployed! Check: sudo systemctl status sentinel"
```

## Post-Deployment

```bash
# View logs
sudo journalctl -u sentinel -f

# Test detection
while true; do :; done &  # Will be killed in ~5 min

# Check database
sudo sqlite3 /var/lib/sentinel/intelligence.db "SELECT COUNT(*) FROM kill_actions;"
```

**For detailed steps, see `DEPLOYMENT_GUIDE.md`**

