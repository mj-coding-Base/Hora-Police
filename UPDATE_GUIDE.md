# 🔄 Hora-Police Update Guide

## Quick Update (Single Command)

```bash
cd /srv/Hora-Police
chmod +x scripts/update.sh
sudo ./scripts/update.sh
```

## Step-by-Step Update Process

### 1. Navigate to Repository

```bash
cd /srv/Hora-Police
```

### 2. Make Script Executable

```bash
chmod +x scripts/update.sh
```

### 3. Pull Latest Code (if needed)

```bash
git pull
```

### 4. Run Update Script

```bash
sudo ./scripts/update.sh
```

## Update Options

### Standard Update
```bash
sudo ./scripts/update.sh
```
- Checks version, backs up, pulls code, builds, installs, verifies
- Automatically rolls back on failure

### Preview Changes (Dry-Run)
```bash
sudo ./scripts/update.sh --dry-run
```
- Shows what would be updated without making changes

### Force Update
```bash
sudo ./scripts/update.sh --force
```
- Updates even if versions match

### Update from Specific Branch
```bash
sudo ./scripts/update.sh --branch=main
```

## Troubleshooting

### Script Not Found or Not Executable

```bash
# Make sure you're in the right directory
cd /srv/Hora-Police

# Make script executable
chmod +x scripts/update.sh

# Verify it exists
ls -l scripts/update.sh

# Run it
sudo ./scripts/update.sh
```

### Binary Not Found

If you get "binary not found" error, the update script will:
1. Build the binary automatically
2. Install it
3. Start the service

If build fails, see "Build Issues" below.

### Build Issues

If the build fails (OOM or other errors):

```bash
# Add swap space (4GB)
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile

# Verify swap
free -h

# Retry update
sudo ./scripts/update.sh
```

### Service Won't Start After Update

The update script automatically rolls back on failure. If rollback occurs:

1. Check logs:
   ```bash
   sudo journalctl -u hora-police -n 100
   sudo cat /var/log/hora-police/update.log
   ```

2. Check service status:
   ```bash
   sudo systemctl status hora-police
   ```

3. Manual rollback (if needed):
   ```bash
   # List backups
   ls -l /var/lib/hora-police/backups/
   
   # Restore from backup
   sudo cp /var/lib/hora-police/backups/hora-police-YYYYMMDD-HHMMSS /usr/local/bin/hora-police
   sudo chmod +x /usr/local/bin/hora-police
   sudo systemctl restart hora-police
   ```

## Manual Update (Alternative)

If the update script doesn't work, you can update manually:

```bash
cd /srv/Hora-Police

# 1. Pull latest code
git pull

# 2. Make build script executable
chmod +x build-lowmem.sh

# 3. Build
./build-lowmem.sh

# 4. Copy to /tmp
cp target/release/hora-police /tmp/hora-police

# 5. Install
chmod +x scripts/install-binary.sh
sudo ./scripts/install-binary.sh
```

## Verify Update

After updating, verify everything works:

```bash
# Check version
/usr/local/bin/hora-police --version

# Check service status
sudo systemctl status hora-police

# Check logs
sudo journalctl -u hora-police -n 50
```

## Update Logs

All update actions are logged to:
```bash
sudo cat /var/log/hora-police/update.log
```

## Backup Location

Backups are stored in:
```bash
ls -l /var/lib/hora-police/backups/
```

The script keeps the last 5 backups automatically.

