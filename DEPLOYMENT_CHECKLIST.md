# ✅ Sentinel Deployment Checklist

Use this checklist to ensure proper deployment on Ubuntu KVM.

## Pre-Deployment

- [ ] **System Check**
  - [ ] Ubuntu 20.04+ verified: `lsb_release -a`
  - [ ] x86_64 architecture: `uname -m`
  - [ ] Root/sudo access: `sudo whoami`
  - [ ] Internet connectivity: `ping google.com`
  - [ ] At least 512MB free RAM: `free -h`
  - [ ] At least 100MB free disk: `df -h`

## Installation

- [ ] **System Preparation**
  - [ ] System updated: `sudo apt-get update && sudo apt-get upgrade -y`
  - [ ] Build tools installed: `sudo apt-get install -y build-essential libsqlite3-dev pkg-config curl`
  - [ ] Rust installed: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
  - [ ] Rust in PATH: `source $HOME/.cargo/env`
  - [ ] Rust verified: `rustc --version` and `cargo --version`

- [ ] **Source Code**
  - [ ] Project directory accessible
  - [ ] All source files present: `ls -la src/`
  - [ ] Cargo.toml exists: `cat Cargo.toml | head -5`

- [ ] **Build**
  - [ ] Build successful: `cargo build --release`
  - [ ] Binary exists: `ls -lh target/release/sentinel-daemon`
  - [ ] Binary executable: `file target/release/sentinel-daemon`

## Configuration

- [ ] **Directories Created**
  - [ ] `/etc/sentinel` exists: `ls -ld /etc/sentinel`
  - [ ] `/var/lib/sentinel` exists: `ls -ld /var/lib/sentinel`
  - [ ] Permissions correct: `ls -la /etc/sentinel /var/lib/sentinel`

- [ ] **Binary Installed**
  - [ ] Binary copied: `ls -lh /usr/local/bin/sentinel-daemon`
  - [ ] Binary executable: `test -x /usr/local/bin/sentinel-daemon && echo "OK"`
  - [ ] In PATH: `which sentinel-daemon`

- [ ] **Configuration File**
  - [ ] Config exists: `ls -la /etc/sentinel/config.toml`
  - [ ] Config readable: `sudo cat /etc/sentinel/config.toml | head -10`
  - [ ] Config edited (if needed): Review settings
  - [ ] Permissions: `ls -l /etc/sentinel/config.toml` (should be 644)

- [ ] **systemd Service**
  - [ ] Service file copied: `ls -la /etc/systemd/system/sentinel.service`
  - [ ] Service file reviewed: Check ExecStart path
  - [ ] systemd reloaded: `sudo systemctl daemon-reload`
  - [ ] Service enabled: `sudo systemctl enable sentinel`

## Startup & Verification

- [ ] **Service Started**
  - [ ] Service started: `sudo systemctl start sentinel`
  - [ ] Service active: `sudo systemctl status sentinel | grep "Active: active"`
  - [ ] Service enabled: `sudo systemctl is-enabled sentinel`

- [ ] **Logs Verified**
  - [ ] Logs accessible: `sudo journalctl -u sentinel -n 10`
  - [ ] Startup message present: Look for "Starting monitoring..."
  - [ ] No errors: Check for ERROR or FAILED messages
  - [ ] Configuration loaded: Look for "Configuration loaded"

- [ ] **Process Running**
  - [ ] Process exists: `ps aux | grep sentinel-daemon | grep -v grep`
  - [ ] Running as root: `ps aux | grep sentinel-daemon | grep "^root"`
  - [ ] Low CPU: `top -p $(pgrep sentinel-daemon)` shows <1%
  - [ ] Low memory: Shows ~30-40MB

- [ ] **Database**
  - [ ] Database file created: `ls -lh /var/lib/sentinel/intelligence.db`
  - [ ] Database accessible: `sudo sqlite3 /var/lib/sentinel/intelligence.db ".tables"`
  - [ ] Tables exist: Should show 5 tables
  - [ ] Database growing: Check size over time

## Testing

- [ ] **Detection Test**
  - [ ] Test process created: `while true; do :; done &`
  - [ ] Process detected: Check logs within 5 minutes
  - [ ] Process killed: Verify process no longer exists
  - [ ] Kill logged: Check database `kill_actions` table

- [ ] **Cron Monitoring**
  - [ ] Test cron added: `echo "* * * * * echo test" | sudo crontab -`
  - [ ] Cron detected: Check logs for cron scan
  - [ ] Test cron removed: `sudo crontab -r`

- [ ] **Resource Usage**
  - [ ] CPU <1%: Monitor with `top`
  - [ ] Memory <50MB: Check with `ps aux`
  - [ ] No memory leaks: Monitor over 24 hours

## Optional: Telegram Setup

- [ ] **Bot Created**
  - [ ] Bot created via @BotFather
  - [ ] Bot token obtained
  - [ ] Bot tested: Sent message to bot

- [ ] **Chat ID**
  - [ ] Username or numeric ID obtained
  - [ ] Chat ID verified

- [ ] **Configuration**
  - [ ] Telegram section added to config
  - [ ] Bot token set: `sudo grep bot_token /etc/sentinel/config.toml`
  - [ ] Chat ID set: `sudo grep chat_id /etc/sentinel/config.toml`
  - [ ] Report time set: `sudo grep daily_report_time /etc/sentinel/config.toml`

- [ ] **Telegram Verified**
  - [ ] Service restarted: `sudo systemctl restart sentinel`
  - [ ] No Telegram errors: Check logs
  - [ ] Test message sent (if real_time_alerts enabled)
  - [ ] Daily report received (wait until configured time)

## Maintenance

- [ ] **Monitoring Setup**
  - [ ] Log rotation configured (optional)
  - [ ] Database backup scheduled (optional)
  - [ ] Alerting configured (if needed)

- [ ] **Documentation**
  - [ ] Deployment steps documented
  - [ ] Configuration changes noted
  - [ ] Custom settings recorded

## Troubleshooting Reference

If any step fails, check:

- [ ] **Build Issues**: See `BUILD_NOTES.md`
- [ ] **Service Issues**: `sudo journalctl -u sentinel -n 50`
- [ ] **Permission Issues**: `ls -la /etc/sentinel /var/lib/sentinel`
- [ ] **Config Issues**: `sudo sentinel-daemon /etc/sentinel/config.toml`
- [ ] **Network Issues**: `ping api.telegram.org` (for Telegram)

## Quick Verification Commands

```bash
# Status check
sudo systemctl status sentinel

# Logs
sudo journalctl -u sentinel -n 20

# Process
ps aux | grep sentinel-daemon

# Database
sudo sqlite3 /var/lib/sentinel/intelligence.db "SELECT COUNT(*) FROM kill_actions;"

# Config
sudo cat /etc/sentinel/config.toml

# Resource usage
top -p $(pgrep sentinel-daemon)
```

## Success Criteria

✅ **Deployment is successful when:**

1. Service shows `active (running)`
2. Logs show "Starting monitoring..."
3. Process visible and using <1% CPU
4. Database file exists and accessible
5. Test process detected and killed
6. No errors in logs
7. Telegram reports working (if configured)

---

**Deployment Date**: _______________

**Deployed By**: _______________

**Notes**: _______________

