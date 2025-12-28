# Quick Fix for OOM Issues

## Immediate Solution

Your VPS is out of memory. Do this FIRST:

```bash
# Add 4GB swap (this will prevent OOM kills)
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab

# Verify swap is active
free -h
```

## Then Try Again

After adding swap, retry the build:

```bash
cd /srv/Hora-Police

# Try git pull again (should work now)
git pull || echo "Git pull failed, continuing with existing code..."

# Create install script manually (if scripts/install-binary.sh doesn't exist)
cat > /tmp/install-binary.sh << 'INSTALL_EOF'
#!/usr/bin/env bash
set -euo pipefail
BINARY_SOURCE="/tmp/hora-police"
BINARY_DEST="/usr/local/bin/hora-police"
sudo systemctl stop hora-police 2>/dev/null || true
sudo cp "${BINARY_SOURCE}" "${BINARY_DEST}"
sudo chown root:root "${BINARY_DEST}"
sudo chmod 755 "${BINARY_DEST}"
sudo mkdir -p /etc/hora-police /var/lib/hora-police/quarantine /var/log/hora-police
sudo chown -R root:root /etc/hora-police /var/lib/hora-police /var/log/hora-police
sudo chmod 0755 /etc/hora-police /var/lib/hora-police /var/log/hora-police
sudo chmod 0700 /var/lib/hora-police/quarantine
sudo systemctl daemon-reload
sudo systemctl start hora-police
sleep 3
sudo systemctl status hora-police --no-pager | head -20
INSTALL_EOF
chmod +x /tmp/install-binary.sh

# Build (should work with swap)
chmod +x build-lowmem.sh
./build-lowmem.sh

# Install
cp target/release/hora-police /tmp/hora-police
/tmp/install-binary.sh
```

## If Build Still Fails

Build on your local machine and transfer:

```bash
# On your local machine
cd /path/to/Hora-Police
cargo build --release
scp target/release/hora-police deploy@mail-server:/tmp/hora-police

# On VPS
/tmp/install-binary.sh
```

