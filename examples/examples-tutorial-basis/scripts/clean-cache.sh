#!/usr/bin/env bash
# `cargo make clean-cache` body: drop the WASM bundles and Rust
# incremental cache so the next `cargo make dev` / `dev-release` rebuilds
# everything from scratch. Used as the first dependency of those
# pipelines to avoid serving stale wasm.
set -euo pipefail

echo "🧹 Cleaning build cache..."

# WASM artifacts
if [ -d "dist-wasm" ]; then
	rm -rf dist-wasm
	echo "  ✓ Removed dist-wasm/"
fi

if [ -d "dist" ]; then
	rm -rf dist
	echo "  ✓ Removed dist/"
fi

# Rust incremental build cache
if [ -d "target/debug/incremental" ]; then
	rm -rf target/debug/incremental
	echo "  ✓ Removed target/debug/incremental/"
fi

# WASM target build cache
if [ -d "target/wasm32-unknown-unknown" ]; then
	rm -rf target/wasm32-unknown-unknown
	echo "  ✓ Removed target/wasm32-unknown-unknown/"
fi

echo "✨ Build cache cleaned"
