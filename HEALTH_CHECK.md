# 🏥 Sentinel Health Check Guide

How to verify that Sentinel Anti-Malware Daemon is running correctly and monitoring your system.

## ✅ Quick Health Check (30 seconds)

Run these commands to quickly verify everything is working:

```bash
# 1. Check service status
sudo systemctl status sentinel

# 2. Check if process is running
ps aux | grep sentinel-daemon | grep -v grep

# 3. Check recent logs for startup messages
sudo journalctl -u sentinel -n 20 | grep -E "🚀|✅|🛡️"
```

**Expected Output:**
- Service should show: `Active: active (running)`
- Process should exist and be running as `root`
- Logs should show: `🚀 Sentinel Anti-Malware Daemon starting...`, `✅ Configuration loaded`, `🛡️ Sentinel daemon initialized`

---

## 🔍 Detailed Health Checks

### 1. Service Status Check

```bash
# Check if service is active and running
sudo systemctl status sentinel

# Check if service is enabled (starts on boot)
sudo systemctl is-enabled sentinel

# Check service uptime
systemctl show sentinel --property=ActiveEnterTimestamp
```

**✅ Healthy Signs:**
- `Active: active (running)`
- `Loaded: loaded`
- `Main PID:` shows a process ID
- No error messages in status output

**❌ Warning Signs:**
- `Active: inactive (dead)` - Service stopped
- `Active: failed` - Service crashed
- `Active: activating` - Service stuck starting

---

### 2. Log Verification

#### Check Startup Logs

```bash
# View last 50 log entries
sudo journalctl -u sentinel -n 50

# View logs since boot
sudo journalctl -u sentinel -b

# Follow logs in real-time
sudo journalctl -u sentinel -f
```

**✅ Healthy Log Messages:**
```
🚀 Sentinel Anti-Malware Daemon starting...
✅ Configuration loaded from: /etc/sentinel/config.toml
Initializing Sentinel daemon components...
✅ Database initialized at: /var/lib/sentinel/intelligence.db
🛡️ Sentinel daemon initialized. Starting monitoring...
🚀 Sentinel daemon running. Monitoring started.
```

**❌ Error Messages to Watch For:**
- `❌ Daemon error:` - Critical error, daemon stopped
- `Failed to get processes:` - Cannot access system processes
- `Failed to initialize database:` - Database issues
- `Failed to load configuration:` - Config file problems

#### Check for Ongoing Activity

```bash
# Check if daemon is actively monitoring (should see periodic activity)
sudo journalctl -u sentinel --since "5 minutes ago" | tail -20

# Check for any warnings or errors in last hour
sudo journalctl -u sentinel --since "1 hour ago" | grep -E "WARN|ERROR|⚠️|❌"
```

**✅ Healthy Signs:**
- Regular log entries (every 5 seconds based on polling interval)
- No ERROR messages
- Occasional INFO messages about process monitoring

---

### 3. Process Verification

```bash
# Check if daemon process exists
ps aux | grep sentinel-daemon | grep -v grep

# Check resource usage
top -p $(pgrep sentinel-daemon)

# Or use ps for detailed info
ps aux | grep sentinel-daemon | grep -v grep | awk '{print "CPU:", $3"%", "MEM:", $4"%", "PID:", $2}'
```

**✅ Healthy Resource Usage:**
- **CPU:** <1% average (should be very low)
- **Memory:** 30-40MB (should be stable)
- **Process:** Running as `root` user
- **PID:** Should be consistent (not restarting frequently)

**❌ Warning Signs:**
- CPU >5% - Unusually high
- Memory >100MB - Possible memory leak
- Process not found - Daemon crashed
- Multiple processes - Duplicate instances

---

### 4. Database Health Check

```bash
# Check if database file exists
ls -lh /var/lib/sentinel/intelligence.db

# Check database tables
sudo sqlite3 /var/lib/sentinel/intelligence.db ".tables"

# Check database size
du -h /var/lib/sentinel/intelligence.db
```

**✅ Expected Tables:**
- `process_history` - Historical process data
- `suspicious_processes` - Flagged processes
- `kill_actions` - Processes that were killed
- `cron_snapshots` - Cron job snapshots
- `npm_infections` - Detected npm threats

**✅ Healthy Database:**
- File exists and is readable
- Size grows slowly over time (10MB per 100k records)
- All 5 tables exist
- No permission errors

#### Query Recent Activity

```bash
# Check recent process monitoring (last hour)
sudo sqlite3 /var/lib/sentinel/intelligence.db \
  "SELECT COUNT(*) as recent_processes FROM process_history WHERE timestamp > datetime('now', '-1 hour');"

# Check suspicious processes detected
sudo sqlite3 /var/lib/sentinel/intelligence.db \
  "SELECT COUNT(*) as suspicious FROM suspicious_processes WHERE last_seen > datetime('now', '-24 hours');"

# Check kill actions (if any)
sudo sqlite3 /var/lib/sentinel/intelligence.db \
  "SELECT COUNT(*) as kills FROM kill_actions WHERE timestamp > datetime('now', '-24 hours');"

# View recent kill actions
sudo sqlite3 /var/lib/sentinel/intelligence.db \
  "SELECT pid, binary_path, reason, confidence, timestamp FROM kill_actions ORDER BY timestamp DESC LIMIT 10;"
```

**✅ Healthy Signs:**
- `recent_processes` > 0 (daemon is monitoring)
- Database queries execute without errors
- Data is being recorded (counts increase over time)

---

### 5. Performance Monitoring

```bash
# Monitor CPU and memory over time
watch -n 2 'ps aux | grep sentinel-daemon | grep -v grep'

# Check system impact
iostat -x 1 5 | grep -A 5 "Device"

# Check if daemon is causing any system load
top -b -n 1 | head -20
```

**✅ Performance Targets:**
- CPU: <1% average
- Memory: 30-40MB stable
- No I/O bottlenecks
- No system slowdown

---

### 6. Functionality Tests

#### Test CPU Abuse Detection

```bash
# Create a test CPU-hogging process
while true; do :; done &
TEST_PID=$!

echo "Test process started with PID: $TEST_PID"
echo "Waiting 5 minutes for detection..."

# Monitor logs
sudo journalctl -u sentinel -f &
JOURNAL_PID=$!

# Wait and check if process gets killed
sleep 300

# Check if process still exists
if ps -p $TEST_PID > /dev/null; then
    echo "⚠️ Process still running (may need more time or threshold adjustment)"
    kill $TEST_PID 2>/dev/null
else
    echo "✅ Process was killed by Sentinel!"
fi

# Stop log monitoring
kill $JOURNAL_PID 2>/dev/null

# Check kill action in database
sudo sqlite3 /var/lib/sentinel/intelligence.db \
  "SELECT * FROM kill_actions WHERE pid = $TEST_PID;"
```

**✅ Success Indicators:**
- Process detected in logs within 5 minutes
- Process killed automatically (if `auto_kill = true`)
- Kill action recorded in database
- Log shows: `🔪 Killing process PID=...`

#### Test Cron Monitoring

```bash
# Add a test cron entry (harmless)
echo "* * * * * echo 'test'" | sudo crontab -

# Wait 5 minutes for cron scan
sleep 300

# Check logs for cron detection
sudo journalctl -u sentinel --since "5 minutes ago" | grep -i cron

# Check database for cron snapshots
sudo sqlite3 /var/lib/sentinel/intelligence.db \
  "SELECT * FROM cron_snapshots ORDER BY detected_at DESC LIMIT 5;"

# Remove test cron
sudo crontab -r
```

**✅ Success Indicators:**
- Cron job detected in logs
- Cron snapshot recorded in database
- No false positives for normal cron jobs

---

### 7. Telegram Integration Check (if configured)

```bash
# Check if Telegram bot token is configured
sudo grep -A 3 "\[telegram\]" /etc/sentinel/config.toml

# Check logs for Telegram connection
sudo journalctl -u sentinel | grep -i telegram

# Manually trigger a test (if Telegram module supports it)
# This would require checking the telegram.rs implementation
```

**✅ Healthy Signs:**
- Bot token configured (not "YOUR_BOT_TOKEN_HERE")
- No Telegram connection errors in logs
- Daily reports scheduled (check logs for "daily report")

---

## 📊 Health Check Script

Create a quick health check script:

```bash
#!/bin/bash
# save as: check_sentinel_health.sh

echo "🛡️ Sentinel Health Check"
echo "========================"
echo ""

# Service status
echo "1. Service Status:"
if systemctl is-active --quiet sentinel; then
    echo "   ✅ Service is running"
else
    echo "   ❌ Service is NOT running"
fi

# Process check
echo ""
echo "2. Process Check:"
if pgrep -x "sentinel-daemon" > /dev/null; then
    PID=$(pgrep -x "sentinel-daemon")
    CPU=$(ps -p $PID -o %cpu --no-headers | tr -d ' ')
    MEM=$(ps -p $PID -o %mem --no-headers | tr -d ' ')
    echo "   ✅ Process running (PID: $PID, CPU: ${CPU}%, MEM: ${MEM}%)"
    
    if (( $(echo "$CPU > 5.0" | bc -l) )); then
        echo "   ⚠️  CPU usage is high (>5%)"
    fi
else
    echo "   ❌ Process not found"
fi

# Database check
echo ""
echo "3. Database Check:"
if [ -f "/var/lib/sentinel/intelligence.db" ]; then
    SIZE=$(du -h /var/lib/sentinel/intelligence.db | cut -f1)
    echo "   ✅ Database exists (Size: $SIZE)"
    
    TABLE_COUNT=$(sudo sqlite3 /var/lib/sentinel/intelligence.db ".tables" | wc -w)
    if [ "$TABLE_COUNT" -eq 5 ]; then
        echo "   ✅ All tables present ($TABLE_COUNT tables)"
    else
        echo "   ⚠️  Expected 5 tables, found $TABLE_COUNT"
    fi
else
    echo "   ❌ Database file not found"
fi

# Recent logs check
echo ""
echo "4. Recent Activity:"
RECENT_LOGS=$(sudo journalctl -u sentinel --since "10 minutes ago" --no-pager | wc -l)
if [ "$RECENT_LOGS" -gt 0 ]; then
    echo "   ✅ Recent log activity ($RECENT_LOGS entries in last 10 min)"
else
    echo "   ⚠️  No recent log activity"
fi

# Error check
echo ""
echo "5. Error Check:"
ERRORS=$(sudo journalctl -u sentinel --since "1 hour ago" --no-pager | grep -c "ERROR\|❌" || echo "0")
if [ "$ERRORS" -eq 0 ]; then
    echo "   ✅ No errors in last hour"
else
    echo "   ⚠️  Found $ERRORS errors in last hour"
    echo "   Run: sudo journalctl -u sentinel --since '1 hour ago' | grep ERROR"
fi

echo ""
echo "========================"
echo "Health check complete!"
```

Make it executable and run:
```bash
chmod +x check_sentinel_health.sh
./check_sentinel_health.sh
```

---

## 🚨 Troubleshooting Common Issues

### Issue: Service shows as "failed"

```bash
# Check detailed error
sudo journalctl -u sentinel -n 50

# Common fixes:
# 1. Check config file syntax
sudo sentinel-daemon /etc/sentinel/config.toml

# 2. Check permissions
sudo ls -la /var/lib/sentinel/
sudo chown root:root /var/lib/sentinel/intelligence.db

# 3. Restart service
sudo systemctl restart sentinel
```

### Issue: No process monitoring

```bash
# Check if daemon has proper permissions
ps aux | grep sentinel-daemon | grep root

# Check logs for permission errors
sudo journalctl -u sentinel | grep -i "permission\|denied\|access"

# Verify daemon is running as root
sudo systemctl show sentinel --property=User
```

### Issue: High CPU usage

```bash
# Check what's causing high CPU
sudo strace -p $(pgrep sentinel-daemon) -c

# Increase polling interval in config
sudo nano /etc/sentinel/config.toml
# Change: polling_interval_ms = 10000  # 10 seconds instead of 5

sudo systemctl restart sentinel
```

### Issue: Database errors

```bash
# Check database permissions
sudo ls -la /var/lib/sentinel/intelligence.db

# Fix permissions if needed
sudo chown root:root /var/lib/sentinel/intelligence.db
sudo chmod 644 /var/lib/sentinel/intelligence.db

# Check database integrity
sudo sqlite3 /var/lib/sentinel/intelligence.db "PRAGMA integrity_check;"
```

---

## 📈 Regular Monitoring Schedule

**Daily:**
- Quick status check: `sudo systemctl status sentinel`
- Check for errors: `sudo journalctl -u sentinel --since "24 hours ago" | grep ERROR`

**Weekly:**
- Full health check using script above
- Review kill actions: `sudo sqlite3 /var/lib/sentinel/intelligence.db "SELECT * FROM kill_actions WHERE timestamp > datetime('now', '-7 days');"`
- Check database size growth

**Monthly:**
- Performance review (CPU, memory trends)
- Review suspicious processes detected
- Clean old database records if needed

---

## ✅ Summary: Healthy System Checklist

- [ ] Service is `active (running)`
- [ ] Process exists and running as `root`
- [ ] CPU usage <1%
- [ ] Memory usage 30-40MB
- [ ] Database file exists and accessible
- [ ] All 5 database tables present
- [ ] Recent log activity (entries every 5 seconds)
- [ ] No ERROR messages in logs
- [ ] Startup messages present in logs
- [ ] Database recording process data
- [ ] Detection tests work correctly

If all checks pass, your Sentinel daemon is running correctly! 🎉

