#!/usr/bin/env bash
# Diagnostic script for hora-police binary
# Checks binary existence, permissions, architecture, dependencies, and executability

set -euo pipefail

BINARY="/usr/local/bin/hora-police"
EXIT_CODE=0

echo "=== Hora-Police Binary Diagnostic ==="
echo "Binary path: ${BINARY}"
echo ""

# 1. Check if binary exists
echo "[1/7] Checking binary existence..."
if [ -f "${BINARY}" ]; then
    echo "✅ Binary exists"
else
    echo "❌ Binary NOT FOUND at ${BINARY}"
    echo ""
    echo "REMEDIATION: Copy binary from WSL:"
    echo "  cd /mnt/f/Personal_Projects/Hora-Police"
    echo "  cargo build --release"
    echo "  scp target/release/hora-police deploy@62.72.13.136:/tmp/"
    echo "  ssh deploy@62.72.13.136 'sudo mv /tmp/hora-police ${BINARY} && sudo chmod +x ${BINARY}'"
    EXIT_CODE=1
    exit $EXIT_CODE
fi

# 2. Check file permissions
echo ""
echo "[2/7] Checking file permissions..."
PERMS=$(stat -c "%a" "${BINARY}" 2>/dev/null || stat -f "%OLp" "${BINARY}" 2>/dev/null || echo "unknown")
ls -l "${BINARY}"
if [ "${PERMS}" = "755" ] || [ "${PERMS}" = "755" ]; then
    echo "✅ Permissions correct (${PERMS})"
elif [ -x "${BINARY}" ]; then
    echo "⚠️  Permissions are ${PERMS} but file is executable"
else
    echo "❌ Binary is NOT executable (permissions: ${PERMS})"
    echo ""
    echo "REMEDIATION:"
    echo "  sudo chmod +x ${BINARY}"
    EXIT_CODE=1
fi

# 3. Check file type and architecture
echo ""
echo "[3/7] Checking file type and architecture..."
FILE_INFO=$(file "${BINARY}" 2>&1 || echo "file command failed")
echo "${FILE_INFO}"

# Check if it's an ELF binary
if echo "${FILE_INFO}" | grep -q "ELF"; then
    echo "✅ Valid ELF binary"
    
    # Extract architecture
    if echo "${FILE_INFO}" | grep -q "x86-64"; then
        echo "✅ Architecture: x86-64 (correct for this system)"
    elif echo "${FILE_INFO}" | grep -q "ARM"; then
        echo "❌ Architecture: ARM (WRONG - system is x86-64)"
        echo ""
        echo "REMEDIATION: Rebuild for x86-64:"
        echo "  cargo build --release --target x86_64-unknown-linux-gnu"
        EXIT_CODE=1
    else
        echo "⚠️  Architecture check: Could not determine from file output"
    fi
else
    echo "❌ NOT a valid ELF binary"
    echo ""
    echo "REMEDIATION: Binary may be corrupted. Rebuild and copy again."
    EXIT_CODE=1
fi

# 4. Check system architecture
echo ""
echo "[4/7] Checking system architecture..."
SYSTEM_ARCH=$(uname -m)
echo "System architecture: ${SYSTEM_ARCH}"
if [ "${SYSTEM_ARCH}" = "x86_64" ]; then
    echo "✅ System is x86_64"
else
    echo "⚠️  System architecture: ${SYSTEM_ARCH}"
fi

# 5. Check shared library dependencies
echo ""
echo "[5/7] Checking shared library dependencies..."
if ldd "${BINARY}" >/dev/null 2>&1; then
    echo "Dependencies:"
    ldd "${BINARY}" || true
    
    # Check for missing dependencies
    MISSING=$(ldd "${BINARY}" 2>&1 | grep "not found" || true)
    if [ -n "${MISSING}" ]; then
        echo ""
        echo "❌ Missing dependencies detected:"
        echo "${MISSING}"
        echo ""
        echo "REMEDIATION: Install missing libraries or use statically linked binary"
        EXIT_CODE=1
    else
        echo "✅ All dependencies available"
    fi
else
    echo "⚠️  ldd failed (may be statically linked or corrupted)"
    # Try to get more info
    if readelf -d "${BINARY}" >/dev/null 2>&1; then
        echo "Binary appears to be dynamically linked but ldd failed"
        EXIT_CODE=1
    else
        echo "Binary may be statically linked (this is OK)"
    fi
fi

# 6. Check SELinux context (if applicable)
echo ""
echo "[6/7] Checking SELinux context..."
if command -v getenforce >/dev/null 2>&1; then
    if [ "$(getenforce)" != "Disabled" ]; then
        CONTEXT=$(stat -c "%C" "${BINARY}" 2>/dev/null || echo "unknown")
        echo "SELinux context: ${CONTEXT}"
        if echo "${CONTEXT}" | grep -q "exec_t\|bin_t"; then
            echo "✅ SELinux context looks correct"
        else
            echo "⚠️  SELinux context may need adjustment"
            echo "REMEDIATION: sudo restorecon ${BINARY}"
        fi
    else
        echo "SELinux is disabled (OK)"
    fi
else
    echo "SELinux not available (OK)"
fi

# 7. Test binary execution
echo ""
echo "[7/7] Testing binary execution..."
if sudo "${BINARY}" --help >/dev/null 2>&1; then
    echo "✅ Binary executes successfully (--help works)"
elif sudo "${BINARY}" --version >/dev/null 2>&1; then
    echo "✅ Binary executes successfully (--version works)"
else
    echo "❌ Binary execution FAILED"
    echo ""
    echo "Attempting to run binary to see error:"
    sudo "${BINARY}" 2>&1 | head -5 || true
    echo ""
    echo "REMEDIATION: Binary may be corrupted or have missing dependencies"
    EXIT_CODE=1
fi

# Summary
echo ""
echo "=== Diagnostic Summary ==="
if [ $EXIT_CODE -eq 0 ]; then
    echo "✅ All checks passed - binary appears to be valid"
    echo ""
    echo "If service still fails with 203/EXEC, check:"
    echo "  1. Service unit file ExecStart path is correct"
    echo "  2. User running service has execute permission"
    echo "  3. No AppArmor/SELinux restrictions"
    echo "  4. Filesystem is not mounted noexec"
else
    echo "❌ Issues found - see remediation steps above"
fi

exit $EXIT_CODE

