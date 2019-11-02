#!/bin/bash

perf report -v --symbol-filter=dmalloc --max-stack=255
