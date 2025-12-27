# Compile Fixes Applied - Summary

## Code Patches

### 1. `src/file_quarantine.rs` - Monitor Mutability Fix

**Error**: `error[E0596]: cannot borrow monitor as mutable`

**Change**:
```diff
-        let monitor = ProcessMonitor::new();
+        let mut monitor = ProcessMonitor::new();
         monitor.refresh();
```

**Location**: Line 94 in `kill_processes_using_file()` method

---

### 2. `src/process_monitor.rs` - Uid API Fix

**Error**: `error[E0599]: no method named as_raw found for reference &Uid`

**Changes**:

1. Added helper function and import:
```rust
use sysinfo::{Pid, System, Process, User, Uid};

/// Helper function to convert sysinfo Uid to u32
/// sysinfo 0.30+ uses .as_() instead of .as_raw()
/// See: https://docs.rs/sysinfo/latest/sysinfo/struct.Uid.html
fn uid_to_u32(uid_opt: Option<&Uid>) -> u32 {
    uid_opt.map(|u| u.as_()).unwrap_or(0u32)
}
```

2. Replaced `as_raw()` calls:
```diff
-            let uid = process.user_id().map(|u| u.as_raw()).unwrap_or(0u32);
+            let uid = uid_to_u32(process.user_id());
```

**Locations**: Lines 61 and 102

---

### 3. `src/kill_engine.rs` - Recursive Async Fix

**Error**: `error[E0733]: recursion in an async fn requires boxing`

**Change**: Replaced recursive await with `tokio::spawn`:

```diff
-                if respawned.ppid > 0 {
-                    let _ = self.kill_process(
-                        respawned.ppid,
-                        respawned.uid,
-                        &format!("Parent of respawned process: {}", binary_path),
-                        "Process respawn detected",
-                        confidence + 0.1,
-                    ).await;
-                }
+                if respawned.ppid > 0 {
+                    let parent_pid = respawned.ppid;
+                    let parent_uid = respawned.uid;
+                    let parent_binary = format!("Parent of respawned process: {}", binary_path);
+                    let escalation_reason = "Process respawn detected".to_string();
+                    let escalation_confidence = confidence + 0.1;
+                    
+                    // Clone necessary data for spawned task
+                    let db_clone = self.db.clone();
+                    let monitor_clone = self.monitor.clone();
+                    let auto_kill = self.auto_kill;
+                    let threshold = self.threshold;
+                    
+                    // Spawn escalation as detached task (don't await to avoid infinite future size)
+                    tokio::spawn(async move {
+                        let mut escalation_engine = KillEngine {
+                            db: db_clone,
+                            monitor: monitor_clone,
+                            auto_kill,
+                            threshold,
+                        };
+                        let _ = escalation_engine.kill_process(
+                            parent_pid,
+                            parent_uid,
+                            &parent_binary,
+                            &escalation_reason,
+                            escalation_confidence,
+                        ).await;
+                    });
+                }
```

**Location**: Lines 98-106 in `kill_process()` method

---

## Build Script Updates

### 4. `build-lowmem.sh` - Low Memory Build Profile

**Updated to use**:
- `RUSTFLAGS="-C opt-level=2 -C codegen-units=1"`
- `-j1` (single job)
- `--locked` (deterministic builds)
- LTO disabled (via profile)

---

### 5. `scripts/install-binary.sh` - Safe Deployment Script

**New script** that:
1. Stops service
2. Copies `/tmp/hora-police` to `/usr/local/bin/hora-police`
3. Sets ownership `root:root` and permissions `755`
4. Ensures all required directories exist
5. Installs tmpfiles.d configuration
6. Reloads systemd
7. Starts service and verifies

---

## Exact Commands to Run on VPS

### Step 1: Pull Latest Changes
```bash
cd /srv/Hora-Police
git pull
```

### Step 2: Build with Low Memory Profile
```bash
chmod +x build-lowmem.sh
./build-lowmem.sh
```

**Expected output**: Binary at `target/release/hora-police`

### Step 3: Install Binary
```bash
# Copy to /tmp
cp target/release/hora-police /tmp/hora-police

# Install
chmod +x scripts/install-binary.sh
./scripts/install-binary.sh
```

### Step 4: Verify
```bash
# Service status
sudo systemctl status hora-police --no-pager

# Check for errors
sudo journalctl -u hora-police -n 50 --no-pager | grep -iE 'EXEC|error' || echo "No errors"

# Verify binary
file /usr/local/bin/hora-police
ldd /usr/local/bin/hora-police

# Verify directories
ls -la /etc/hora-police /var/lib/hora-police /var/log/hora-police
```

---

## Verification Checklist

Run these commands to verify success:

```bash
# Binary exists and is executable
test -f /usr/local/bin/hora-police && echo "✅ Binary exists" || echo "❌ Binary missing"
test -x /usr/local/bin/hora-police && echo "✅ Binary executable" || echo "❌ Binary not executable"

# Service is active
systemctl is-active --quiet hora-police && echo "✅ Service active" || echo "❌ Service not active"

# No EXEC errors in journal
journalctl -u hora-police -n 50 | grep -qi "EXEC" && echo "❌ EXEC errors found" || echo "✅ No EXEC errors"

# Directories exist
test -d /etc/hora-police && test -d /var/lib/hora-police && test -d /var/log/hora-police && echo "✅ Directories exist" || echo "❌ Directories missing"

# tmpfiles.d installed
test -f /etc/tmpfiles.d/hora-police.conf && echo "✅ tmpfiles.d installed" || echo "❌ tmpfiles.d missing"
```

---

## File Permissions Expected

```bash
/usr/local/bin/hora-police          # 755, root:root
/etc/hora-police                    # 755, root:root
/var/lib/hora-police                # 755, root:root
/var/lib/hora-police/quarantine     # 700, root:root
/var/log/hora-police                # 755, root:root
/etc/tmpfiles.d/hora-police.conf    # 644, root:root
```

---

## If Build Fails with OOM

Add swap space:
```bash
sudo fallocate -l 2G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

Then retry build.

