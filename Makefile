# ==============================================================================
# Variables
# ==============================================================================
# Build mode: release (default) or debug
ifndef DEBUG
	CARGO_FLAGS := -r
	TARGET_DIR := release
else
	CARGO_FLAGS :=
	TARGET_DIR := debug
endif

# OS-specific library extension
ifeq ($(shell uname -s), Darwin)
	LIB_EXT := dylib
else
	LIB_EXT := so
endif

# Cargo build command: normal (default) or zigbuild
ifndef ZIG
	CARGO_BUILD_CMD := build
	TARGET_DIR_PREFIX := target
else
	CARGO_BUILD_CMD := zigbuild --target x86_64-unknown-linux-gnu.2.17
	TARGET_DIR_PREFIX := target/x86_64-unknown-linux-gnu
endif

# Python version
PYTHON ?= 3.12

# Paths
DATA_SCRIPTS_DIR := python/probing
PROBING_CLI := ${TARGET_DIR_PREFIX}/${TARGET_DIR}/probing
PROBING_LIB := ${TARGET_DIR_PREFIX}/${TARGET_DIR}/libprobing.${LIB_EXT}

# Pytest runner command
PYTEST_RUN := PYTHONPATH=python/ uv run --python ${PYTHON} -w pytest -w websockets -w pandas -w torch -w ipykernel -- python -m pytest --doctest-modules

# ==============================================================================
# Standard Targets
# ==============================================================================
.PHONY: all
all: wheel

.PHONY: help
help:
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  all        Build the wheel (default)."
	@echo "  wheel      Build the Python wheel."
	@echo "  test       Run Rust tests."
	@echo "  pytest     Run Python tests."
	@echo "  bootstrap  Install Python versions for testing."
	@echo "  clean      Remove build artifacts."
	@echo "  app/dist   Build the web app."

# ==============================================================================
# Build Targets
# ==============================================================================
.PHONY: wheel
wheel: ${PROBING_CLI} ${PROBING_LIB}
	@echo "Building wheel..."
	python make_wheel.py

.PHONY: app/dist
app/dist:
	@echo "Building web app..."
	@mkdir -p app/dist
	cd app && trunk build --filehash false --release -M -d dist/
	cd ..

${DATA_SCRIPTS_DIR}:
	@echo "Creating data scripts directory..."
	@mkdir -p ${DATA_SCRIPTS_DIR}

.PHONY: ${PROBING_CLI}
${PROBING_CLI}: ${DATA_SCRIPTS_DIR}
	@echo "Building probing CLI..."
	cargo ${CARGO_BUILD_CMD} ${CARGO_FLAGS} --package probing-cli
	cp ${PROBING_CLI} ${DATA_SCRIPTS_DIR}/probing

.PHONY: ${PROBING_LIB}
${PROBING_LIB}: ${DATA_SCRIPTS_DIR} app/dist
	@echo "Building probing library..."
	cargo ${CARGO_BUILD_CMD} ${CARGO_FLAGS}
	cp ${PROBING_LIB} ${DATA_SCRIPTS_DIR}/libprobing.${LIB_EXT}

# ==============================================================================
# Testing & Utility Targets
# ==============================================================================
.PHONY: test
test:
	@echo "Running Rust tests..."
	cargo nextest run --workspace --no-default-features --nff

.PHONY: bootstrap
bootstrap:
	@echo "Bootstrapping Python environments..."
	uv python install 3.8 3.9 3.10 3.11 3.12 3.13

.PHONY: pytest
pytest:
	@echo "Running pytest for probing package..."
	${PYTEST_RUN} python/probing
	@echo "Running pytest for tests directory..."
	${PYTEST_RUN} tests

.PHONY: clean
clean:
	@echo "Cleaning up..."
	rm -rf dist
	rm -rf app/dist
	cargo clean
