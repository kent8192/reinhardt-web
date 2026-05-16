#!/usr/bin/env bash
# Final step of `cargo make dev`: start the development server against the
# wasm bundle that `wasm-build-dev` already produced. The directory check
# guards against running `cargo make dev` from a parent directory — that
# happens to find this Makefile.toml via cargo-make's upward search but
# would still build the wrong project, so fail loudly instead.
#
# Flags:
#   --with-pages         hosts the SPA frontend alongside the API.
#   --noreload           we explicitly skip the runserver's own file
#                        watcher because `dev` already rebuilds the bundle.
#   --no-override-wasm   suppress runserver's internal wasm rebuild so it
#                        does not stomp on the artifacts wasm-build-dev
#                        just placed under `dist/`.
set -euo pipefail

CURRENT_DIR=$(basename "$PWD")
if [ "$CURRENT_DIR" != "examples-tutorial-basis" ]; then
	echo "Error: This command must be run from examples/examples-tutorial-basis directory"
	echo "Current directory: $PWD"
	echo "Please run: cd examples/examples-tutorial-basis && cargo make dev"
	exit 1
fi

echo "🚀 Starting development server with WASM frontend..."
cargo run --bin manage -- runserver --with-pages --noreload --no-override-wasm
