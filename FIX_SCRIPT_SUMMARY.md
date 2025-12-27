# Hora-Police Service Fix - Complete Summary

## Files Created/Updated

### 1. `fix-service-directories.sh` ✅
- **Location**: `/srv/Hora-Police/fix-service-directories.sh`
- **Purpose**: Idempotent fix script for service directory and systemd unit issues
- **Status**: Created and ready to execute

### 2. `hora-police.service` ✅
- **Location**: `/srv/Hora-Police/hora-police.service`
- **Changes**:
  - Moved `StartLimitIntervalSec=300` and `StartLimitBurst=5` to `[Unit]` section
  - Changed `ProtectSystem=full` to `ProtectSystem=strict`
  - Set `PrivateTmp=false` explicitly
  - Changed `RestartSec=5` to `RestartSec=10`
- **Status**: Updated

### 3. `etc/tmpfiles.d/hora-police.conf` ✅
- **Location**: `/srv/Hora-Police/etc/tmpfiles.d/hora-police.conf`
- **Purpose**: Auto-create directories on boot
- **Status**: Updated (removed duplicate entry)

### 4. Documentation Files ✅
- `VERIFICATION_COMMANDS.md` - Verification steps
- `REMEDIATION_BINARY_MISSING.md` - Binary copy instructions
- `RUN_FIX_ON_VPS.md` - Quick start guide

## Systemd Unit File Content

The script will install this unit file:

```ini
[Unit]
Description=Hora-Police Anti-Malware Daemon
After=network.target
StartLimitIntervalSec=300
StartLimitBurst=5

[Service]
Type=simple
User=root
ExecStart=/usr/local/bin/hora-police /etc/hora-police/config.toml
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal
CPUQuota=15%
MemoryMax=128M
TasksMax=1024
NoNewPrivileges=true
PrivateTmp=false
ProtectSystem=strict
ProtectHome=true
ReadOnlyPaths=/proc /sys
ReadWritePaths=/var/lib/hora-police /etc/hora-police /var/log/hora-police

[Install]
WantedBy=multi-user.target
```

## tmpfiles.d Configuration

The script will install this at `/etc/tmpfiles.d/hora-police.conf`:

```
# Hora-Police directory structure
# Auto-create directories on boot with proper permissions
# Type Path          Mode UID  GID Age Argument

d /var/lib/hora-police 0755 root root -
d /var/lib/hora-police/quarantine 0700 root root -
d /var/lib/hora-police/rollbacks 0755 root root -
d /etc/hora-police 0755 root root -
d /etc/hora-police/keys 0700 root root -
d /var/log/hora-police 0755 root root -
```

## Execution Instructions

### On VPS (as deploy user):

```bash
# 1. Navigate to repo
cd /srv/Hora-Police

# 2. Pull latest code
git pull

# 3. Make script executable
chmod +x fix-service-directories.sh

# 4. Run fix script
./fix-service-directories.sh
```

### If Binary is Missing:

The script will exit with code 2 and show instructions. Quick fix:

**From WSL (Windows)**:
```bash
cd /mnt/f/Personal_Projects/Hora-Police
scp target/release/hora-police deploy@<VPS_IP>:/tmp/
```

**On VPS**:
```bash
sudo mv /tmp/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
./fix-service-directories.sh
```

## Verification Commands

After script execution, run:

```bash
# 1. Service status
sudo systemctl status hora-police --no-pager

# 2. Error check
sudo journalctl -u hora-police -n 200 --no-pager | egrep -i 'NAMESPACE|EXEC|Failed|error' || echo "No errors"

# 3. Directories
ls -la /etc/hora-police /var/lib/hora-police /var/log/hora-police

# 4. Binary
stat /usr/local/bin/hora-police
file /usr/local/bin/hora-police
ldd /usr/local/bin/hora-police

# 5. Database
sudo sqlite3 /var/lib/hora-police/intelligence.db "PRAGMA journal_mode;" || echo "DB will be created on first run"
```

## Expected Outcomes

### Success Indicators:
- ✅ `systemctl status` shows `Active: active (running)`
- ✅ No `226/NAMESPACE` or `203/EXEC` errors in journal
- ✅ All directories exist with correct permissions
- ✅ Binary is executable and dependencies resolved
- ✅ Service restart count is stable (not increasing rapidly)

### Failure Indicators:
- ❌ `status=226/NAMESPACE` - Directories missing (script should fix this)
- ❌ `status=203/EXEC` - Binary missing or not executable (see REMEDIATION_BINARY_MISSING.md)
- ❌ Service keeps restarting - Check journal for crash reasons

## Script Safety Features

1. **Idempotent**: Safe to run multiple times
2. **Backup**: Backs up existing service file before replacing
3. **Validation**: Checks binary existence and executability
4. **Error Handling**: Exits with clear error codes and messages
5. **No Data Loss**: Only creates directories, doesn't delete user files
6. **Minimal Sudo**: Only uses sudo where required

## Next Steps

1. **Commit and push** these files to repository
2. **SSH to VPS** and pull latest code
3. **Run** `fix-service-directories.sh`
4. **Verify** using commands above
5. **Monitor** service for 5-10 minutes to ensure stability

