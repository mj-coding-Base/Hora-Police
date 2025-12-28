# Fix: linker `cc` not found

## Problem
The build fails with:
```
error: linker `cc` not found
```

This means the C compiler (gcc) is not installed.

## Solution

### Step 1: Install Build Dependencies

```bash
cd /srv/Hora-Police

# Make install script executable
chmod +x install-build-deps.sh

# Install dependencies
./install-build-deps.sh
```

Or manually:

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev libsqlite3-dev ca-certificates curl
```

### Step 2: Verify Installation

```bash
# Check C compiler
cc --version

# Check Cargo
cargo --version
```

### Step 3: Build

```bash
cd /srv/Hora-Police

# Make sure swap is added (if not already)
free -h
# If no swap, add it:
# sudo fallocate -l 4G /swapfile && sudo chmod 600 /swapfile && sudo mkswap /swapfile && sudo swapon /swapfile

# Build
chmod +x build-lowmem.sh
./build-lowmem.sh
```

### Step 4: Install

```bash
# Copy binary
cp target/release/hora-police /tmp/hora-police

# Install
chmod +x scripts/install-binary.sh
./scripts/install-binary.sh
```

## Complete Sequence

```bash
# 1. Install dependencies
cd /srv/Hora-Police
chmod +x install-build-deps.sh
./install-build-deps.sh

# 2. Add swap (if not already done)
free -h | grep -q "Swap:.*0B" && sudo fallocate -l 4G /swapfile && sudo chmod 600 /swapfile && sudo mkswap /swapfile && sudo swapon /swapfile && echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab || echo "Swap already exists"

# 3. Build
chmod +x build-lowmem.sh
./build-lowmem.sh

# 4. Install
cp target/release/hora-police /tmp/hora-police
chmod +x scripts/install-binary.sh
./scripts/install-binary.sh
```

