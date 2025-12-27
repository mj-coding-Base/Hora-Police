#!/usr/bin/env bash
set -euo pipefail
REPO_DIR="/srv/Hora-Police"
SERVICE_UNIT="/etc/systemd/system/hora-police.service"
TMPFILE="/etc/tmpfiles.d/hora-police.conf"
BINARY="/usr/local/bin/hora-police"
CONFIG="/etc/hora-police/config.toml"

echo "== Hora-Police fix script starting at $(date -u) =="

# 1. Stop service to avoid restart loops
sudo systemctl stop hora-police || true

# 2. Create required directories
sudo mkdir -p /etc/hora-police
sudo mkdir -p /var/lib/hora-police/quarantine
sudo mkdir -p /var/log/hora-police

# 3. Set safe ownership and permissions
sudo chown -R root:root /etc/hora-police /var/lib/hora-police /var/log/hora-police
sudo chmod 0755 /etc/hora-police /var/lib/hora-police /var/log/hora-police
sudo chmod 0700 /var/lib/hora-police/quarantine

# 4. Install tmpfiles.d config (if present in repo)
if [ -f "${REPO_DIR}/etc/tmpfiles.d/hora-police.conf" ]; then
  sudo cp "${REPO_DIR}/etc/tmpfiles.d/hora-police.conf" "${TMPFILE}"
  sudo systemd-tmpfiles --create "${TMPFILE}" || true
  echo "✅ tmpfiles.d configuration installed"
else
  echo "⚠️  tmpfiles.d config not found in repo, creating minimal version"
  sudo tee "${TMPFILE}" > /dev/null <<'TMPFILES'
# Type Path          Mode UID  GID Age Argument
d /var/lib/hora-police 0755 root root -
d /var/lib/hora-police/quarantine 0700 root root -
d /var/log/hora-police 0755 root root -
d /etc/hora-police 0755 root root -
TMPFILES
  sudo systemd-tmpfiles --create "${TMPFILE}" || true
fi

# 5. Ensure config exists (if not, copy example if present)
if [ ! -f "${CONFIG}" ]; then
  if [ -f "${REPO_DIR}/config.toml.example" ]; then
    sudo cp "${REPO_DIR}/config.toml.example" "${CONFIG}"
    sudo chmod 0644 "${CONFIG}"
    echo "✅ Config file created from example"
  else
    echo "WARNING: ${CONFIG} not found and no example to copy. Create a valid config before enabling auto-kill."
  fi
fi

# 6. Validate service unit path & binary
echo ""
echo "=== Binary Validation ==="
if [ -f "${BINARY}" ]; then
  echo "✅ Binary found: ${BINARY}"
  echo ""
  echo "=== Binary details ==="
  ls -l "${BINARY}" || true
  FILE_INFO=$(file "${BINARY}" 2>&1 || echo "file command failed")
  echo "${FILE_INFO}"
  
  # Check if it's a valid ELF binary
  if ! echo "${FILE_INFO}" | grep -q "ELF"; then
    echo ""
    echo "❌ ERROR: ${BINARY} is NOT a valid ELF executable"
    echo "Binary may be corrupted or wrong file type"
    echo ""
    echo "REMEDIATION: Rebuild and copy binary again"
    echo "  See: REMEDIATION_BINARY_MISSING.md or run: ./copy-binary-from-wsl.sh"
    exit 2
  fi
  
  # Check architecture
  if echo "${FILE_INFO}" | grep -q "x86-64"; then
    echo "✅ Architecture: x86-64 (correct)"
  elif echo "${FILE_INFO}" | grep -q "ARM"; then
    echo ""
    echo "❌ ERROR: Binary is ARM architecture but system is x86_64"
    echo "REMEDIATION: Rebuild for x86_64 target"
    exit 2
  fi
  
  echo ""
  echo "=== Binary dependencies ==="
  if ldd "${BINARY}" >/dev/null 2>&1; then
    ldd "${BINARY}" || true
    MISSING_DEPS=$(ldd "${BINARY}" 2>&1 | grep "not found" || true)
    if [ -n "${MISSING_DEPS}" ]; then
      echo ""
      echo "❌ ERROR: Missing shared library dependencies:"
      echo "${MISSING_DEPS}"
      echo ""
      echo "REMEDIATION: Install missing libraries or use statically linked binary"
      exit 2
    fi
    echo "✅ All dependencies available"
  else
    echo "Binary appears to be statically linked (OK)"
  fi
  
  # Ensure executable permission
  sudo chmod +x "${BINARY}" || true
  PERMS=$(stat -c "%a" "${BINARY}" 2>/dev/null || stat -f "%OLp" "${BINARY}" 2>/dev/null || echo "unknown")
  echo "Permissions: ${PERMS}"
  
  echo ""
  echo "=== Testing binary execution ==="
  if sudo "${BINARY}" --help >/dev/null 2>&1; then
    echo "✅ Binary executes successfully (--help works)"
  elif sudo "${BINARY}" --version >/dev/null 2>&1; then
    echo "✅ Binary executes successfully (--version works)"
  else
    echo "⚠️  Binary execution test failed"
    echo "Attempting to see error:"
    sudo "${BINARY}" 2>&1 | head -3 || true
    echo ""
    echo "⚠️  WARNING: Binary exists but execution test failed"
    echo "This may cause 203/EXEC errors. Continuing anyway..."
  fi
else
  echo "❌ ERROR: binary ${BINARY} not found."
  echo ""
  echo "=== REMEDIATION REQUIRED ==="
  echo "Please copy a prebuilt Linux x86_64 binary to ${BINARY}"
  echo ""
  echo "Option 1: From WSL (Recommended):"
  echo "  cd /mnt/f/Personal_Projects/Hora-Police"
  echo "  cargo build --release"
  echo "  scp target/release/hora-police deploy@62.72.13.136:/tmp/"
  echo "  ssh deploy@62.72.13.136 'sudo mv /tmp/hora-police ${BINARY} && sudo chmod +x ${BINARY}'"
  echo ""
  echo "Option 2: Use copy script from WSL:"
  echo "  ./copy-binary-from-wsl.sh"
  echo ""
  echo "Option 3: Build on VPS (if memory allows):"
  echo "  cd ${REPO_DIR}"
  echo "  source \$HOME/.cargo/env"
  echo "  cargo build --release -j1"
  echo "  sudo cp target/release/hora-police ${BINARY}"
  echo "  sudo chmod +x ${BINARY}"
  echo ""
  exit 2
fi

# 7. Replace systemd unit with safe content (back up old unit)
if [ -f "${SERVICE_UNIT}" ]; then
  sudo cp "${SERVICE_UNIT}" "${SERVICE_UNIT}.bak-$(date -u +%Y%m%dT%H%M%SZ)" || true
  echo "✅ Backed up existing service file"
fi

sudo tee "${SERVICE_UNIT}" > /dev/null <<'UNIT'
[Unit]
Description=Hora-Police Anti-Malware Daemon
After=network.target
StartLimitIntervalSec=300
StartLimitBurst=5

[Service]
Type=simple
User=root
ExecStart=/usr/local/bin/hora-police /etc/hora-police/config.toml
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal
CPUQuota=15%
MemoryMax=128M
TasksMax=1024
NoNewPrivileges=true
PrivateTmp=false
ProtectSystem=strict
ProtectHome=true
ReadOnlyPaths=/proc /sys
ReadWritePaths=/var/lib/hora-police /etc/hora-police /var/log/hora-police

[Install]
WantedBy=multi-user.target
UNIT

echo "✅ Service unit file installed"

# 8. Reload systemd and clear failed state
sudo systemctl daemon-reload
sudo systemctl reset-failed hora-police || true
echo "✅ systemd reloaded and failed state cleared"

# 9. Start and follow journals briefly
echo ""
echo "=== Starting service ==="
sudo systemctl start hora-police
sleep 2

echo ""
echo "=== Service Status ==="
sudo systemctl status -l hora-police --no-pager || true

echo ""
echo "=== Journal (last 120 lines) ==="
sudo journalctl -u hora-police -n 120 --no-pager || true

echo ""
echo "=== Checking for errors ==="
JOURNAL_ERRORS=$(sudo journalctl -u hora-police -n 200 --no-pager | grep -iE 'NAMESPACE|EXEC|Failed.*EXEC|error.*226|error.*203' || true)
if [ -n "${JOURNAL_ERRORS}" ]; then
  echo "⚠️  WARNING: Found NAMESPACE or EXEC errors in journal"
  echo "=== Error details ==="
  echo "${JOURNAL_ERRORS}"
  echo ""
  
  # Check specifically for 203/EXEC
  if echo "${JOURNAL_ERRORS}" | grep -qi "203/EXEC\|status=203"; then
    echo "❌ 203/EXEC error detected - binary execution failed"
    echo ""
    echo "=== Additional Diagnostics ==="
    echo "Run diagnostic script for detailed analysis:"
    echo "  ./diagnose-binary.sh"
    echo ""
    echo "Common causes:"
    echo "  1. Binary missing or not executable"
    echo "  2. Wrong architecture (ARM vs x86_64)"
    echo "  3. Missing shared library dependencies"
    echo "  4. Corrupted binary"
    echo "  5. SELinux/AppArmor restrictions"
    echo ""
    echo "REMEDIATION:"
    echo "  1. Run: ./diagnose-binary.sh"
    echo "  2. If binary missing/wrong: ./copy-binary-from-wsl.sh (from WSL)"
    echo "  3. Re-run this script after fixing binary"
  fi
else
  echo "✅ No NAMESPACE or EXEC errors found in recent journal"
fi

echo ""
echo "== Fix script completed at $(date -u) =="
