# Exact Commands to Run on VPS

## Complete Build and Deploy Sequence

Copy and paste these commands in order on your VPS:

```bash
# 1. Navigate to repo
cd /srv/Hora-Police

# 2. Pull latest changes (with fixes)
git pull

# 3. Make scripts executable
chmod +x build-lowmem.sh scripts/install-binary.sh

# 4. Build with low-memory profile (10-20 minutes)
./build-lowmem.sh

# 5. Copy binary to /tmp for installation
cp target/release/hora-police /tmp/hora-police

# 6. Install binary and start service
./scripts/install-binary.sh
```

## Verification Commands

After installation, verify everything works:

```bash
# Check service status
sudo systemctl status hora-police --no-pager

# Should show: Active: active (running)

# Check for errors
sudo journalctl -u hora-police -n 50 --no-pager | grep -iE 'EXEC|NAMESPACE|error' || echo "✅ No errors"

# Verify binary
file /usr/local/bin/hora-police
ldd /usr/local/bin/hora-police

# Verify directories
ls -la /etc/hora-police /var/lib/hora-police /var/log/hora-police

# Test binary execution
sudo /usr/local/bin/hora-police --help
```

## One-Liner Verification

```bash
test -f /usr/local/bin/hora-police && test -x /usr/local/bin/hora-police && systemctl is-active --quiet hora-police && echo "✅ All checks passed" || echo "❌ Some checks failed"
```

## If Build Fails with OOM

```bash
# Add 2GB swap
sudo fallocate -l 2G /swapfile && sudo chmod 600 /swapfile && sudo mkswap /swapfile && sudo swapon /swapfile

# Verify swap
free -h

# Retry build
./build-lowmem.sh
```

## Troubleshooting

### Service shows 203/EXEC
```bash
# Check binary exists
ls -l /usr/local/bin/hora-police

# Test binary
sudo /usr/local/bin/hora-police --help

# Check architecture
file /usr/local/bin/hora-police
```

### Service shows 226/NAMESPACE
```bash
# Ensure directories exist
sudo mkdir -p /var/log/hora-police
sudo systemd-tmpfiles --create /etc/tmpfiles.d/hora-police.conf

# Restart service
sudo systemctl restart hora-police
```

