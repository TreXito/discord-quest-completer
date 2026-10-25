#!/bin/sh
# Build the Linux dummy game runner.
#
#   ./build.sh                -> src-linux-bin (cc) or target/release/src-linux (cargo)
#   ./build.sh --copy         -> also drop it in ../src-tauri/resources/src-linux
#   FORCE_CC=1 ./build.sh     -> cc only, no cargo fallback
#   FORCE_CARGO=1 ./build.sh  -> cargo first (what CI used historically)
#
# cc is preferred on Linux: the whole point of this binary is that it is a tiny
# libc-only blob (a copy is made once per fake game), and a C build gives ~11KB
# while the Rust wrapper is ~265KB and additionally links libgcc_s. Cargo is kept
# as the fallback for machines without a C compiler, since the runner is plain C.
set -eu

cd "$(dirname "$0")"

COPY=0
for arg in "$@"; do
    case "$arg" in
        --copy) COPY=1 ;;
        *) echo "unknown argument: $arg" >&2; exit 2 ;;
    esac
done

OUT=""

try_cargo() {
    command -v cargo >/dev/null 2>&1 || return 1
    # cargo may be a stub that refuses to compile on constrained hosts
    if ! cargo --version >/dev/null 2>&1; then
        echo "cargo present but not usable, falling back to the C compiler" >&2
        return 1
    fi
    cargo build --release
    OUT=target/release/src-linux
}

try_cc() {
    CC=${CC:-}
    if [ -z "$CC" ]; then
        for candidate in cc gcc clang; do
            if command -v "$candidate" >/dev/null 2>&1; then
                CC=$candidate
                break
            fi
        done
    fi
    if [ -z "$CC" ]; then
        echo "no C compiler found (and cargo did not work)" >&2
        return 1
    fi
    "$CC" -Os -s -fno-ident -ffunction-sections -fdata-sections \
        -Wl,--gc-sections -o src-linux-bin src/main.c
    OUT=src-linux-bin
}

if [ "${FORCE_CARGO:-0}" = "1" ]; then
    try_cargo || try_cc
elif [ "${FORCE_CC:-0}" = "1" ]; then
    try_cc
else
    try_cc || try_cargo
fi

echo "built: $(pwd)/$OUT ($(wc -c <"$OUT") bytes)"

if [ "$COPY" = "1" ]; then
    mkdir -p ../src-tauri/resources
    cp "$OUT" ../src-tauri/resources/src-linux
    echo "copied: $(cd .. && pwd)/src-tauri/resources/src-linux"
fi
