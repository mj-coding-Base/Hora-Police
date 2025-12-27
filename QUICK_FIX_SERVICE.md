# Quick Fix for Service NAMESPACE Error

## Problem
Service fails with: `Failed to set up mount namespacing: /var/log/hora-police: No such file or directory`

## Solution

Run these commands on your VPS:

```bash
# 1. Create the missing directory
sudo mkdir -p /var/log/hora-police
sudo chown root:root /var/log/hora-police
sudo chmod 755 /var/log/hora-police

# 2. Or use the fix script
cd /srv/Hora-Police
git pull
chmod +x fix-service-directories.sh
./fix-service-directories.sh

# 3. Start service
sudo systemctl start hora-police
sudo systemctl status hora-police
```

## Alternative: Remove /var/log/hora-police from Service File

If you don't need a separate log directory, you can remove it from the service file:

```bash
sudo nano /etc/systemd/system/hora-police.service
```

Change:
```
ReadWritePaths=/var/lib/hora-police /etc/hora-police /var/log/hora-police
```

To:
```
ReadWritePaths=/var/lib/hora-police /etc/hora-police
```

Then:
```bash
sudo systemctl daemon-reload
sudo systemctl restart hora-police
```

## Permanent Fix

The deployment script and tmpfiles.d configuration will ensure this directory exists in the future.

