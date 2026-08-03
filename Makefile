data_scripts_dir := probing.data/scripts/
ifndef DEBUG
	CARGO_FLAGS := -r
	TARGET_DIR := release
else
	CARGO_FLAGS :=
	TARGET_DIR := debug
endif

ifndef ZIG
	CARGO_BUILD_CMD := build
	CARGO_BUILD_ENV :=
	TARGET_DIR_PREFIX := target
else
	CARGO_TARGET_DIR ?= target/probing-zig-x86_64-linux-gnu.2.17
	CARGO_BUILD_CMD := zigbuild --target x86_64-unknown-linux-gnu.2.17
	CARGO_BUILD_ENV := CARGO_TARGET_DIR=$(abspath $(CARGO_TARGET_DIR))
	TARGET_DIR_PREFIX := $(CARGO_TARGET_DIR)/x86_64-unknown-linux-gnu
endif

LIBUNWIND_VERSION := 1.8.3
LIBUNWIND_PREFIX := target/libunwind-$(LIBUNWIND_VERSION)-x86_64-linux-gnu.2.17
LIBUNWIND_STATIC_DIR := $(abspath $(LIBUNWIND_PREFIX)/lib)
LIBUNWIND_STAMP := $(LIBUNWIND_PREFIX)/.probing-libunwind-$(LIBUNWIND_VERSION)-x86_64-linux-gnu.2.17

all: wheel

wheel: ${TARGET_DIR_PREFIX}/${TARGET_DIR}/probing ${TARGET_DIR_PREFIX}/${TARGET_DIR}/libprobing.so
	rm dist/* -rf
	PROBING_BUILD_DIR=$(abspath ${TARGET_DIR_PREFIX}/${TARGET_DIR}) python make_wheel.py

.PHONY: app/dist
app/dist:
	test -e app/dist || mkdir -p app/dist
	cd app && trunk build --filehash false --release -M -d dist/
	cd ..

${data_scripts_dir}:
	test -e ${data_scripts_dir}|| mkdir -p ${data_scripts_dir}

.PHONY: ${TARGET_DIR_PREFIX}/${TARGET_DIR}/probing
${TARGET_DIR_PREFIX}/${TARGET_DIR}/probing: ${data_scripts_dir}
	${CARGO_BUILD_ENV} cargo ${CARGO_BUILD_CMD} ${CARGO_FLAGS} --package probing-cli
	test -e ${data_scripts_dir} || mkdir -p ${data_scripts_dir}
	cp ${TARGET_DIR_PREFIX}/${TARGET_DIR}/probing ${data_scripts_dir}

$(LIBUNWIND_STAMP): scripts/build_libunwind_static.sh
	PROBING_LIBUNWIND_PREFIX=$(abspath $(LIBUNWIND_PREFIX)) ./scripts/build_libunwind_static.sh

.PHONY: ${TARGET_DIR_PREFIX}/${TARGET_DIR}/libprobing.so
${TARGET_DIR_PREFIX}/${TARGET_DIR}/libprobing.so: ${data_scripts_dir} app/dist $(LIBUNWIND_STAMP)
	${CARGO_BUILD_ENV} PROBING_LIBUNWIND_STATIC_DIR=$(LIBUNWIND_STATIC_DIR) cargo ${CARGO_BUILD_CMD} ${CARGO_FLAGS}
	cp ${TARGET_DIR_PREFIX}/${TARGET_DIR}/libprobing.so ${data_scripts_dir}

.PHONY: test
test:
	cargo nextest run --workspace --no-default-features --nff
