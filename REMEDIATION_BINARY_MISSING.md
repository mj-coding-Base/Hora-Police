# Remediation: Binary Missing or Not Executable

If `fix-service-directories.sh` exits with error code 2, or you see `status=203/EXEC` errors, the binary is missing, not executable, or has issues.

## Quick Diagnosis

**On VPS, run the diagnostic script**:

```bash
cd /srv/Hora-Police
chmod +x diagnose-binary.sh
./diagnose-binary.sh
```

**Or manually check**:

```bash
# Check if binary exists
ls -l /usr/local/bin/hora-police

# Check file type
file /usr/local/bin/hora-police

# Check dependencies
ldd /usr/local/bin/hora-police

# Check permissions
stat /usr/local/bin/hora-police

# Test execution
sudo /usr/local/bin/hora-police --help
```

## Solution: Copy Prebuilt Binary

### Option 1: Use Automated Script (Recommended)

**On your Windows machine (in WSL)**:

```bash
cd /mnt/f/Personal_Projects/Hora-Police
chmod +x copy-binary-from-wsl.sh
./copy-binary-from-wsl.sh
```

This script will:
- Build the binary if needed
- Copy it to VPS
- Install with correct permissions
- Verify installation

### Option 2: Manual Copy from WSL

**On your Windows machine (in WSL)**:

```bash
# Navigate to project
cd /mnt/f/Personal_Projects/Hora-Police

# Build if not already built
cargo build --release

# Copy to VPS (replace with your VPS IP)
scp target/release/hora-police deploy@62.72.13.136:/tmp/hora-police

# On VPS, install binary
ssh deploy@62.72.13.136 'sudo mv /tmp/hora-police /usr/local/bin/hora-police && sudo chmod +x /usr/local/bin/hora-police'
```

**Or in one step**:

```bash
# From WSL
cd /mnt/f/Personal_Projects/Hora-Police
cargo build --release
scp target/release/hora-police deploy@62.72.13.136:/tmp/
ssh deploy@62.72.13.136 'sudo mv /tmp/hora-police /usr/local/bin/hora-police && sudo chmod +x /usr/local/bin/hora-police && sudo systemctl restart hora-police'
```

### Option 3: From Local Linux Machine

```bash
# Build
cd /path/to/Hora-Police
cargo build --release

# Copy to VPS (replace with your VPS IP)
scp target/release/hora-police deploy@62.72.13.136:/tmp/

# On VPS
ssh deploy@62.72.13.136
sudo mv /tmp/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
sudo systemctl restart hora-police
```

### Option 4: Build on VPS (If Memory Allows)

**⚠️ WARNING**: This may fail with OOM on low-memory VPS instances.

```bash
# On VPS
cd /srv/Hora-Police
source $HOME/.cargo/env

# Build with minimal memory usage
RUSTFLAGS="-C opt-level=3" cargo build --release -j1

# If successful, install
sudo cp target/release/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
sudo systemctl restart hora-police
```

### Option 4: From CI/CD Artifact

If you have GitHub Actions or CI/CD:

```bash
# Download from CI artifact URL (example)
curl -L https://github.com/your-org/Hora-Police/releases/download/v0.1.0/hora-police-linux-x86_64 -o /tmp/hora-police

# Install
sudo mv /tmp/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
sudo systemctl restart hora-police
```

## Verification After Copy

After copying the binary, verify it works:

**On VPS**:

```bash
# Run comprehensive diagnostic
cd /srv/Hora-Police
./diagnose-binary.sh

# Or manually verify
sudo /usr/local/bin/hora-police --help
file /usr/local/bin/hora-police
ldd /usr/local/bin/hora-police

# Run fix script to ensure everything is set up
./fix-service-directories.sh

# Check service status
sudo systemctl status hora-police --no-pager

# Check for errors
sudo journalctl -u hora-police -n 50 --no-pager | grep -iE 'EXEC|error' || echo "No errors"
```

## Expected Binary Properties

- **Size**: ~5-15 MB (stripped release binary)
- **Type**: ELF 64-bit LSB executable, x86-64
- **Permissions**: 0755 (-rwxr-xr-x)
- **Owner**: root:root
- **Dependencies**: Should link to libc.so.6 (or be statically linked)
- **Architecture**: x86-64 (must match system: `uname -m` should show `x86_64`)

## VPS Information

- **VPS IP**: 62.72.13.136
- **User**: deploy
- **Binary Path**: /usr/local/bin/hora-police
- **Project Path**: /srv/Hora-Police

## Common Issues

### Binary Not Executable

```bash
sudo chmod +x /usr/local/bin/hora-police
```

### Wrong Architecture

If you see "cannot execute binary file: Exec format error":
- Binary is for wrong architecture (e.g., ARM instead of x86_64)
- Rebuild for correct target: `cargo build --release --target x86_64-unknown-linux-gnu`

### Missing Dependencies

If `ldd` shows "not found" for libraries:
- Install required system libraries
- Or use statically linked binary

### Permission Denied

```bash
sudo chown root:root /usr/local/bin/hora-police
sudo chmod 755 /usr/local/bin/hora-police
```

### 203/EXEC Error Persists

If you still get 203/EXEC after copying binary:

1. **Run diagnostic**:
   ```bash
   cd /srv/Hora-Police
   ./diagnose-binary.sh
   ```

2. **Check filesystem mount**:
   ```bash
   mount | grep $(df /usr/local/bin | tail -1 | awk '{print $1}')
   ```
   If mounted with `noexec`, that's the problem.

3. **Check AppArmor/SELinux**:
   ```bash
   # AppArmor
   sudo aa-status | grep hora-police || echo "No AppArmor profile"
   
   # SELinux
   getenforce 2>/dev/null || echo "SELinux not available"
   ```

4. **Verify service unit**:
   ```bash
   sudo systemctl cat hora-police | grep ExecStart
   ```
   Should show: `ExecStart=/usr/local/bin/hora-police /etc/hora-police/config.toml`

5. **Test manual execution as root**:
   ```bash
   sudo /usr/local/bin/hora-police --help
   ```
   If this fails, the binary itself has issues.

