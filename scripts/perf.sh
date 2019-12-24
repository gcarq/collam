#!/bin/bash
set -e

CHANNEL="release"
TMP_DIR="/tmp/collam-test"

if [ -z "$1" ]; then
    EXECUTABLE="${TMP_DIR}/test"
else
    EXECUTABLE="${1}"
fi

# Cleanup workdir
rm -rf ${TMP_DIR}
mkdir -p ${TMP_DIR}

# Build everything
cargo build --manifest-path posix/Cargo.toml --release
gcc test.c -o ${TMP_DIR}/test

# Start test executable
perf record -g bash -c "LD_PRELOAD=\"$(pwd)/posix/target/${CHANNEL}/libcollam.so\" ${EXECUTABLE}"