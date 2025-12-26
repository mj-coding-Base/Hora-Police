# 🏗️ Sentinel Architecture Documentation

## System Overview

Sentinel is a modular, high-performance anti-malware daemon built in Rust. It operates as a continuous monitoring system with minimal overhead, designed specifically for multi-user Ubuntu VPS environments.

## Core Components

### 1. Process Monitor (`process_monitor.rs`)

**Purpose**: Enumerate and track all running processes on the system.

**Key Features**:
- Uses `sysinfo` crate for efficient process enumeration
- Tracks: PID, PPID, UID, binary path, command line, CPU usage
- Maintains process tree relationships
- Implements safe binary whitelist

**API**:
- `get_all_processes()` - Returns all current processes
- `get_process_by_pid(pid)` - Get specific process info
- `get_process_tree(pid)` - Get parent process chain
- `is_safe_binary(path)` - Check if binary is whitelisted

### 2. CPU Analyzer (`cpu_analyzer.rs`)

**Purpose**: Detect processes exceeding CPU thresholds for sustained periods.

**Algorithm**:
1. Track processes with CPU > threshold
2. Record first-seen timestamp
3. Calculate duration of high CPU usage
4. Flag when duration exceeds configured limit

**Configuration**:
- `cpu_threshold`: Default 20%
- `duration_minutes`: Default 5 minutes

### 3. Cron Watcher (`cron_watcher.rs`)

**Purpose**: Monitor cron jobs for suspicious patterns and persistence mechanisms.

**Monitored Locations**:
- `/etc/crontab`
- `/etc/cron.d/*`
- `/etc/cron.{hourly,daily,weekly,monthly}/*`
- `/var/spool/cron/crontabs/*`

**Detection Patterns**:
- Base64 encoded commands
- `curl | wget | bash` patterns
- `npm install` at runtime
- Obfuscated variable expansion
- Suspicious URL patterns

**Storage**: All cron snapshots stored with SHA256 hashes for change detection.

### 4. npm Scanner (`npm_scanner.rs`)

**Purpose**: Detect supply-chain attacks via malicious npm packages.

**Detection Methods**:
1. **Known Malicious Packages**: Blacklist of known miners
2. **Suspicious Script Names**: Patterns like "miner", "crypto", "hash"
3. **Post-Install Scripts**: Common attack vector
4. **Native Binaries**: `.node` files in node_modules

**Scanning Process**:
1. Identify Node.js processes
2. Extract working directory from command line
3. Parse `package.json` and `package-lock.json`
4. Walk `node_modules` for suspicious packages
5. Calculate threat level (0.0-1.0)

### 5. React Abuse Detector (`react_detector.rs`)

**Purpose**: Heuristic detection of crypto miners hidden in React Flight protocol handlers.

**Heuristics**:
1. High CPU in React server processes (>15%)
2. Sustained high CPU (>20%)
3. Crypto-related code in command line
4. Dynamic code execution (`eval`, `Function()`)
5. Long-running deserialization loops (inferred)

**Confidence Scoring**: Combines multiple signals for threat assessment.

### 6. Behavior Intelligence (`intelligence.rs`)

**Purpose**: Learn from past actions and build threat profiles.

**Learning Mechanisms**:
- **Repeat Behavior**: Processes seen before get higher confidence
- **Restart Detection**: Processes that respawn after kill = higher threat
- **Spawn Count**: High spawn frequency = suspicious
- **Location Analysis**: Processes from `/tmp`, `/var/tmp` = higher threat
- **Command Line Analysis**: Keywords like "miner", "xmrig" = higher threat

**Database Integration**: Stores and retrieves historical threat data.

### 7. Kill Engine (`kill_engine.rs`)

**Purpose**: Safely terminate malicious processes with escalation handling.

**Kill Process**:
1. Send SIGTERM (graceful)
2. Wait 2 seconds
3. If still alive, send SIGKILL
4. Check for respawn after 5 seconds
5. If respawned, escalate to parent process

**Safety Features**:
- Never kills system processes (whitelist)
- Records all actions to database
- Logs all kills with reason and confidence

### 8. Intelligence Database (`database.rs`)

**Purpose**: Persistent storage for all intelligence and actions.

**Schema**:

```sql
-- Process history (sampled, high CPU only)
process_history (
    id, pid, ppid, uid, binary_path, command_line,
    cpu_percent, timestamp
)

-- Suspicious processes (with threat scoring)
suspicious_processes (
    id, pid, ppid, uid, binary_path, command_line,
    cpu_percent, duration_seconds, threat_confidence,
    first_seen, last_seen, spawn_count, restart_detected
)

-- Cron job snapshots
cron_snapshots (
    id, file_path, content_hash, content, user,
    detected_at, suspicious
)

-- npm infections
npm_infections (
    id, package_name, version, install_scripts,
    binary_path, detected_at, threat_level
)

-- Kill actions (forensic log)
kill_actions (
    id, pid, uid, binary_path, reason,
    confidence, timestamp
)
```

**Indexes**: Optimized for time-range queries and binary lookups.

### 9. Telegram Reporter (`telegram.rs`)

**Purpose**: Send daily summaries and optional real-time alerts.

**Features**:
- Daily scheduled reports
- Real-time alerts (optional)
- Formatted Markdown messages
- Error handling and retry logic

**Message Format**:
- Summary statistics
- Recent kill actions
- Threat details with confidence scores

### 10. Main Daemon (`daemon.rs`)

**Purpose**: Orchestrate all components in async event loop.

**Event Loop**:
1. Refresh process list (every 5 seconds)
2. Record high-CPU processes to database
3. Analyze CPU usage for abuses
4. For each abuse:
   - Calculate threat confidence
   - Scan for npm infections
   - Check for React abuse
   - Kill if threshold exceeded
5. Periodically scan cron jobs (every 5 minutes)
6. Sleep and repeat

**Async Architecture**:
- Uses Tokio for async runtime
- Spawns separate task for daily reports
- Non-blocking database operations
- Efficient polling with adaptive intervals

## Data Flow

```
Process Monitor
    ↓
CPU Analyzer → Detects Abuse
    ↓
Behavior Intelligence → Calculates Confidence
    ↓
npm Scanner → Checks for Supply-Chain
    ↓
React Detector → Checks for React Abuse
    ↓
Kill Engine → Terminates if Threshold Exceeded
    ↓
Database → Records All Actions
    ↓
Telegram Reporter → Sends Reports
```

## Performance Optimizations

1. **Sampled Recording**: Only record processes with CPU > 1%
2. **Indexed Queries**: All database queries use indexes
3. **Async I/O**: Non-blocking database and network operations
4. **Adaptive Polling**: Configurable intervals (default 5s)
5. **Memory Efficiency**: Bounded collections, no unbounded growth
6. **CPU Efficiency**: No busy loops, proper sleep intervals

## Security Considerations

1. **Privilege Model**: Runs as root (required for process monitoring)
2. **File Access**: Hardened with systemd security options
3. **Network**: Only outbound HTTPS to Telegram API
4. **Input Sanitization**: All file reads are bounded
5. **Kill Safety**: Whitelist prevents killing system processes
6. **Logging**: Tamper-resistant SQLite database

## Configuration System

Single TOML file with:
- CPU thresholds
- Duration limits
- Telegram settings
- Database path
- Feature flags (auto_kill, learning_mode, etc.)

## Error Handling

- All operations use `Result<T>` types
- Errors logged but don't crash daemon
- Database errors are non-fatal
- Network errors retry automatically
- Process kill failures are logged

## Testing Strategy

1. **Unit Tests**: Each module independently testable
2. **Integration Tests**: Full daemon with mock processes
3. **Simulation**: Test scripts for CPU abuse, npm infections
4. **Performance Tests**: Verify overhead targets

## Future Enhancements

1. **eBPF Integration**: Kernel-level hooks for better visibility
2. **ML Threat Scoring**: Machine learning for confidence calculation
3. **Web Dashboard**: Real-time monitoring interface
4. **Prometheus Metrics**: Export metrics for monitoring
5. **Docker Support**: Container-aware process monitoring

## Dependencies

- **tokio**: Async runtime
- **sysinfo**: Process enumeration
- **sqlx**: Database operations
- **reqwest**: HTTP client (Telegram)
- **serde**: Configuration parsing
- **nix**: Process signals
- **regex**: Pattern matching
- **chrono**: Time handling

## Build Configuration

- **Release Profile**: Optimized for performance
- **LTO**: Link-time optimization enabled
- **Strip**: Binary stripped for size
- **Target**: Linux x86_64 (Ubuntu)

---

**Last Updated**: 2024
**Version**: 0.1.0

