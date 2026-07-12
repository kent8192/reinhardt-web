#!/usr/bin/env bash
set -euo pipefail

echo "Running collectstatic..."
cargo run --bin manage collectstatic --no-input
mkdir -p dist
find dist-wasm -maxdepth 1 -type f \( -name '*.js' -o -name '*.wasm' -o -name '*.d.ts' \) -exec cp -f {} dist/ \;

if command -v wasm-opt >/dev/null 2>&1; then
	echo "Running wasm-opt..."
	WASM_BG=$(find dist -maxdepth 1 -type f -name '*_bg.wasm' -print -quit)
	if [ -n "$WASM_BG" ]; then
		wasm-opt -O3 "$WASM_BG" -o "$WASM_BG.opt"
		mv "$WASM_BG.opt" "$WASM_BG"
		echo "✓ WASM optimized"
	fi
else
	echo "⚠️  wasm-opt not found, skipping optimization"
fi

echo "✓ WASM release build and collectstatic completed"
