# Building from Source

This guide covers building Probing from source code, including all dependencies and components.

## Prerequisites

### Required Tools

**Rust Toolchain:**
```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

**Python Development:**
```bash
# Python 3.7+ required
python3 --version

# Development dependencies
pip install setuptools wheel build
```

**System Dependencies:**
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install build-essential pkg-config libssl-dev

# CentOS/RHEL/Fedora
sudo yum groupinstall "Development Tools"
sudo yum install openssl-devel

# macOS (with Homebrew)
brew install pkg-config openssl
```

### Optional Tools

**For cross-platform builds:**
```bash
# Install Zig for better compatibility
cargo install cargo-zigbuild
```

**For web UI development:**
```bash
# Install Trunk for WebAssembly builds
cargo install trunk
```

## Quick Build

### Standard Build

Clone and build the project:
```bash
# Clone repository
git clone https://github.com/reiase/probing.git
cd probing

# Build all components
make

# Verify build
./target/release/probing --version
```

### Development Build

For faster development cycles:
```bash
# Development build (debug mode)
cargo build

# Build specific components
cargo build -p probing-cli     # CLI only
cargo build -p probing-core    # Core library
cargo build -p probing-server  # HTTP server
```

## Detailed Build Process

### Core Components

**1. Probing Core Library:**
```bash
cd probing/core
cargo build --release
```

**2. Command Line Interface:**
```bash
cd probing/cli
cargo build --release
```

**3. Python Extension:**
```bash
cd probing/extensions/python
cargo build --release
```

**4. Server Component:**
```bash
cd probing/server
cargo build --release
```

### Python Wheel Package

**Build Python wheel:**
```bash
# Standard build
python make_wheel.py

# Cross-platform build with Zig
make ZIG=1

# Install locally
pip install dist/probing-*.whl --force-reinstall
```

### Web UI (Optional)

**Build web interface:**
```bash
cd app
trunk build --release
```

The web UI is built as WebAssembly and provides a graphical interface for Probing.

## Build Configurations

### Release Build

For production deployment:
```bash
# Optimized release build
cargo build --release

# Full production build with all optimizations
make ZIG=1
```

### Debug Build

For development and debugging:
```bash
# Debug build with symbols
cargo build

# With additional debug info
RUSTFLAGS="-C debug-assertions=on" cargo build
```

### Feature Flags

Enable specific features during build:
```bash
# Build with all features
cargo build --all-features

# Build with specific features
cargo build --features "python-ext,server"

# Build minimal version
cargo build --no-default-features
```

## Cross-Platform Building

### Linux Distributions

**For older glibc compatibility:**
```bash
# Use Zig for better compatibility
make ZIG=1

# Manual Zig build
cargo zigbuild --release --target x86_64-unknown-linux-gnu
```

**Static linking:**
```bash
# Static binary (self-contained)
RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --target x86_64-unknown-linux-musl
```

### macOS

```bash
# Standard macOS build
cargo build --release

# Universal binary (Intel + Apple Silicon)
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
```

### Windows

```bash
# Windows build (from Linux using cross-compilation)
cargo build --release --target x86_64-pc-windows-gnu
```

## Testing the Build

### Basic Functionality

```bash
# Run test suite
make test

# Verify CLI installation
./target/release/probing --version

# Test basic functionality
PROBE=1 python examples/test_probing.py
```

### Advanced Testing

```bash
# Test PyTorch integration
PROBE_TORCH_EXPRS="loss@train,acc1@train" PROBE=1 python examples/imagenet.py

# Test SQL interface
./target/release/probing $ENDPOINT query "SELECT * FROM information_schema.df_settings"

# Test web UI (if built)
cd app && trunk serve
```

### Performance Testing

```bash
# Benchmark core components
cargo bench

# Memory usage testing
valgrind ./target/release/probing $ENDPOINT inject

# Load testing
./scripts/load_test.sh
```

## Installation

### System-wide Installation

```bash
# Install to system location
sudo cp target/release/probing /usr/local/bin/
sudo cp target/release/libprobing.so /usr/local/lib/

# Update library cache
sudo ldconfig
```

### User Installation

```bash
# Install to user directory
mkdir -p ~/.local/bin ~/.local/lib
cp target/release/probing ~/.local/bin/
cp target/release/libprobing.so ~/.local/lib/

# Add to PATH if needed
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
```

### Python Package Installation

```bash
# Install Python wheel
pip install dist/probing-*.whl --force-reinstall

# Verify Python integration
python -c "import probing; print('Success')"
```

## Development Setup

### IDE Configuration

**VS Code setup:**
```json
// .vscode/settings.json
{
  "rust-analyzer.cargo.features": ["all"],
  "rust-analyzer.check.command": "clippy"
}
```

**Environment variables:**
```bash
# Development environment
export RUST_LOG=debug
export PROBING_LOGLEVEL=debug
export RUST_BACKTRACE=1
```

### Pre-commit Hooks

```bash
# Install pre-commit hooks
pip install pre-commit
pre-commit install

# Manual format and lint
cargo fmt
cargo clippy -- -D warnings
```

## Troubleshooting

### Common Build Issues

**Linker errors:**
```bash
# Install required system libraries
sudo apt install build-essential pkg-config libssl-dev

# Check linker configuration
rustc --print cfg
```

**Python extension issues:**
```bash
# Ensure Python headers are available
sudo apt install python3-dev

# Check Python version compatibility
python3 --version
```

**Cross-compilation issues:**
```bash
# Install cross-compilation target
rustup target add x86_64-unknown-linux-musl

# Use Zig for better compatibility
cargo install cargo-zigbuild
cargo zigbuild --release
```

### Performance Issues

**Slow builds:**
```bash
# Use more CPU cores
export CARGO_BUILD_JOBS=8

# Enable parallel frontend
export RUSTFLAGS="-C codegen-units=16"

# Use faster linker (Linux)
sudo apt install lld
export RUSTFLAGS="-C link-arg=-fuse-ld=lld"
```

**Memory usage during build:**
```bash
# Reduce memory usage
export RUSTFLAGS="-C opt-level=1"

# Build with reduced parallelism
cargo build --jobs 2
```

## Project Structure

Understanding the codebase organization:

```
probing/
├── probing/cli/          # Command-line interface
├── probing/core/         # Core profiling engine  
├── probing/extensions/   # Language-specific extensions
│   ├── python/          # Python integration
│   └── cc/              # C++ integration
├── probing/server/       # HTTP API server
├── app/                 # Web UI (Leptos + WebAssembly)
├── python/              # Python hooks and bindings
├── examples/            # Usage examples and demos
└── docs/                # Documentation
```

### Key Files

- `Cargo.toml` - Main workspace configuration
- `Makefile` - Build automation
- `make_wheel.py` - Python package builder
- `probing/cli/src/main.rs` - CLI entry point
- `probing/core/src/lib.rs` - Core library
- `python/probing/__init__.py` - Python package entry

## Contributing to the Build System

### Adding New Features

1. **Add Cargo features:**
```toml
# In Cargo.toml
[features]
new-feature = ["dep:some-crate"]
```

2. **Update build scripts:**
```bash
# In Makefile
build-new-feature:
    cargo build --features new-feature
```

3. **Test integration:**
```bash
# Add tests
cargo test --features new-feature
```

### Build Optimizations

**Profile-guided optimization:**
```bash
# Generate profile data
RUSTFLAGS="-C profile-generate=/tmp/pgo-data" cargo build --release

# Run representative workload
./target/release/probing <typical-usage>

# Build with profile data
RUSTFLAGS="-C profile-use=/tmp/pgo-data" cargo build --release
```

**Link-time optimization:**
```bash
# Enable LTO in Cargo.toml
[profile.release]
lto = true
codegen-units = 1
```

For development workflow and contribution guidelines, see the main [README.md](../README.md) and [Contributing Guide](contributing.md).
