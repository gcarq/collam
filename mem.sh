#!/bin/bash
set -e

CHANNEL="release"
TMP_DIR="/tmp/dmalloc-test"

if [ -z "$1" ]; then
    EXECUTABLE="${TMP_DIR}/mem"
else
    EXECUTABLE="${1}"
fi

# Cleanup workdir
rm -rf ${TMP_DIR}
mkdir -p ${TMP_DIR}

# Build everything
cargo build --features debug --release
gcc mem.c -o ${TMP_DIR}/mem

# Start test executable
time LD_PRELOAD=target/${CHANNEL}/libdmalloc.so "${EXECUTABLE}"
