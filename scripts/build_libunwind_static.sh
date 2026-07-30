#!/usr/bin/env bash
set -euo pipefail

LIBUNWIND_VERSION="1.8.3"
LIBUNWIND_SHA256="be30d910e67f58d82e753231f1357f326a1a088acf126b21ff77e60aab19b90b"
LIBUNWIND_URL="https://github.com/libunwind/libunwind/releases/download/v${LIBUNWIND_VERSION}/libunwind-${LIBUNWIND_VERSION}.tar.gz"
TARGET_TRIPLE="x86_64-linux-gnu.2.17"

BUILD_ROOT="${CARGO_TARGET_DIR:-target}"
PREFIX="${PROBING_LIBUNWIND_PREFIX:-${BUILD_ROOT}/libunwind-${LIBUNWIND_VERSION}-${TARGET_TRIPLE}}"
CACHE_DIR="${PROBING_LIBUNWIND_CACHE_DIR:-${BUILD_ROOT}/vendor}"
ARCHIVE="${PROBING_LIBUNWIND_ARCHIVE:-${CACHE_DIR}/libunwind-${LIBUNWIND_VERSION}.tar.gz}"
STAMP="${PREFIX}/.probing-libunwind-${LIBUNWIND_VERSION}-${TARGET_TRIPLE}"

if [[ -f "${STAMP}" \
    && -f "${PREFIX}/lib/libunwind-x86_64.a" \
    && -f "${PREFIX}/lib/libunwind.a" ]]; then
    printf 'Using cached static libunwind: %s\n' "${PREFIX}"
    exit 0
fi

if [[ ! -f "${ARCHIVE}" ]]; then
    mkdir -p "${CACHE_DIR}"
    if command -v curl >/dev/null 2>&1; then
        curl -L --fail --retry 3 -o "${ARCHIVE}" "${LIBUNWIND_URL}"
    elif command -v wget >/dev/null 2>&1; then
        wget -O "${ARCHIVE}" "${LIBUNWIND_URL}"
    else
        printf 'Neither curl nor wget is available to fetch %s\n' "${LIBUNWIND_URL}" >&2
        exit 1
    fi
fi

actual_sha256="$(sha256sum "${ARCHIVE}" | awk '{print $1}')"
if [[ "${actual_sha256}" != "${LIBUNWIND_SHA256}" ]]; then
    printf 'Invalid libunwind archive checksum: expected %s, got %s\n' \
        "${LIBUNWIND_SHA256}" "${actual_sha256}" >&2
    exit 1
fi

if [[ -n "${CARGO_ZIGBUILD_ZIG_PATH:-}" && -x "${CARGO_ZIGBUILD_ZIG_PATH}" ]]; then
    ZIG_BIN="${CARGO_ZIGBUILD_ZIG_PATH}"
elif command -v zig >/dev/null 2>&1; then
    ZIG_BIN="$(command -v zig)"
else
    ZIG_BIN="$(
        python3 -c \
            'import pathlib, ziglang; print(pathlib.Path(ziglang.__file__).parent / "zig")'
    )"
fi

if [[ ! -x "${ZIG_BIN}" ]]; then
    printf 'Unable to find Zig. Install it with: python3 -m pip install ziglang\n' >&2
    exit 1
fi

BUILD_TEMP="$(mktemp -d "${TMPDIR:-/tmp}/probing-libunwind.XXXXXXXX")"
cleanup() {
    rm -rf "${BUILD_TEMP}"
}
trap cleanup EXIT

tar -xzf "${ARCHIVE}" -C "${BUILD_TEMP}"
mkdir -p "${BUILD_TEMP}/build" "${PREFIX}"

pushd "${BUILD_TEMP}/build" >/dev/null
CC="${ZIG_BIN} cc -target ${TARGET_TRIPLE}" \
CXX="${ZIG_BIN} c++ -target ${TARGET_TRIPLE}" \
CFLAGS="-O2 -fPIC" \
CXXFLAGS="-O2 -fPIC" \
"${BUILD_TEMP}/libunwind-${LIBUNWIND_VERSION}/configure" \
    --host=x86_64-linux-gnu \
    --prefix="$(cd "${PREFIX}" && pwd)" \
    --disable-shared \
    --enable-static \
    --disable-tests \
    --disable-documentation \
    --disable-minidebuginfo \
    --disable-zlibdebuginfo \
    --disable-coredump \
    --disable-ptrace \
    --disable-setjmp

make -j"${PROBING_BUILD_JOBS:-$(getconf _NPROCESSORS_ONLN)}"
make install
popd >/dev/null

touch "${STAMP}"
printf 'Built static libunwind: %s\n' "${PREFIX}"
