#!/bin/bash
START_DIR=$(pwd)
ROOT=$(git rev-parse --show-toplevel)
cd $ROOT/programs/dex

RUST_LOG= cargo test-bpf
cd $START_DIR