#!/bin/bash

perf report -v --symbol-filter=collam --max-stack=255
