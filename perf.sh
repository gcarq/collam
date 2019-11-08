#!/bin/bash
set -e

CHANNEL="release"
EXECUTABLE="${1}"

cargo build --release

# Start test executable
perf record -g bash -c "LD_PRELOAD=target/${CHANNEL}/libdmalloc.so ${EXECUTABLE}"