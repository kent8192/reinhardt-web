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
300 ms watcher default. Using the Issue #5218 summarized means as p50 proxies,
that maps to estimated p50 reductions of about 59% for compile-free static
`page!` hot patches, 21% for app fixture WASM builds, 18% for server-only
rebuilds, and 9% for framework Pages WASM builds. If p95 rebuild cost is 10-20%
above those means, the same fixed debounce cut maps to about 18-20% p95 for app
fixture WASM builds, 16-17% p95 for server-only rebuilds, and 8% p95 for
framework Pages WASM builds. Treat these as planning estimates until
browser-visible p50/p95 runtime measurements are added.

## Pages WASM Binary Size Measurements

Use a detached browser fixture release build before claiming browser artifact
size improvements:

```bash
cargo build --manifest-path crates/reinhardt-pages/tests/fixtures/spa_navigation_app/Cargo.toml --target wasm32-unknown-unknown --release
stat -f '%N %z bytes' target/wasm32-unknown-unknown/release/spa_navigation_app.wasm
```

The 2026-06-25 measurement compared `origin/develop/0.3.0` with the same
fixture after applying a size-oriented release profile and excluding
`console_error_panic_hook` from release startup. Values are raw `.wasm` bytes
before `wasm-bindgen` or `wasm-opt` post-processing.

| Fixture | Baseline | Optimized | Reduction |
|---|---:|---:|---:|
| `spa_navigation_app.wasm` | 2,541,068 bytes | 1,879,383 bytes | 26.0% |

The optimized fixture uses `opt-level = "z"`, `lto = true`,
`codegen-units = 1`, `panic = "abort"`, and `strip = true` for release builds.
The development panic hook remains available in debug builds.

## Migration Graph Plan Measurements

Use a focused migration-graph probe before claiming plan-generation wins. The
2026-06-25 probe built a 10,000-migration graph with 100 apps, diamond
dependencies within each app, and cross-app dependencies every tenth migration,
then measured `MigrationGraph::topological_sort()` five times in release mode.

| Version | Median | Per migration | Reduction |
|---|---:|---:|---:|
| `origin/develop/0.3.0` | 777.38 ms | 77.74 us | baseline |
| adjacency-list dependents | 6.12 ms | 611 ns | 99.2% |

This measures the graph ordering phase only. It does not include database
introspection, schema validation, or migration execution.

## Native Endpoint Runtime Measurements

Use the native endpoint benchmark before claiming request-dispatch or
server-function hot-path improvements:

```bash
cargo bench -p reinhardt-pages --bench server_fn_endpoint_benchmarks -- --sample-size 30 --measurement-time 2
```

The 2026-06-25 measurement compared `origin/develop/0.3.0` with only the
benchmark harness applied against the optimized branch. Values are the mean of
two Criterion runs on the same host; Criterion's persisted `change` lines were
ignored because each worktree had independent historical samples.

| Benchmark | Baseline mean | Optimized mean | Reduction |
|---|---:|---:|---:|
| `http_endpoint_plain_get` | 343.38 ns | 276.47 ns | 19.5% |
| `http_endpoint_path_param_get` | 414.14 ns | 337.18 ns | 18.6% |
| `server_fn_json_post` | 915.86 ns | 611.51 ns | 33.2% |

The optimized path avoids per-request path-string allocations when the input
path is already normalized, avoids allocating alternate trailing-slash vectors,
skips response-cookie jar creation for body-only server functions, and
deserializes JSON server-function requests directly from bytes when content
negotiation is not required.

Static `page!(|| { ... })` edits under WASM-owned client source paths can use
the compile-free development hot patch path. The HMR payload replaces `#app`
contents, preserving the development script and page shell. Dynamic Rust
expressions, event handlers, control flow, components, and shared/server-owned
files still fall back to the normal rebuild path. The Pages macro also emits
batched attribute builders instead of one chained method call per generated
attribute, which reduces generated Rust for attribute-heavy templates when a
rebuild is still required.

## Develop 0.3.0 Browser-WASM Pruning

The browser-WASM `reinhardt-web --features pages` check path must not pull
native build-time tooling or an HTTP client abstraction that duplicates the
browser Fetch API. The 2026-06-24 dependency-pruning pass removed the root
package's unused `tonic-prost-build` build script path and replaced the
`reinhardt-pages` client-side `reqwest` usage with a small internal Fetch API
wrapper.

Measured against `origin/develop/0.3.0` at `0a60cfc3d3` with fresh
`CARGO_TARGET_DIR` and `CARGO_BUILD_BUILD_DIR` directories, local
`rustc-wrapper` disabled, and the browser-WASM check path warmed by one prior
run:

| Measurement | Baseline | Current | Reduction |
|---|---:|---:|---:|
| `cargo check -p reinhardt-web --no-default-features --features pages --target wasm32-unknown-unknown` with empty build output dirs | 28.79s | 22.00s | 23.6% |
| Unique normal/build dependency nodes for the same target | 140 | 111 | 20.7% |

The removed browser-WASM dependency nodes are `reqwest`, `sync_wrapper`, and the
root build path's `prost-build`/`tonic-build`/`tonic-prost-build` support
stack. Keep future browser-WASM additions on native browser APIs unless a
cross-target abstraction is required by generated user code.

Combining this measured browser-WASM pruning with the 120 ms autoreload debounce
default gives the following browser-visible tail-latency planning estimates:

| Edit loop | Baseline tail | Current tail | Estimated reduction |
|---|---:|---:|---:|
| Static `page!` hot patch | 305 ms | 125 ms | 59.0% |
| App fixture WASM build | 864 ms | 684 ms | 20.8% |
| Server-only rebuild | 1.010 s | 0.830 s | 17.8% |
| Browser-WASM `reinhardt-web --features pages` check | 29.09 s | 22.12 s | 24.0% |
| Pages client rebuild, applying the measured 23.6% browser-WASM pruning ratio to the 1.753 s build component | 2.053 s | 1.459 s | 28.9% |

The combined numbers are estimates for browser-visible tails rather than a live
browser timing run: they add the fixed watcher debounce to the measured build or
hot-patch means. The static patch, app fixture, server-only, and browser-WASM
check rows use directly measured component timings; the Pages client rebuild row
projects the measured browser-WASM pruning ratio onto the client rebuild
component to size the rebuild-heavy tail-latency opportunity.

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
