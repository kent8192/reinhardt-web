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
| `incremental-pages-wasm-check` | A Pages runtime WASM check loop |
| `incremental-pages-wasm-build` | A Pages runtime WASM library build loop |
| `incremental-pages-fixture-wasm-build` | A detached browser app fixture build after editing `page!` source |
| `incremental-pages-fixture-hot-patch` | Compile-free static `page!` hot patch loop for the detached browser fixture |
| `incremental-pages-fixture-hot-reload-legacy-both-build` | Legacy detached browser app `page!` edit work shape: fixture WASM build plus native server build |
| `incremental-server-build` | A server crate native build loop |
| `incremental-hot-reload-client-legacy-both-build` | Legacy Pages client-edit hot-reload work shape: WASM build plus native server build |
| `incremental-hot-reload-server-legacy-both-build` | Legacy server-edit hot-reload work shape: native server build plus WASM build |
| `incremental-leaf-build` | Incremental build/link cost for a low-fan-out crate |

## Interpreting Results

Compare every optimization PR against the same baseline report where possible.
For Issue #5218, the target loops are:

- Pure `page!` UI/template edit latency.
- WASM-side Rust logic edit latency.
- Server-only Rust logic edit latency.
- Shared/core/proc-macro edit latency.
- Cold workspace build/check latency.

The current script covers the Rust build side of those loops, including
separate Pages WASM, native server, shared/core, and proc-macro scenarios.
The dev profile is optimized for hot-reload throughput with `debug = 0` and
`codegen-units = 16`; the test profile keeps `debug = 1` for debuggable test
artifacts. Changing these profile settings causes a one-time rebuild of
affected dev artifacts; compare warmed edit loops after that rebuild.
The Pages WASM build scenario currently measures Cargo's library artifact only;
it does not run `wasm-bindgen` against a browser-loadable `cdylib` fixture.
Add that fixture-level scenario before claiming end-to-end browser artifact
latency.
Use `incremental-pages-fixture-wasm-build` when the claim is specifically
about app-side `page!` edits rather than framework runtime edits.
Use `incremental-pages-fixture-hot-patch` for static `page!(|| { ... })`
edits that can be parsed and broadcast through the development HMR channel
without rebuilding the WASM artifact.
When reducing this scenario, prefer removing non-browser modules from the
WASM compilation graph before changing Cargo profiles: profile changes measured
noisy or slower locally, while feature/target gating gives a direct dependency
and codegen reduction.
Browser-visible hot-reload now has a success-gated HMR notification channel for Pages
`runserver --with-pages`: the autoreload parent keeps a WebSocket listener
alive, the child injects the HMR client into SPA HTML, and the watcher sends a
full reload only after the selected rebuild pipelines succeed. Reloads that
depend on a native server respawn also wait briefly for the child server TCP
address to become reachable. Runtime measurements still need to be added before
claiming browser-visible latency numbers.
The default debounce window for the autoreload watcher is 120 ms and can be
overridden with `runserver --watch-delay <milliseconds>`. Include the effective
debounce value when reporting browser-visible tail-latency measurements because
it is part of the request-to-HMR critical path.
This default reduces the fixed debounce component by 180 ms versus the previous
300 ms watcher default. On the Issue #5218 summarized loops, that maps to about
9-10% less fixed latency for Pages client-edit loops and about 18-25% for
server-only loops before any rebuild-time variance is considered.

Static `page!(|| { ... })` edits under WASM-owned client source paths can use
the compile-free development hot patch path. The HMR payload replaces `#app`
contents, preserving the development script and page shell. Dynamic Rust
expressions, event handlers, control flow, components, and shared/server-owned
files still fall back to the normal rebuild path. The Pages macro also emits
batched attribute builders instead of one chained method call per generated
attribute, which reduces generated Rust for attribute-heavy templates when a
rebuild is still required.

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
