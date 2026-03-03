#!/bin/sh
set -e

wasm-pack build --target web "$@"

# Generate cache-busting version file in pkg/ (gitignored)
GIT_HASH=$(git rev-parse --short HEAD)
echo "export const BUILD_HASH = '${GIT_HASH}';" > pkg/build_hash.js
sed "s/__BUILD_HASH__/${GIT_HASH}/g" worker_template.js > pkg/worker.js

echo "Build complete. Run a local server to view index.html"
echo "Example: python3 -m http.server"
