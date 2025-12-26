# 🔧 Build Notes & Troubleshooting

## Potential API Compatibility Issues

### sysinfo Crate

The `sysinfo` crate has undergone significant API changes. If you encounter compilation errors related to `sysinfo`, you may need to adjust the API calls in `src/process_monitor.rs`.

**Common Issues**:

1. **Process enumeration**: The API for iterating processes may differ
2. **Method names**: `exe()`, `cmd()`, `cpu_usage()` may have different names
3. **User ID access**: `user_id()` vs `uid()` method names

**Solution**: Check the [sysinfo documentation](https://docs.rs/sysinfo/) for version 0.30 to verify correct API usage.

### sqlx Crate

If you encounter issues with `sqlx`, ensure you have:
- SQLite3 development libraries: `sudo apt-get install libsqlite3-dev`
- Correct feature flags in `Cargo.toml`

### nix Crate

The `nix` crate requires Linux. This project is designed for Linux/Ubuntu only.

## Building on Ubuntu

```bash
# Install dependencies
sudo apt-get update
sudo apt-get install -y build-essential libsqlite3-dev pkg-config

# Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Build
./build.sh
# or
cargo build --release
```

## Testing Without Full Build

If you can't build immediately, you can verify the code structure:

```bash
# Check syntax (requires Rust)
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy
```

## Common Compilation Errors

### Error: "cannot find crate `sysinfo`"

**Solution**: Run `cargo build` to download dependencies.

### Error: "linker `cc` not found"

**Solution**: Install build tools:
```bash
sudo apt-get install build-essential
```

### Error: "package `sqlite3-sys` not found"

**Solution**: Install SQLite development libraries:
```bash
sudo apt-get install libsqlite3-dev
```

### Error: "use of undeclared crate or module"

**Solution**: Check that all modules are declared in `src/lib.rs`.

## API Updates Needed

If `sysinfo` 0.30 has different APIs, update `src/process_monitor.rs`:

1. Check [sysinfo 0.30 docs](https://docs.rs/sysinfo/0.30.0/sysinfo/)
2. Update method calls to match the API
3. Test with `cargo check`

## Alternative: Use Older sysinfo Version

If compatibility issues persist, you can pin to an older version:

```toml
sysinfo = "0.29"  # or another compatible version
```

Then update the code to match that version's API.

## Verification Checklist

- [ ] Rust toolchain installed (`rustc --version`)
- [ ] SQLite3 dev libraries installed
- [ ] All dependencies in `Cargo.toml` are valid
- [ ] `cargo check` passes
- [ ] `cargo build --release` succeeds
- [ ] Binary runs without errors

## Getting Help

1. Check Rust compiler errors carefully
2. Verify dependency versions match documentation
3. Test on a clean Ubuntu 20.04+ system
4. Review [Rust Book](https://doc.rust-lang.org/book/) for syntax help

---

**Note**: This project is designed for Linux. Windows/Mac builds are not supported.

