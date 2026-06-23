#!/usr/bin/env bash
# Final step of `cargo make dev-release`: mirrors `run-dev-server.sh` but
# starts the release-mode server against the optimised bundle that
# `wasm-build-release` produced. See `run-dev-server.sh` for the rationale
# behind `--noreload` and `--no-override-wasm`.
set -euo pipefail

# Resolve DATABASE_URL the same way the `migrate` / `runserver` tasks do.
eval "$(bash scripts/db_url.sh)"

echo "🚀 Starting server with optimized WASM frontend..."
cargo run --release --bin manage -- runserver --with-pages --noreload --no-override-wasm
