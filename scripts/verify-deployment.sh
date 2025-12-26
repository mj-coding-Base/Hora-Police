#!/usr/bin/env bash
# Hora-Police Deployment Verification Script
# Run after deployment to verify all components are working correctly

set -euo pipefail

echo "🔍 Verifying Hora-Police deployment..."
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track failures
FAILURES=0

# Function to check and report
check_status() {
    local name="$1"
    local command="$2"
    local expected="$3"
    
    if eval "$command" >/dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} $name"
        return 0
    else
        echo -e "${RED}✗${NC} $name (expected: $expected)"
        FAILURES=$((FAILURES + 1))
        return 1
    fi
}

# 1. Check systemd service status
echo "1. Systemd Service Status:"
if systemctl is-active --quiet hora-police; then
    echo -e "${GREEN}✓${NC} Service is active"
    systemctl show hora-police --property=MainPID --property=ActiveState --property=SubState --no-pager
else
    echo -e "${RED}✗${NC} Service is not active"
    FAILURES=$((FAILURES + 1))
fi
echo ""

# 2. Check resource limits
echo "2. Resource Limits:"
CPU_QUOTA=$(systemctl show hora-police --property=CPUQuota --value)
MEMORY_MAX=$(systemctl show hora-police --property=MemoryMax --value)
TASKS_MAX=$(systemctl show hora-police --property=TasksMax --value)

echo "  CPUQuota: $CPU_QUOTA"
echo "  MemoryMax: $MEMORY_MAX"
echo "  TasksMax: $TASKS_MAX"
echo ""

# 3. Verify WAL mode in database
echo "3. Database Configuration:"
if [ -f "/var/lib/hora-police/intelligence.db" ]; then
    JOURNAL_MODE=$(sqlite3 /var/lib/hora-police/intelligence.db "PRAGMA journal_mode;" 2>/dev/null || echo "unknown")
    if [ "$JOURNAL_MODE" = "wal" ]; then
        echo -e "${GREEN}✓${NC} Database is in WAL mode"
    else
        echo -e "${YELLOW}⚠${NC} Database journal mode: $JOURNAL_MODE (expected: wal)"
    fi
    
    # Check database integrity
    INTEGRITY=$(sqlite3 /var/lib/hora-police/intelligence.db "PRAGMA integrity_check;" 2>/dev/null | head -1)
    if [ "$INTEGRITY" = "ok" ]; then
        echo -e "${GREEN}✓${NC} Database integrity check passed"
    else
        echo -e "${RED}✗${NC} Database integrity check failed: $INTEGRITY"
        FAILURES=$((FAILURES + 1))
    fi
else
    echo -e "${YELLOW}⚠${NC} Database file not found (may be first run)"
fi
echo ""

# 4. Check inotify limits
echo "4. Inotify Configuration:"
MAX_WATCHES=$(sysctl -n fs.inotify.max_user_watches 2>/dev/null || echo "unknown")
if [ "$MAX_WATCHES" != "unknown" ] && [ "$MAX_WATCHES" -ge 524288 ]; then
    echo -e "${GREEN}✓${NC} Inotify max_user_watches: $MAX_WATCHES (>= 524288)"
else
    echo -e "${YELLOW}⚠${NC} Inotify max_user_watches: $MAX_WATCHES (recommended: >= 524288)"
fi
echo ""

# 5. Check zombie processes
echo "5. Zombie Process Check:"
ZOMBIE_COUNT=$(ps aux | awk '$8=="Z" {count++} END {print count+0}')
if [ "$ZOMBIE_COUNT" -eq 0 ]; then
    echo -e "${GREEN}✓${NC} No zombie processes detected"
elif [ "$ZOMBIE_COUNT" -lt 100 ]; then
    echo -e "${YELLOW}⚠${NC} $ZOMBIE_COUNT zombie processes detected (acceptable)"
else
    echo -e "${RED}✗${NC} $ZOMBIE_COUNT zombie processes detected (high count)"
    FAILURES=$((FAILURES + 1))
    
    # Show top zombie parents
    echo "  Top zombie parent PIDs:"
    ps -eo ppid,stat | awk '$2 ~ /Z/ {print $1}' | sort | uniq -c | sort -rn | head -5
fi
echo ""

# 6. Check binary and permissions
echo "6. Binary and Permissions:"
if [ -f "/usr/local/bin/hora-police" ]; then
    echo -e "${GREEN}✓${NC} Binary exists at /usr/local/bin/hora-police"
    BINARY_SIZE=$(stat -c%s /usr/local/bin/hora-police 2>/dev/null || echo "0")
    BINARY_SIZE_MB=$((BINARY_SIZE / 1024 / 1024))
    echo "  Binary size: ${BINARY_SIZE_MB}MB"
    
    if [ -x "/usr/local/bin/hora-police" ]; then
        echo -e "${GREEN}✓${NC} Binary is executable"
    else
        echo -e "${RED}✗${NC} Binary is not executable"
        FAILURES=$((FAILURES + 1))
    fi
else
    echo -e "${RED}✗${NC} Binary not found"
    FAILURES=$((FAILURES + 1))
fi
echo ""

# 7. Check directories
echo "7. Directory Structure:"
check_status "Quarantine directory exists" "[ -d /var/lib/hora-police/quarantine ]" "directory"
check_status "Rollbacks directory exists" "[ -d /var/lib/hora-police/rollbacks ]" "directory"
check_status "Config directory exists" "[ -d /etc/hora-police ]" "directory"
check_status "Keys directory exists" "[ -d /etc/hora-police/keys ]" "directory"
echo ""

# 8. Check configuration file
echo "8. Configuration:"
if [ -f "/etc/hora-police/config.toml" ]; then
    echo -e "${GREEN}✓${NC} Configuration file exists"
    # Check if it's valid TOML (basic check)
    if command -v toml-cli &>/dev/null; then
        if toml-cli validate /etc/hora-police/config.toml 2>/dev/null; then
            echo -e "${GREEN}✓${NC} Configuration file is valid TOML"
        else
            echo -e "${YELLOW}⚠${NC} Could not validate TOML (toml-cli not available)"
        fi
    fi
else
    echo -e "${RED}✗${NC} Configuration file not found"
    FAILURES=$((FAILURES + 1))
fi
echo ""

# 9. Check logs
echo "9. Logging:"
if journalctl -u hora-police --no-pager -n 1 >/dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Service logs accessible via journalctl"
    RECENT_ERRORS=$(journalctl -u hora-police --since "5 minutes ago" --no-pager | grep -i "error\|failed" | wc -l)
    if [ "$RECENT_ERRORS" -eq 0 ]; then
        echo -e "${GREEN}✓${NC} No recent errors in logs"
    else
        echo -e "${YELLOW}⚠${NC} $RECENT_ERRORS recent errors in logs (check with: journalctl -u hora-police -n 50)"
    fi
else
    echo -e "${YELLOW}⚠${NC} Could not access service logs"
fi
echo ""

# 10. Check systemd watchdog (if Type=notify)
echo "10. Systemd Watchdog:"
SERVICE_TYPE=$(systemctl show hora-police --property=Type --value)
if [ "$SERVICE_TYPE" = "notify" ]; then
    echo -e "${GREEN}✓${NC} Service type is 'notify' (watchdog enabled)"
    WATCHDOG_SEC=$(systemctl show hora-police --property=WatchdogUSec --value)
    if [ -n "$WATCHDOG_SEC" ] && [ "$WATCHDOG_SEC" != "0" ]; then
        WATCHDOG_SECONDS=$((WATCHDOG_SEC / 1000000))
        echo "  Watchdog timeout: ${WATCHDOG_SECONDS}s"
    fi
else
    echo -e "${YELLOW}⚠${NC} Service type is '$SERVICE_TYPE' (watchdog requires 'notify')"
fi
echo ""

# Summary
echo "=========================================="
if [ $FAILURES -eq 0 ]; then
    echo -e "${GREEN}✓ All checks passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ $FAILURES check(s) failed${NC}"
    echo ""
    echo "Troubleshooting:"
    echo "  - Check service status: sudo systemctl status hora-police"
    echo "  - View logs: sudo journalctl -u hora-police -f"
    echo "  - Verify config: sudo cat /etc/hora-police/config.toml"
    exit 1
fi

