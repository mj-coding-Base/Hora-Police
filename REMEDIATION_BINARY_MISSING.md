# Remediation: Binary Missing or Not Executable

If `fix-service-directories.sh` exits with error code 2, the binary is missing or not executable.

## Quick Diagnosis

Run these commands to diagnose:

```bash
# Check if binary exists
ls -l /usr/local/bin/hora-police

# Check file type
file /usr/local/bin/hora-police

# Check dependencies
ldd /usr/local/bin/hora-police

# Check permissions
stat /usr/local/bin/hora-police
```

## Solution: Copy Prebuilt Binary

### Option 1: From Local WSL Build (Recommended)

**On your Windows machine (in WSL)**:

```bash
# Navigate to project
cd /mnt/f/Personal_Projects/Hora-Police

# Build if not already built
cargo build --release

# Copy to VPS
scp target/release/hora-police deploy@<VPS_IP>:/tmp/hora-police

# On VPS, install binary
ssh deploy@<VPS_IP> 'sudo mv /tmp/hora-police /usr/local/bin/hora-police && sudo chmod +x /usr/local/bin/hora-police'
```

**Or in one step**:

```bash
# From WSL
cd /mnt/f/Personal_Projects/Hora-Police
cargo build --release
scp target/release/hora-police deploy@<VPS_IP>:/tmp/
ssh deploy@<VPS_IP> 'sudo mv /tmp/hora-police /usr/local/bin/hora-police && sudo chmod +x /usr/local/bin/hora-police && sudo systemctl restart hora-police'
```

### Option 2: From Local Linux Machine

```bash
# Build
cd /path/to/Hora-Police
cargo build --release

# Copy to VPS
scp target/release/hora-police deploy@<VPS_IP>:/tmp/

# On VPS
ssh deploy@<VPS_IP>
sudo mv /tmp/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
sudo systemctl restart hora-police
```

### Option 3: Build on VPS (If Memory Allows)

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

```bash
# Test binary
sudo /usr/local/bin/hora-police --help

# Check it's executable
file /usr/local/bin/hora-police

# Check dependencies
ldd /usr/local/bin/hora-police

# Restart service
sudo systemctl restart hora-police
sudo systemctl status hora-police
```

## Expected Binary Properties

- **Size**: ~5-15 MB (stripped release binary)
- **Type**: ELF 64-bit LSB executable, x86-64
- **Permissions**: 0755 (-rwxr-xr-x)
- **Owner**: root:root
- **Dependencies**: Should link to libc.so.6 (or be statically linked)

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

