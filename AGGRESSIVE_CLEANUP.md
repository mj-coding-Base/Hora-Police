# 🧹 Aggressive Malware Origin Cleanup

Hora-Police now includes **aggressive cleanup mode** that deletes malware origins with administrative authority.

## 🎯 What Gets Deleted

When `aggressive_cleanup = true`, Hora-Police will:

1. **Delete Parent Directories**: If a directory contains only malicious files, the entire directory is removed
2. **Delete Related Files**: Other suspicious files in the same directory are also deleted
3. **Clean Cron Jobs**: Removes cron entries that reference the malware file
4. **Kill Processes**: Terminates all processes using or referencing the malware

## ⚙️ Configuration

Add to your `config.toml`:

```toml
[file_scanning]
enabled = true
aggressive_cleanup = true  # Enable aggressive origin deletion
auto_delete = false         # Delete malware files (not just quarantine)
kill_processes_using_file = true
```

## 🔒 Administrative Authority

Hora-Police runs as `root` and has full administrative authority to:
- Delete files and directories
- Modify cron jobs
- Kill processes
- Remove entire directory trees

**⚠️ WARNING**: Aggressive cleanup permanently deletes files and directories. Use with caution!

## 📋 Cleanup Process

When malware is detected:

1. **Kill Processes**: All processes using the malware file are terminated
2. **Delete Related Files**: Other suspicious files in the same directory are removed
3. **Delete Parent Directory**: If directory contains only malware, entire directory is deleted
4. **Clean Cron Jobs**: Cron entries referencing the malware are removed
5. **Delete/Quarantine Malware**: The malware file itself is deleted or quarantined

## 🔍 Example Cleanup

If malware is found at `/home/deploy/tilak-traders/solrz`:

```
🧹 Origin Cleanup:
- Deleted 3 related files (solrz, e386, payload.so)
- Removed 1 directory (/home/deploy/tilak-traders)
- Cleaned 2 cron jobs
```

## 📊 Database Tracking

All cleanup actions are logged:

```sql
-- View cleanup history
SELECT * FROM malware_files 
WHERE action_taken = 'deleted' 
ORDER BY detected_at DESC;
```

## 🛡️ Safety Features

- **Directory Check**: Only deletes directories that contain ONLY suspicious files
- **Legitimate File Protection**: Directories with legitimate files are preserved
- **Logging**: All deletions are logged to the database
- **Telegram Alerts**: Real-time notifications of cleanup actions

## ⚠️ Important Notes

1. **Irreversible**: Deleted files cannot be recovered
2. **Root Access Required**: Must run as root for full cleanup authority
3. **False Positives**: Review signatures to avoid deleting legitimate files
4. **Backup**: Consider backing up important directories before enabling

## 🧪 Testing

Test aggressive cleanup:

```bash
# Create test malware directory
mkdir -p /tmp/test-malware
echo "malware" > /tmp/test-malware/solrz
echo "malware" > /tmp/test-malware/e386
chmod +x /tmp/test-malware/*

# Wait for scan (or trigger manually)
# Check logs
sudo journalctl -u hora-police -f

# Verify cleanup
ls -la /tmp/test-malware  # Should be deleted
```

## 📝 Log Messages

Watch for these log messages:

- `🧹 Cleaned malware origin: X files, Y dirs, Z cron jobs` - Cleanup summary
- `🗑️  Deleting suspicious parent directory: ...` - Directory deletion
- `🗑️  Deleting related suspicious file: ...` - Related file deletion
- `🗑️  Removing cron job referencing malware: ...` - Cron cleanup

---

**Aggressive cleanup ensures malware origins are completely eliminated!** 🛡️

