#!/bin/sh
set -e

wasm-pack test --headless --chrome --package puzzle-explorer-math "$@"
wasm-pack test --headless --chrome "$@"

