# Building Probing from Source

This guide provides comprehensive instructions for building Probing from source code, including environment setup, compilation, and verification steps.

## Prerequisites

### System Requirements

- **Operating System**: Linux (Ubuntu 20.04+, CentOS 8+) or macOS
- **Memory**: Minimum 4GB RAM (8GB+ recommended for large projects)
- **Disk Space**: At least 2GB for dependencies and build artifacts

### Required Dependencies

Before building Probing, install the following dependencies:

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install nightly toolchain (required for advanced features)
rustup toolchain install nightly
rustup default nightly

# Add WebAssembly target for web UI
rustup target add wasm32-unknown-unknown

# Install trunk for building WebAssembly frontend
cargo install trunk

# Install cross-compilation tools (for distribution builds)
cargo install cargo-zigbuild
pip install ziglang
```

### Optional Dependencies

```bash
# For testing and development
cargo install cargo-nextest    # Faster test runner
pip install pytest            # Python testing framework
```

## Building from Source

### Development Build

For quick iteration and development:

```bash
# Clone repository
git clone https://github.com/reiase/probing.git
cd probing

# Development build (faster compilation, debug symbols)
make

# Build web UI (optional, included in make)
cd app && trunk build --release && cd ..
```

### Production Build

For distribution and deployment:

```bash
# Production build with cross-platform compatibility
make ZIG=1

# Generate Python wheel package
make wheel

# Install the built package
pip install dist/probing-*.whl --force-reinstall
```

## Build Verification

## Build Verification

### Basic Functionality Test

Verify the build by running basic tests:

```bash
# Run the test suite
make test

# Verify CLI installation
probing --version

# Test basic functionality
PROBE=1 python examples/test_probing.py
```

### Advanced Feature Testing

Test advanced features and integrations:

```bash
# Test PyTorch integration with variable tracking
PROBE_TORCH_EXPRS="loss@train,acc1@train" PROBE=1 python examples/imagenet.py

# Test distributed monitoring (if available)
probing cluster test

# Test SQL analytics interface
python -c "
import time
import os
os.environ['PROBE'] = '1'
import probing
# Run your test here
"
```

## Build Targets and Options

### Available Make Targets

```bash
# Development build (default)
make

# Production build with cross-compilation
make ZIG=1

# Run tests
make test

# Build Python wheel package
make wheel

# Build web UI only
make app/dist

# Clean build artifacts
make clean
```

### Environment Variables

Control the build process with environment variables:

```bash
# Debug build (faster compilation, larger binaries)
DEBUG=1 make

# Cross-compilation build
ZIG=1 make

# Verbose build output
VERBOSE=1 make
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

## Troubleshooting

### Common Build Issues

**Rust nightly not found:**
```bash
rustup toolchain install nightly
rustup default nightly
```

**WebAssembly target missing:**
```bash
rustup target add wasm32-unknown-unknown
```

**Trunk installation failed:**
```bash
cargo install trunk --force
```

**Cross-compilation errors:**
```bash
# Ensure ziglang is properly installed
pip install ziglang --upgrade
```

### Performance Issues

**Slow compilation:**
- Use development build: `make` (without ZIG=1)
- Enable parallel compilation: `export MAKEFLAGS="-j$(nproc)"`
- Use faster linker: `sudo apt install lld` and add to ~/.cargo/config.toml

**Large binary size:**
- Production build automatically enables optimizations
- Strip debug symbols: `strip target/release/probing`

### Platform-Specific Notes

**Ubuntu/Debian:**
```bash
# Install additional dependencies
sudo apt update
sudo apt install build-essential pkg-config libssl-dev
```

**CentOS/RHEL:**
```bash
# Install development tools
sudo yum groupinstall "Development Tools"
sudo yum install openssl-devel
```

**macOS:**
```bash
# Install Xcode command line tools
xcode-select --install
```

## Contributing to Build System

### Adding New Build Targets

1. Edit `Makefile` to add your target
2. Update this documentation
3. Test on multiple platforms
4. Submit pull request

### Cross-Platform Testing

We recommend testing builds on:
- Ubuntu 20.04 LTS
- CentOS 8
- macOS 12+
- Windows (via WSL2)

For detailed contribution guidelines, see [CONTRIBUTING.md](../CONTRIBUTING.md).
