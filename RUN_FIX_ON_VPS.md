# Instructions: Run Fix Script on VPS

## Prerequisites

1. SSH access to VPS as `deploy` user
2. Binary must exist at `/usr/local/bin/hora-police` OR be ready to copy

## Step 1: Pull Latest Code

```bash
cd /srv/Hora-Police
git pull
```

## Step 2: Make Script Executable and Run

```bash
chmod +x fix-service-directories.sh
./fix-service-directories.sh
```

## Step 3: If Binary is Missing

The script will exit with error code 2 and show remediation instructions. Follow the instructions in `REMEDIATION_BINARY_MISSING.md` or:

**Quick fix from WSL (if binary exists locally)**:

```bash
# On your Windows machine, in WSL:
cd /mnt/f/Personal_Projects/Hora-Police
scp target/release/hora-police deploy@<VPS_IP>:/tmp/
```

**Then on VPS**:

```bash
sudo mv /tmp/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
./fix-service-directories.sh
```

## Step 4: Verify

After the script completes, run verification commands from `VERIFICATION_COMMANDS.md`:

```bash
# Service status
sudo systemctl status hora-police --no-pager

# Check for errors
sudo journalctl -u hora-police -n 200 --no-pager | egrep -i 'NAMESPACE|EXEC|Failed|error' || echo "No errors"

# Directory check
ls -la /etc/hora-police /var/lib/hora-police /var/log/hora-police

# Binary check
stat /usr/local/bin/hora-police
file /usr/local/bin/hora-police
ldd /usr/local/bin/hora-police
```

## Expected Results

✅ Service status shows: `Active: active (running)`
✅ No NAMESPACE or EXEC errors in journal
✅ All directories exist with correct permissions
✅ Binary is executable and has correct dependencies
✅ Service restart count is low (not flapping)

