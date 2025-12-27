# Verification Commands for Hora-Police Service Fix

After running `fix-service-directories.sh`, execute these verification commands:

## 1. Service Status Check

```bash
sudo systemctl status hora-police --no-pager
```

**Expected Output (Success)**:
```
● hora-police.service - Hora-Police Anti-Malware Daemon
     Loaded: loaded (/etc/systemd/system/hora-police.service; enabled; preset: enabled)
     Active: active (running) since [timestamp]
   Main PID: [pid] (hora-police)
      Tasks: 1 (limit: 1024)
     Memory: [size] (max: 128.0M)
        CPU: [time]
```

**If Failed**: Check journal for specific error codes (226/NAMESPACE or 203/EXEC)

## 2. Error Check in Journal

```bash
sudo journalctl -u hora-police -n 200 --no-pager | egrep -i 'NAMESPACE|EXEC|Failed|error' || echo "No errors found"
```

**Expected Output (Success)**: `No errors found` or empty output

**If Errors Found**: Will show lines containing NAMESPACE, EXEC, Failed, or error

## 3. Directory Verification

```bash
ls -la /etc/hora-police /var/lib/hora-police /var/log/hora-police
```

**Expected Output**:
```
/etc/hora-police:
total [size]
drwxr-xr-x  root root  [date] .
drwxr-xr-x  root root  [date] ..
-rw-r--r--  root root  [date] config.toml

/var/lib/hora-police:
total [size]
drwxr-xr-x  root root  [date] .
drwxr-xr-x  root root  [date] ..
drwx------  root root  [date] quarantine
-rw-r--r--  root root  [date] intelligence.db (if exists)

/var/log/hora-police:
total [size]
drwxr-xr-x  root root  [date] .
drwxr-xr-x  root root  [date] ..
```

## 4. Binary Verification

```bash
stat /usr/local/bin/hora-police
```

**Expected Output**:
```
  File: /usr/local/bin/hora-police
  Size: [size]        Blocks: [blocks]   IO Block: 4096   regular file
Device: [device]     Inode: [inode]     Links: 1
Access: (0755/-rwxr-xr-x)  Uid: (    0/    root)   Gid: (    0/    root)
Access: [timestamp]
Modify: [timestamp]
Change: [timestamp]
 Birth: -
```

## 5. Binary File Type Check

```bash
file /usr/local/bin/hora-police
```

**Expected Output**:
```
/usr/local/bin/hora-police: ELF 64-bit LSB executable, x86-64, version 1 (SYSV), dynamically linked, interpreter /lib64/ld-linux-x86-64.so.2, BuildID=[id], for GNU/Linux [version], stripped
```

**If Missing**: Binary not found - need to copy prebuilt binary

## 6. Binary Dependencies Check

```bash
ldd /usr/local/bin/hora-police
```

**Expected Output**:
```
        linux-vdso.so.1 (0x00007ffc[address])
        libc.so.6 => /lib/x86_64-linux-gnu/libc.so.6 (0x00007f[address])
        /lib64/ld-linux-x86-64.so.2 (0x00007f[address])
```

**If Static Binary**: May show "not a dynamic executable" - this is OK

**If Missing Dependencies**: Will show "not found" for required libraries

## 7. Database Check

```bash
sudo sqlite3 /var/lib/hora-police/intelligence.db "PRAGMA journal_mode;" || echo "DB missing or not createable"
```

**Expected Output (Success)**:
```
wal
```

**If Missing**: Database will be created on first run

## 8. Service Restart Count Check

```bash
sudo systemctl show hora-police -p NRestarts --value
```

**Expected Output (Success)**: `0` or a low number (not rapidly increasing)

**If High/Increasing**: Service is flapping - check journal for errors

## 9. Full Journal Check (Last 50 Lines)

```bash
sudo journalctl -u hora-police -n 50 --no-pager
```

**Expected Output (Success)**: Recent log entries showing normal operation, no error messages

## 10. Process Check

```bash
ps aux | grep hora-police | grep -v grep
```

**Expected Output (Success)**:
```
root      [pid]  0.0  [cpu%]  [mem%]  [timestamp]  /usr/local/bin/hora-police /etc/hora-police/config.toml
```

## Troubleshooting

### If status=226/NAMESPACE:
- Check all directories in `ReadWritePaths` exist
- Verify tmpfiles.d config is installed
- Check directory permissions

### If status=203/EXEC:
- Verify binary exists: `ls -l /usr/local/bin/hora-police`
- Check binary is executable: `file /usr/local/bin/hora-police`
- Check binary dependencies: `ldd /usr/local/bin/hora-police`
- Verify binary architecture matches system: `uname -m` should match binary

### If service keeps restarting:
- Check StartLimitIntervalSec and StartLimitBurst in [Unit] section
- Review journal for crash reasons
- Verify config file is valid: `sudo /usr/local/bin/hora-police /etc/hora-police/config.toml --help`

