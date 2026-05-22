#!/usr/bin/env bash
# Optional post-step for `cargo make wasm-build-release`: shrink the
# release wasm bundle with `wasm-opt -O3` when the Binaryen toolchain is
# available. Falls back to a no-op (with a warning) if wasm-opt is not on
# PATH so the release build still completes on machines without
# Binaryen installed.
set -euo pipefail

if command -v wasm-opt &> /dev/null; then
	echo "Running wasm-opt..."
	WASM_FILE="dist-wasm/examples_tutorial_basis_bg.wasm"
	wasm-opt -O3 -o "$WASM_FILE.opt" "$WASM_FILE"
	mv "$WASM_FILE.opt" "$WASM_FILE"
	echo "✓ WASM optimized"
else
	echo "⚠️  wasm-opt not found, skipping optimization"
fi
