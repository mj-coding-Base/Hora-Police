# Installing Pre-built Binary

If you have the binary file, use these steps to install it.

## Step 1: Transfer Binary to Server

If binary is on your local machine:

```bash
# From Windows (WSL or PowerShell with OpenSSH)
scp /path/to/hora-police deploy@mail-server:/tmp/hora-police

# Or use WinSCP/FileZilla to upload to /tmp/hora-police
```

## Step 2: Install on Server

```bash
# 1. Stop service
sudo systemctl stop hora-police

# 2. Install binary
sudo cp /tmp/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police

# 3. Verify binary works
sudo /usr/local/bin/hora-police --help

# Should show help text or version info
```

## Step 3: Fix Service File

```bash
# Make fix script executable
chmod +x fix-service-complete.sh

# Run fix
./fix-service-complete.sh

# Or manually fix:
sudo nano /etc/systemd/system/hora-police.service
# Ensure it has Type=simple (not notify)
# Ensure PrivateTmp is removed
# Ensure ProtectSystem=full (not strict)
```

## Step 4: Start Service

```bash
# Reload systemd
sudo systemctl daemon-reload

# Start service
sudo systemctl start hora-police

# Check status
sudo systemctl status hora-police

# View logs
sudo journalctl -u hora-police -f
```

## Complete Installation Script

```bash
#!/bin/bash
set -e

BINARY_PATH="/tmp/hora-police"

if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ Binary not found at $BINARY_PATH"
    echo "Please transfer binary first:"
    echo "  scp hora-police deploy@mail-server:/tmp/hora-police"
    exit 1
fi

echo "=== Installing Hora-Police ==="

# Stop service
sudo systemctl stop hora-police 2>/dev/null || true

# Install binary
echo "Installing binary..."
sudo cp "$BINARY_PATH" /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police

# Verify
if /usr/local/bin/hora-police --help >/dev/null 2>&1; then
    echo "✅ Binary verified"
else
    echo "⚠️  Binary test failed, but continuing..."
fi

# Fix service
echo "Fixing service file..."
chmod +x fix-service-complete.sh
./fix-service-complete.sh

# Start service
echo "Starting service..."
sudo systemctl start hora-police
sleep 2

# Check status
sudo systemctl status hora-police --no-pager -l | head -20

if systemctl is-active --quiet hora-police; then
    echo ""
    echo "✅ Installation complete! Service is running."
    echo "View logs: sudo journalctl -u hora-police -f"
else
    echo ""
    echo "❌ Service failed to start"
    echo "Check: sudo journalctl -u hora-police -n 50"
fi
```

Save as `install-prebuilt.sh`, make executable, and run:
```bash
chmod +x install-prebuilt.sh
./install-prebuilt.sh
```

