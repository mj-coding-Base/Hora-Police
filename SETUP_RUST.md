# Setting Up Rust Toolchain

## Problem: Rust Commands Not Found

After installing rustup, you need to add Rust to your PATH.

## Quick Fix

```bash
# Load Rust environment
source $HOME/.cargo/env

# Verify it works
rustc --version
cargo --version
```

## Permanent Fix

Add to your shell profile so it loads automatically:

```bash
# Add to ~/.bashrc
echo 'source $HOME/.cargo/env' >> ~/.bashrc

# Reload shell
source ~/.bashrc

# Or just start a new shell session
```

## Complete Setup (If Rust Not Installed)

If Rust is not installed at all:

```bash
# Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Load environment
source $HOME/.cargo/env

# Set default toolchain
rustup default stable

# Verify
rustc --version
cargo --version
```

## For Current Session Only

If you just need to build now:

```bash
# Load Rust environment
source $HOME/.cargo/env

# Now build
cd /srv/Hora-Police
cargo build -j1
```

