#!/usr/bin/env bash
# Post-step for `cargo make wasm-build-dev`: after `wasm-pack` writes the
# debug bundle into `dist-wasm/`, copy it (plus other static assets) into
# the runserver's `--static-dir`. `--no-input` skips the interactive
# overwrite prompt so this runs cleanly in cargo-make.
set -euo pipefail

echo "Running collectstatic..."
cargo run --bin manage collectstatic --no-input
echo "✓ WASM build and collectstatic completed"
