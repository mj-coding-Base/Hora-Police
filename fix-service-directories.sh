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
if [ -f "${BINARY}" ]; then
  echo "✅ Binary found: ${BINARY}"
  echo "=== Binary details ==="
  ls -l "${BINARY}" || true
  file "${BINARY}" || true
  echo "=== Binary dependencies ==="
  ldd "${BINARY}" 2>&1 || echo "ldd failed (static binary or missing deps)"
  sudo chmod +x "${BINARY}" || true
  echo "=== Testing binary execution ==="
  if sudo "${BINARY}" --help >/dev/null 2>&1; then
    echo "✅ Binary is executable"
  else
    echo "⚠️  Binary exists but --help test failed (may need config file)"
  fi
else
  echo "❌ ERROR: binary ${BINARY} not found."
  echo ""
  echo "=== REMEDIATION REQUIRED ==="
  echo "Please copy a prebuilt Linux x86_64 binary to ${BINARY}"
  echo ""
  echo "Option 1: From local machine (WSL/Linux):"
  echo "  scp target/release/hora-police deploy@$(hostname -I | awk '{print $1}'):/tmp/"
  echo "  ssh deploy@$(hostname -I | awk '{print $1}') 'sudo mv /tmp/hora-police ${BINARY} && sudo chmod +x ${BINARY}'"
  echo ""
  echo "Option 2: From Windows (if binary built in WSL):"
  echo "  # In WSL:"
  echo "  scp /mnt/f/Personal_Projects/Hora-Police/target/release/hora-police deploy@<VPS_IP>:/tmp/"
  echo "  # Then on VPS:"
  echo "  sudo mv /tmp/hora-police ${BINARY} && sudo chmod +x ${BINARY}"
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
if sudo journalctl -u hora-police -n 200 --no-pager | grep -qiE 'NAMESPACE|EXEC|Failed.*EXEC|error.*226|error.*203'; then
  echo "⚠️  WARNING: Found NAMESPACE or EXEC errors in journal"
  echo "=== Error details ==="
  sudo journalctl -u hora-police -n 200 --no-pager | grep -iE 'NAMESPACE|EXEC|Failed.*EXEC|error.*226|error.*203' || true
else
  echo "✅ No NAMESPACE or EXEC errors found in recent journal"
fi

echo ""
echo "== Fix script completed at $(date -u) =="
