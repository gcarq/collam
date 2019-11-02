#!/bin/bash
set -e

CHANNEL="release"
EXECUTABLE="${1}"

# Start test executable
perf record -g bash -c "LD_PRELOAD=target/${CHANNEL}/libdmalloc.so ${EXECUTABLE}"