# Build Performance Measurements

This directory stores reproducible build-loop measurements for Reinhardt.
Use these measurements before claiming compile-time or hot-reload latency
improvements.

## Quick Start

Preview the benchmark commands without running them:

```bash
cargo make bench-builds-dry-run
```

Run the standard benchmark suite:

```bash
cargo make bench-builds
```

Run a focused scenario:

```bash
scripts/bench-build.sh incremental-leaf-check
```

`scripts/bench-build.sh` writes a JSON report under `docs/build-perf/` and
an adjacent `.env.txt` file with the Rust version, host, and cache-relevant
environment variables.

## Scenario Meaning

| Scenario | Measures |
|---|---|
| `cold-check-all-features` | Full all-features workspace check after `cargo clean` |
| `cold-build-standard` | Default workspace build after `cargo clean` |
| `cold-test-build` | Test binary build/link cost before tests execute |
| `incremental-leaf-check` | A low-fan-out crate edit loop |
| `incremental-core-check` | A shared core edit loop |
| `incremental-db-macro-check` | A proc-macro fan-out edit loop |
| `incremental-page-macro-check` | A Pages macro fan-out edit loop |
| `incremental-leaf-build` | Incremental build/link cost for a low-fan-out crate |

## Interpreting Results

Compare every optimization PR against the same baseline report where possible.
For Issue #5218, the target loops are:

- Pure `page!` UI/template edit latency.
- WASM-side Rust logic edit latency.
- Server-only Rust logic edit latency.
- Shared/core/proc-macro edit latency.
- Cold workspace build/check latency.

The current script covers the Rust build side of those loops. Browser-visible
hot-reload latency still needs a runtime benchmark once the reload notification
channel and dev-mode `page!` path exist.

## Hot-Reload Target Selection

The autoreload watcher classifies debounced paths before dispatching rebuilds.
For Pages projects, `src/client.rs`, `src/client/**`, and
`src/apps/*/client/**` are treated as WASM-only; `src/bin/**` plus server
process configuration are treated as server-only; shared code, manifests, and
lockfiles still rebuild both sides. Keep this conservative unless a new path is
proved to be target-specific by generated project structure and tests.
