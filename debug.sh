#!/bin/bash
set -e

CHANNEL="release"
TMP_DIR="/tmp/dmalloc-test"

if [ -z "$1" ]; then
    EXECUTABLE="${TMP_DIR}/test"
else
    EXECUTABLE="${1}"
fi

# Cleanup workdir
rm -rf ${TMP_DIR}
mkdir -p ${TMP_DIR}

# Build everything
cargo build --release
gcc test.c -o ${TMP_DIR}/test

# Start debugger
gdb --args env LD_PRELOAD=target/${CHANNEL}/libdmalloc.so "${EXECUTABLE}"