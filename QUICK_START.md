# Quick Start - Get Hora-Police Running Now

## Step 1: Load Rust Environment

```bash
# Load Rust (required after rustup install)
source $HOME/.cargo/env

# Verify Rust is available
rustc --version
cargo --version
```

If commands still not found, install Rust:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env
rustup default stable
```

## Step 2: Build Debug Version

```bash
cd /srv/Hora-Police
git pull

# Build debug (uses less memory)
cargo build -j1

# This will take 5-15 minutes
```

## Step 3: Install Binary

```bash
# Install debug binary
sudo cp target/debug/hora-police /usr/local/bin/hora-police
sudo chmod +x /usr/local/bin/hora-police

# Verify
/usr/local/bin/hora-police --help
```

## Step 4: Fix Service and Start

```bash
# Stop service
sudo systemctl stop hora-police

# Update service file (remove PrivateTmp)
sudo cp hora-police.service /etc/systemd/system/
sudo systemctl daemon-reload

# Start service
sudo systemctl start hora-police
sudo systemctl status hora-police
```

## All-in-One Command

```bash
source $HOME/.cargo/env && \
cd /srv/Hora-Police && \
git pull && \
cargo build -j1 && \
sudo cp target/debug/hora-police /usr/local/bin/hora-police && \
sudo chmod +x /usr/local/bin/hora-police && \
sudo systemctl stop hora-police && \
sudo cp hora-police.service /etc/systemd/system/ && \
sudo systemctl daemon-reload && \
sudo systemctl start hora-police && \
sudo systemctl status hora-police
```

