#!/usr/bin/env bash
set -euo pipefail

echo "Running collectstatic..."
cargo run --bin manage collectstatic --no-input
mkdir -p dist
find dist-wasm -maxdepth 1 -type f \( -name '*.js' -o -name '*.wasm' -o -name '*.d.ts' \) -exec cp -f {} dist/ \;
echo "WASM build and collectstatic completed"
