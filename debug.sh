#/bin/bash

CHANNEL="debug"
TMP_DIR="/tmp/dmalloc-test"

# Cleanup workdir
rm -rf ${TMP_DIR}
mkdir -p ${TMP_DIR}

# Build everything
cargo build
gcc test.c -o ${TMP_DIR}/test

# Start debugger
gdb --args env LD_PRELOAD=target/${CHANNEL}/libdmalloc.so ${TMP_DIR}/test