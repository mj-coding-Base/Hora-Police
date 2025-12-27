# Changelog: Compile Fixes and Low-Memory Build Support

## Summary

Fixed 3 compile errors and added low-memory build support for VPS deployment.

## Code Fixes

### 1. Fixed Monitor Mutability (`src/file_quarantine.rs`)
- **Issue**: `error[E0596]: cannot borrow monitor as mutable`
- **Fix**: Changed `let monitor = ProcessMonitor::new();` to `let mut monitor = ProcessMonitor::new();`
- **Line**: 94

### 2. Fixed Uid API Compatibility (`src/process_monitor.rs`)
- **Issue**: `error[E0599]: no method named as_raw found for reference &Uid`
- **Fix**: 
  - Added `uid_to_u32()` helper function using `Uid.as_()` (sysinfo 0.30+ API)
  - Replaced `u.as_raw()` calls with `uid_to_u32(process.user_id())`
- **Lines**: 61, 102
- **Note**: sysinfo 0.30+ changed API from `as_raw()` to `as_()`

### 3. Fixed Recursive Async Function (`src/kill_engine.rs`)
- **Issue**: `error[E0733]: recursion in an async fn requires boxing`
- **Fix**: Replaced recursive `self.kill_process().await` with `tokio::spawn` to avoid infinite future size
- **Line**: 98-106
- **Approach**: Extract data, clone necessary structs, spawn detached task

## Build Scripts

### 4. Updated `build-lowmem.sh`
- Uses `RUSTFLAGS="-C opt-level=2 -C codegen-units=1"`
- Single job build (`-j1`) to reduce memory usage
- `--locked` for deterministic builds
- LTO disabled (via profile)

### 5. Created `scripts/install-binary.sh`
- Safe deployment script
- Stops service, installs binary, ensures directories, reloads systemd, starts service
- Includes error checking and verification

## Files Changed

```
M  src/file_quarantine.rs
M  src/process_monitor.rs
M  src/kill_engine.rs
M  build-lowmem.sh
A  scripts/install-binary.sh
A  DEPLOYMENT_GUIDE.md
A  FIXES_APPLIED.md
A  VPS_BUILD_COMMANDS.md
```

## Deployment

See `VPS_BUILD_COMMANDS.md` for exact commands to run on VPS.

Quick sequence:
1. `git pull`
2. `./build-lowmem.sh`
3. `cp target/release/hora-police /tmp/hora-police`
4. `./scripts/install-binary.sh`

## Verification

After deployment, verify:
- Binary exists and is executable
- Service is active
- No EXEC/NAMESPACE errors in journal
- All directories exist with correct permissions

See `DEPLOYMENT_GUIDE.md` for complete verification checklist.

