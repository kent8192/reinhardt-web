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

Record at least three runs and report the median of Criterion's point
estimates. This benchmark is the backend/native endpoint complement to the
loopback HTTP runtime comparison: it measures in-process Reinhardt endpoint and
`server_fn` dispatch without the socket and HTTP client costs used by
`cargo make benchmark-runtime-http`. The benchmark keeps using the legacy
`ServerRouter::handle()` entry point so its values remain comparable with the
0.3 baseline; measure `dispatch()` or `try_dispatch_sync()` with a separate
probe when validating those newer fast paths. For automated aggregation, read
`median.point_estimate` from `target/criterion/**/new/estimates.json` after
each run and take the median across runs; ignore Criterion's persisted
`change` lines when comparing separate worktrees.

For disposable remote validation, prefer a GitHub Codespaces
`largePremiumLinux` machine and record the machine type, CPU model, toolchain,
commit, and exact commands with the results. Do not compare remote x86_64 VM
absolute timings directly with local macOS baseline target ranges; re-run the
baseline on the same remote host class when making absolute pass/fail claims.
The detailed remote procedure and the 2026-06-29 UTC Codespaces measurements
are recorded in
[`0.4-performance-scorecard.md`](0.4-performance-scorecard.md).

The 2026-06-29 UTC same-host Codespaces backend run compared
`origin/develop/0.3.0` at `b046f6184b3047010dd184383bd1fbf22dd5e6c7` with the
0.4 integrated head at `292af106f392e12becf3523895cee8e200cb028a` on a
`largePremiumLinux` Codespace with an AMD EPYC 7763 CPU and `rustc 1.96.0`.
Values are the median of three Criterion point estimates from the native
endpoint benchmark:

| Benchmark | Baseline median | Integrated median | Reduction |
|---|---:|---:|---:|
| `http_endpoint_plain_get` | 516.1 ns | 479.1 ns | 7.2% |
| `http_endpoint_path_param_get` | 599.5 ns | 561.1 ns | 6.4% |
| `server_fn_json_post` | 1.208 us | 1.180 us | 2.4% |

A 2026-06-29 UTC backend-only follow-up on the same Codespaces host class
compared `b046f6184b3047010dd184383bd1fbf22dd5e6c7` with
`52067ef9f712d8a60c07b3f4e674ebec4984dad4`. Values are the median of three
Criterion point estimates:

| Benchmark | Baseline median | Final head median | Reduction |
|---|---:|---:|---:|
| `http_endpoint_plain_get` | 488.0 ns | 459.2 ns | 5.9% |
| `http_endpoint_path_param_get` | 601.3 ns | 553.0 ns | 8.0% |
| `server_fn_json_post` | 1.185 us | 943.3 ns | 20.4% |

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
negotiation is not required. The 0.4 server-function dispatch path also keeps
serialized responses as `bytes::Bytes` and writes static typed response
content-type headers. Generated JSON server-function handlers borrow request
bodies directly when extractor or DI parameters do not need body-consumption
state after decoding. Generated `ServerFnRegistration::handle()` implementations
return their concrete future, while the legacy `ServerRouter::handle()` backend
benchmark still includes the outer route `Handler` trait-object box.

## Reinhardt 0.4 Performance Scorecard

The `develop/0.4.0` breaking-change line uses
[`0.4-performance-scorecard.md`](0.4-performance-scorecard.md) as the planning
contract for runtime HTTP, endpoint dispatch, request allocation, and
compile-time feature-graph reductions. Use it before removing compatibility
APIs or claiming cross-framework runtime parity.

### Request Allocation Probe

Use the allocation probe before claiming request-allocation changes:

```bash
cargo run --release -p reinhardt-benchmarks --bin request_alloc_probe
```

Use the percentile probe before claiming p95 or p99 request-latency changes:

```bash
cargo run --release -p reinhardt-benchmarks --bin request_latency_percentile_probe
```

The 2026-06-25 measurement compared `origin/develop/0.3.0` with the same probe
after inlining small path-parameter sets and adding the single-middleware chain
fast path:

The probe uses a current-thread Tokio runtime so RSS measurements track the
request path instead of idle worker-thread stacks.

| Probe | Baseline | Optimized | Reduction |
|---|---:|---:|---:|
| `server_router_static_build_plus_handle` | 6 alloc/request | 6 alloc/request | 0.0% |
| `server_router_two_params_build_plus_handle` | 11 alloc/request | 10 alloc/request | 9.1% |
| `server_router_one_middleware_build_plus_handle` | 15 alloc/request | 12 alloc/request | 20.0% |

The 2026-06-29 measurement compared `develop/0.4.0` with lazy request query
parameter parsing:

| Probe | Baseline | Optimized | Reduction |
|---|---:|---:|---:|
| `request_build_empty_path` | 2 alloc/request | 2 alloc/request | 0.0% |
| `request_build_two_query_params` | 7 alloc/request | 3 alloc/request | 57.1% |
| `clone_for_di_empty_path` | 1 alloc/request | 1 alloc/request | 0.0% |
| `clone_for_di_two_query_params` | 6 alloc/request | 2 alloc/request | 66.7% |

The 2026-06-29 measurement compared `develop/0.4.0` with lazy request extension
backing-store initialization:

| Probe | Baseline | Optimized | Reduction |
|---|---:|---:|---:|
| `request_build_empty_path` | 2 alloc/request | 1 alloc/request | 50.0% |
| `request_build_two_query_params` | 7 alloc/request | 6 alloc/request | 14.3% |
| `direct_handler_build_plus_handle` | 4 alloc/request | 3 alloc/request | 25.0% |
| `direct_handler_handle_only` | 2 alloc/request | 2 alloc/request | 0.0% |
| `clone_for_di_empty_path` | 1 alloc/request | 1 alloc/request | 0.0% |
| `clone_for_di_two_query_params` | 6 alloc/request | 6 alloc/request | 0.0% |
| `server_router_static_build_plus_handle` | 6 alloc/request | 5 alloc/request | 16.7% |
| `server_router_two_params_build_plus_handle` | 10 alloc/request | 9 alloc/request | 10.0% |
| `server_router_one_middleware_build_plus_handle` | 12 alloc/request | 11 alloc/request | 8.3% |

The 0.4 HTTP/1 adapter fast path skips request body collection for `GET` and
`HEAD` requests that declare neither a positive `Content-Length` nor
`Transfer-Encoding`. HTTP/2 keeps the existing collection path because DATA
frames can appear without a `Content-Length`. This reduces loopback HTTP
overhead for the common empty request path, but it is not visible in
`request_alloc_probe` because that probe starts after Hyper has already produced
a Reinhardt `Request`.

The integrated 2026-06-29 0.4 fast-path measurement combines lazy query
parameters with raw `get` lookups, lazy extension backing-store initialization,
empty HTTP/1 body skipping, shared route parameter names, inline path parameter
values, an immutable compiled router table, borrowed matched handlers, and the
concrete router dispatch and synchronous handler fast paths:

| Probe | `develop/0.4.0` baseline | Integrated fast path | Reduction |
|---|---:|---:|---:|
| `request_build_empty_path` | 2 alloc/request | 1 alloc/request | 50.0% |
| `request_build_two_query_params` | 7 alloc/request | 2 alloc/request | 71.4% |
| `direct_handler_build_plus_handle` | 4 alloc/request | 3 alloc/request | 25.0% |
| `direct_handler_handle_only` | 2 alloc/request | 2 alloc/request | 0.0% |
| `clone_for_di_empty_path` | 1 alloc/request | 1 alloc/request | 0.0% |
| `clone_for_di_two_query_params` | 6 alloc/request | 0 alloc/request | 100.0% |
| `server_router_static_build_plus_handle` | 6 alloc/request | 1 alloc/request | 83.3% |
| `server_router_two_params_build_plus_handle` | 10 alloc/request | 1 alloc/request | 90.0% |
| `server_router_one_middleware_build_plus_handle` | 12 alloc/request | 8 alloc/request | 33.3% |

Runtime HTTP scorecard acceptance still requires a low-noise loopback rerun.
During the inline path-parameter measurement, system load was above normal and
all compared frameworks regressed together, so those runtime samples were not
used as acceptance evidence.

The immutable compiled router table removes per-request `RwLock::read()` calls
from method dispatch. It does not change `request_alloc_probe` counts because
the previous lock guard did not allocate; the expected benefit is lower
loopback latency and less contention under concurrent request load.

The zero-child, zero-middleware router path also dispatches directly to the
router's own compiled route table. Static routes avoid cloning empty shared
path-parameter name storage, and the router skips middleware-stack assembly
when no middleware can run. These changes are fixed-cost latency reductions and
do not change the allocation probe counts above.

Server adapters construct `Request` directly from validated Hyper parts instead
of round-tripping through `RequestBuilder`. This bypasses validation branches
and optional-field checks that are useful for public request construction but
redundant after Hyper has already parsed the request.
HTTP/1 and HTTP/2 adapters also share a single request-body plan step, so
empty GET/HEAD requests skip body collection and content-length checks without
duplicating header lookups across the size precheck and collector.

Synchronous route handlers registered through `ServerRouter::endpoint_sync()`
or `ServerRouter::handler_sync()` avoid the async trait-object boxed future
when no middleware is attached. Middleware routes still use the async adapter
because the middleware contract remains `Arc<dyn Handler>`.
Requestless synchronous route handlers registered through
`ServerRouter::endpoint_requestless_sync()` or
`ServerRouter::handler_requestless_sync()` go one step further: the HTTP/1
adapter can serve eligible empty-body routes before constructing a full
`Request`. Use this path only for routes that do not inspect headers, query
strings, path parameters, body bytes, extensions, or DI state.

Follow-up fixed-cost reductions keep `QueryParams::get()` duplicate-key
semantics while scanning cached raw pairs from the end, so last-value lookups
can stop early. Server response conversion also skips status and header-map
mutation when the Reinhardt response is the common `200 OK` response with no
headers.

HTTP/1 and HTTP/2 adapters use Hyper `service_fn` concrete futures instead of a
boxed `Service::Future` on each request. Response conversion also moves the
already-validated Reinhardt `HeaderMap` into the Hyper response instead of
re-inserting every header through `Response::builder`.

`QueryParams::get` caches raw query key/value ranges on first lookup. This keeps
the zero-`HashMap` behavior while avoiding repeated query-string splitting when
a handler reads several known parameters from the same request.

Router matches borrow the compiled route handler and only clone its `Arc` when
middleware composition needs an owned handler. The common no-middleware route
path calls the matched handler through the borrowed compiled-route entry and
dispatches directly to the `dyn Handler`, avoiding the extra `async_trait` box
from the blanket `Arc<T>` handler implementation.
The middleware chain uses the same direct trait-object dispatch for internal
handler calls, reducing one boxed future on skipped middleware and final handler
execution paths.

`ServerRouter::dispatch` exposes the router's concrete request future for
callers that do not need to erase the router behind `dyn Handler`.
`HttpServer::handle_connection_with` accepts a concrete request-handler
closure, so benchmark and embedding code can keep the router concrete and avoid
one `async_trait` boxed future on the HTTP adapter -> router boundary. The
existing `handle_connection` API remains available for arbitrary `Handler`
trait objects.

`Response::with_static_body` sets a response body from `Bytes::from_static`.
Small constant responses such as health checks can avoid copying static string
data into an owned `Bytes` allocation.

## Admin List Query Count Measurements

Use the admin database mock tests before claiming query-count reductions on the
admin list endpoint:

```bash
cargo test -p reinhardt-integration-tests test_list_with_condition_and_count -- --nocapture
```

The non-empty admin list path now uses `COUNT(*) OVER()` to return page rows and
filtered pagination count from one SQL statement. Empty first pages also finish
with the same single query. Empty out-of-range pages still issue the existing
count query as a fallback so pagination metadata remains correct.

| Request shape | Baseline DB calls | Current DB calls | Reduction |
|---|---:|---:|---:|
| Non-empty admin list page | 2 | 1 | 50.0% |
| Empty first admin list page | 2 | 1 | 50.0% |
| Empty out-of-range admin list page | 2 | 2 | 0.0% |

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
