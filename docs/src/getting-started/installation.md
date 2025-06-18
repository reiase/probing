# Installation

This guide provides instructions on how to install Probing on your system.

## Prerequisites

Before you begin, ensure you have the following:

- Python (version 3.7 or higher)
- Pip (Python package installer)
- For building from source:
    - Rust (latest stable version recommended)
    - Cargo (Rust's package manager and build system)

## Installation Methods

You can install Probing using one of the following methods:

### 1. Using Pip (Recommended)

This is the easiest way to install Probing:

```bash
pip install probing
```

This command will download and install the latest stable release of Probing from the Python Package Index (PyPI).

### 2. Building from Source

If you want the latest development version or want to contribute to Probing, you can build it from source:

```bash
# 1. Clone the repository
git clone https://github.com/reiase/probing.git
cd probing

# 2. Build and install the Python package
make wheel
pip install target/wheels/probing-*.whl
```

This will compile the Rust components and build the Python wheel for installation.

For detailed instructions on building from source, including prerequisites and troubleshooting, see the [Building from Source](../development/building.md) guide.

## Verifying the Installation

After installation, you can verify that Probing is correctly installed by running:

```bash
probing --version
```

This should print the installed version of Probing, for example:

```
probing 0.1.0
```

You can also check if the `probing` command is available:
```bash
probing list
```
This command should list available probing commands or indicate that no processes are currently being probed.

## Next Steps

With Probing installed, you are ready to start using it. Head back to the [Introduction](introduction.md) to learn about its core capabilities and how to get started with your first analysis.
