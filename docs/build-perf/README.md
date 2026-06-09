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
hot-reload now has a success-gated HMR notification channel for Pages
`runserver --with-pages`: the autoreload parent keeps a WebSocket listener
alive, the child injects the HMR client into SPA HTML, and the watcher sends a
full reload only after the selected rebuild pipelines succeed. Reloads that
depend on a native server respawn also wait briefly for the child server TCP
address to become reachable. Runtime measurements still need to be added before
claiming browser-visible latency numbers.

Pure `page!` edits still compile as Rust today. A compile-free dev-mode
template path is a separate architecture change and should not be implied by
the current HMR notification channel.

## Hot-Reload Target Selection

The autoreload watcher classifies debounced paths before dispatching rebuilds.
For Pages projects, `src/client.rs`, `src/client/**`, and
`src/apps/*/client/**` are treated as WASM-only; `src/bin/**` plus server
process configuration are treated as server-only; shared code, manifests, and
lockfiles still rebuild both sides. Keep this conservative unless a new path is
proved to be target-specific by generated project structure and tests.

## Browser Reload Notification

When Pages autoreload is enabled, `runserver` starts a stable HMR WebSocket
listener in the autoreload parent process. The spawned `--noreload` child
receives the listener port through `REINHARDT_HMR_PORT` and injects the
framework-owned HMR script into SPA fallback HTML. This keeps the browser
connected across native server restarts.

The watcher broadcasts `full_reload` only after all rebuild targets selected
for the change batch succeed:

- WASM-only edits reload after the WASM bundle rebuild succeeds.
- Server-only edits reload after the native binary rebuilds, respawns, and the
  server address accepts TCP.
- Shared edits reload only after both selected pipelines succeed and the
  respawned server is reachable.
- Failed rebuilds keep the old page running and wait for the next change.

`runserver --with-pages --no-override-wasm` reuses existing Pages artifacts
only when `dist/<crate>_bg.wasm` is newer than tracked source files. A plain
`dist/<crate>.js` existence check is not enough: it can serve stale WASM after a
Rust edit and hides the real feedback-loop cost.
