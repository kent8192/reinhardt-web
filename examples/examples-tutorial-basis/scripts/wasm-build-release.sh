#!/usr/bin/env bash
# Post-step for `cargo make wasm-build-release`: mirrors the dev variant,
# copying the optimised release bundle out of `dist-wasm/` into the
# runserver's `--static-dir` via `collectstatic`.
set -euo pipefail

echo "Running collectstatic..."
cargo run --bin manage collectstatic --no-input
echo "✓ WASM release build and collectstatic completed"
