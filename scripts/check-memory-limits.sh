#!/bin/bash
# Script to diagnose memory limits and OOM issues

echo "=== Memory Limit Diagnostics ==="
echo ""

echo "1. System Memory:"
free -h
echo ""

echo "2. User Ulimits:"
ulimit -a
echo ""

echo "3. Virtual Memory Overcommit Setting:"
cat /proc/sys/vm/overcommit_memory
echo "  (0 = heuristic, 1 = always, 2 = never)"
echo ""

echo "4. Overcommit Ratio:"
cat /proc/sys/vm/overcommit_ratio
echo ""

echo "5. OOM Killer Logs (last 20):"
dmesg | grep -i oom | tail -20 || echo "No OOM entries found"
echo ""

echo "6. Current Memory Usage by Process (top 10):"
ps aux --sort=-%mem | head -11
echo ""

echo "7. Zombie Process Count:"
ps aux | awk '$8=="Z" {count++} END {print "Zombies:", count+0}'
echo ""

echo "8. Cgroup Memory Limits (if applicable):"
if [ -f /sys/fs/cgroup/memory/memory.limit_in_bytes ]; then
    echo "Memory limit: $(cat /sys/fs/cgroup/memory/memory.limit_in_bytes)"
    echo "Memory usage: $(cat /sys/fs/cgroup/memory/memory.usage_in_bytes)"
fi
echo ""

echo "9. Available Memory for Build:"
AVAIL=$(free -m | awk 'NR==2{printf "%.0f", $7}')
echo "Available: ${AVAIL}MB"
echo ""

echo "10. Rust/Cargo Process Memory (if running):"
ps aux | grep -E "rustc|cargo" | grep -v grep || echo "No Rust processes running"
echo ""

echo "=== Recommendations ==="
if [ "$AVAIL" -lt 2048 ]; then
    echo "⚠️  Low available memory (<2GB). Consider:"
    echo "   - Building debug version first"
    echo "   - Building on different machine"
    echo "   - Adding more swap"
fi

if dmesg | grep -qi oom | tail -1; then
    echo "⚠️  OOM killer has been active. Check dmesg output above."
fi

