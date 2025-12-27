# Alternative Build Methods for Hora-Police

When building on the target server fails due to memory constraints, use these alternative methods.

## Method 1: Debug Build (Lowest Memory)

Debug builds use significantly less memory during compilation:

```bash
cd /srv/Hora-Police
cargo build -j1

# Install debug binary
sudo cp target/debug/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
```

**Pros:**
- Uses ~50% less memory during compilation
- Faster compilation
- Works on low-memory systems

**Cons:**
- Binary is 3-5x larger
- Slower runtime performance
- Can rebuild release later when system is stable

## Method 2: Low-Memory Profile Build

Use the custom lowmem profile in Cargo.toml:

```bash
cd /srv/Hora-Police
cargo build --profile lowmem -j1

# Install
sudo cp target/lowmem/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
```

**Pros:**
- Balanced between memory usage and performance
- Smaller than debug, faster than debug

**Cons:**
- Still uses more memory than debug
- May still fail on very constrained systems

## Method 3: Build on Different Machine

Build on a machine with more RAM, then transfer the binary.

### On Build Machine (with more RAM):

```bash
# Clone repository
git clone <repository-url> Hora-Police
cd Hora-Police

# Build release
cargo build --release

# Verify binary
ls -lh target/release/hora-police

# Transfer to server
scp target/release/hora-police deploy@mail-server:/tmp/hora-police
```

### On Target Server:

```bash
# Install transferred binary
sudo cp /tmp/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police

# Verify
/usr/local/bin/hora-police --help
```

**Build Machine Requirements:**
- Same architecture (x86_64-unknown-linux-gnu)
- Rust toolchain installed
- Same or newer Rust version

## Method 4: Use Pre-built Binary

If available, download a pre-built binary:

```bash
# Download (example - adjust URL)
wget https://example.com/releases/hora-police-latest -O /tmp/hora-police

# Verify checksum (if provided)
# sha256sum /tmp/hora-police

# Install
sudo cp /tmp/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
```

## Method 5: Increase System Limits

If build is being killed due to ulimits or cgroups:

### Check Current Limits:

```bash
# User limits
ulimit -a

# OOM killer logs
dmesg | grep -i oom | tail -20

# Cgroup limits (if applicable)
cat /sys/fs/cgroup/memory/memory.limit_in_bytes
```

### Increase Virtual Memory Limit:

```bash
# Check current limit
ulimit -v

# Increase to unlimited (for current session)
ulimit -v unlimited

# Or set specific limit (e.g., 8GB)
ulimit -v 8388608  # 8GB in KB

# Try build again
cargo build --release -j1
```

### Make Permanent:

Add to `~/.bashrc` or `/etc/security/limits.conf`:

```bash
# In ~/.bashrc
ulimit -v unlimited

# Or in /etc/security/limits.conf (requires root)
deploy soft as unlimited
deploy hard as unlimited
```

## Method 6: Build in Stages

Build dependencies first, then the main binary:

```bash
cd /srv/Hora-Police

# Fetch and build dependencies only
cargo fetch
cargo build --release --lib -j1

# Then build main binary
cargo build --release --bin hora-police -j1
```

## Method 7: Use Docker (if available)

Build in a container with controlled memory limits:

```bash
# Create Dockerfile
cat > Dockerfile << 'EOF'
FROM rust:1.92-slim
WORKDIR /build
COPY . .
RUN cargo build --release
EOF

# Build
docker build -t hora-police-build .

# Extract binary
docker create --name temp hora-police-build
docker cp temp:/build/target/release/hora-police ./hora-police
docker rm temp

# Install
sudo cp hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police
```

## Recommended Approach for Your Situation

Given your current issues (OOM kills, 2631 zombies):

1. **First**: Try debug build (Method 1) - fastest to get running
2. **Second**: If debug works, use it temporarily and rebuild release later
3. **Third**: If debug also fails, build on different machine (Method 3)
4. **Long-term**: Fix zombie processes, then rebuild release

## Quick Commands

```bash
# Quick debug build
cd /srv/Hora-Police && cargo build -j1 && sudo cp target/debug/hora-police /usr/local/bin/hora-police && sudo chmod +x /usr/local/bin/hora-police

# Check memory limits
./scripts/check-memory-limits.sh

# Use lowmem build script
chmod +x build-lowmem.sh
./build-lowmem.sh
```

## Troubleshooting Build Failures

### If build is killed immediately:

```bash
# Check OOM killer
dmesg | tail -50 | grep -i kill

# Check available memory
free -h

# Check zombie processes
ps aux | awk '$8=="Z" {print}' | wc -l
```

### If build fails with "signal: 9":

This is OOM kill. Try:
1. Increase swap (already done - 4GB)
2. Kill unnecessary processes
3. Use debug build
4. Build on different machine

### If build fails with compilation errors:

```bash
# Check Rust version
rustc --version

# Update Rust
rustup update stable

# Clean and rebuild
cargo clean
cargo build --release -j1
```

