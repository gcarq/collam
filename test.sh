#/bin/bash

set -e

CHANNEL="release"
TMP_DIR="/tmp/dmalloc-test"

# Cleanup workdir
rm -rf ${TMP_DIR}
mkdir -p ${TMP_DIR}

# Build everything
cargo build --release
gcc test.c -o ${TMP_DIR}/test

# Start test executable
LD_PRELOAD=target/${CHANNEL}/libdmalloc.so ${TMP_DIR}/test