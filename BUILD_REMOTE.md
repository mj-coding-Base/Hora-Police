# Building Hora-Police on Remote Machine

Since local build keeps failing due to OOM, build on a machine with more resources.

## Method 1: Build on Your Windows Machine (WSL)

### Prerequisites
- Windows with WSL2 installed
- Or Docker Desktop

### Steps

```bash
# In WSL
cd /mnt/f/Personal_Projects/Hora-Police

# Or if using Docker
docker run -it -v /f/Personal_Projects/Hora-Police:/build rust:1.92 bash
cd /build

# Build
cargo build --release

# Binary will be at: target/release/hora-police
```

### Transfer to Server

```bash
# From Windows (PowerShell or WSL)
scp target/release/hora-police deploy@mail-server:/tmp/hora-police

# Or use WinSCP / FileZilla to transfer
```

## Method 2: Build on Temporary Cloud Instance

### Using DigitalOcean / AWS / Linode

```bash
# 1. Create temporary droplet/instance (2GB+ RAM, Ubuntu 24.04)

# 2. SSH into it
ssh root@<instance-ip>

# 3. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env
rustup default stable

# 4. Clone and build
git clone <your-repo-url> Hora-Police
cd Hora-Police
cargo build --release

# 5. Transfer to your server
scp target/release/hora-police deploy@mail-server:/tmp/hora-police

# 6. Delete the temporary instance (save costs)
```

## Method 3: Use GitHub Actions

If your repo is on GitHub:

1. Create `.github/workflows/build.yml`:
```yaml
name: Build Hora-Police

on:
  workflow_dispatch:

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo build --release
      - uses: actions/upload-artifact@v3
        with:
          name: hora-police-binary
          path: target/release/hora-police
```

2. Run workflow, download artifact
3. Transfer to server

## On Your Server After Transfer

```bash
# 1. Install binary
sudo cp /tmp/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police

# 2. Verify
/usr/local/bin/hora-police --help

# 3. Fix service (use fix-service-complete.sh)
chmod +x fix-service-complete.sh
./fix-service-complete.sh

# 4. Start service
sudo systemctl start hora-police
sudo systemctl status hora-police
```

## Quick Transfer Commands

### From Local Windows (WSL)
```bash
# In WSL, after building
scp target/release/hora-police deploy@mail-server:/tmp/hora-police
```

### From Cloud Instance
```bash
# After building on cloud instance
scp target/release/hora-police deploy@mail-server:/tmp/hora-police
```

### Using SCP from Windows (PowerShell)
```powershell
# If you have OpenSSH client installed
scp target\release\hora-police deploy@mail-server:/tmp/hora-police
```

### Using WinSCP / FileZilla
1. Connect to server
2. Navigate to `/tmp/`
3. Upload `target/release/hora-police` from your local machine
4. Rename to `hora-police` on server

## Verify Binary Compatibility

The binary must match your server's architecture:

```bash
# Check server architecture
uname -m
# Should be: x86_64

# Check binary architecture (on build machine)
file target/release/hora-police
# Should show: ELF 64-bit LSB executable, x86-64

# If architectures don't match, build won't work
```

## After Installing Binary

```bash
# Test binary
sudo /usr/local/bin/hora-police --help

# Fix service
chmod +x fix-service-complete.sh
./fix-service-complete.sh

# Start service
sudo systemctl start hora-police
sudo systemctl status hora-police
```

