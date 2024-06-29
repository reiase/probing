data_scripts_dir := probe-0.1.0.data/scripts/
ifndef DEBUG
	CARGO_FLAGS := "-r"
	TARGET_DIR := target/release
else
	CARGO_FLAGS :=
	TARGET_DIR := target/debug
endif

app_src := $(wildcard app/src/**.rs)

all: ${TARGET_DIR}/probe ${TARGET_DIR}/libprobe.so wheel

wheel: ${TARGET_DIR}/probe ${TARGET_DIR}/libprobe.so
	maturin build -r --zig

.PHONY: dist
dist:
	test -e dist || mkdir dist
	cd app && trunk build --filehash false --release -M -d ../dist/
	cd ..

${data_scripts_dir}:
	test -e ${data_scripts_dir}|| mkdir -p ${data_scripts_dir}

.PHONY: ${TARGET_DIR}/probe
${TARGET_DIR}/probe: ${data_scripts_dir}
	cargo build ${CARGO_FLAGS} --package probe-cli
	test -e ${data_scripts_dir} || mkdir -p ${data_scripts_dir}
	cp ${TARGET_DIR}/probe ${data_scripts_dir}

.PHONY: ${TARGET_DIR}/libprobe.so
${TARGET_DIR}/libprobe.so: ${data_scripts_dir} dist
	cargo build ${CARGO_FLAGS}
	cp ${TARGET_DIR}/libprobe.so ${data_scripts_dir}

.PHONY: test
test:
	echo ${DEBUG} "," ${CARGO_FLAGS}
	echo ${app_src}

