#!/bin/sh
# Build the Linux dummy game runner.
#
#   ./build.sh                -> target/release/src-linux (cargo) or src-linux-bin (cc)
#   ./build.sh --copy         -> also drop it in ../src-tauri/resources/src-linux
#   FORCE_CC=1 ./build.sh     -> skip cargo, use the system C compiler
#
# Cargo is preferred because CI and the Windows runner use it, but the runner is
# plain C, so a machine with only gcc can still build it. Either way the result
# is a ~15KB binary that links nothing but libc: it gets copied once per fake
# game, so size is the whole point.
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

if [ "${FORCE_CC:-0}" = "1" ]; then
    try_cc
else
    try_cargo || try_cc
fi

echo "built: $(pwd)/$OUT ($(wc -c <"$OUT") bytes)"

if [ "$COPY" = "1" ]; then
    mkdir -p ../src-tauri/resources
    cp "$OUT" ../src-tauri/resources/src-linux
    echo "copied: $(cd .. && pwd)/src-tauri/resources/src-linux"
fi
