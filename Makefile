data_scripts_dir := probing-0.1.4.data/scripts/
ifndef DEBUG
	CARGO_FLAGS := -r
	TARGET_DIR := release
else
	CARGO_FLAGS :=
	TARGET_DIR := debug
endif

ifndef ZIG
	CARGO_BUILD_CMD := build
	MATURIN_FLAGS :=
	TARGET_DIR_PREFIX := target
else
	CARGO_BUILD_CMD := zigbuild --target x86_64-unknown-linux-gnu.2.17
	MATURIN_FLAGS := --zig
	TARGET_DIR_PREFIX := target/x86_64-unknown-linux-gnu
endif

all: wheel

wheel: ${TARGET_DIR_PREFIX}/${TARGET_DIR}/probing ${TARGET_DIR_PREFIX}/${TARGET_DIR}/libprobing.so
	rm -rf dist
	maturin build -r ${MATURIN_FLAGS}
	test -e dist || mkdir -p dist
	cp target/wheels/*.whl dist/

.PHONY: app/dist
app/dist:
	test -e app/dist || mkdir -p app/dist
	cd app && trunk build --filehash false --release -M -d dist/
	cd ..

${data_scripts_dir}:
	test -e ${data_scripts_dir}|| mkdir -p ${data_scripts_dir}

.PHONY: ${TARGET_DIR_PREFIX}/${TARGET_DIR}/probing
${TARGET_DIR_PREFIX}/${TARGET_DIR}/probing: ${data_scripts_dir}
	cargo ${CARGO_BUILD_CMD} ${CARGO_FLAGS} --package probing-cli 
	test -e ${data_scripts_dir} || mkdir -p ${data_scripts_dir}
	cp ${TARGET_DIR_PREFIX}/${TARGET_DIR}/probing ${data_scripts_dir}

.PHONY: ${TARGET_DIR_PREFIX}/${TARGET_DIR}/libprobing.so
${TARGET_DIR_PREFIX}/${TARGET_DIR}/libprobing.so: ${data_scripts_dir} app/dist
	cargo ${CARGO_BUILD_CMD} ${CARGO_FLAGS}
	cp ${TARGET_DIR_PREFIX}/${TARGET_DIR}/libprobing.so ${data_scripts_dir}


