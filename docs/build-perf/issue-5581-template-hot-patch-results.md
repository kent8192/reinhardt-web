# Issue #5581 Template Hot-Patch Measurements

## Status

The deterministic parser/HMR-payload proxy is implemented and measured. A
real Cocrea `runserver` plus browser-visible latency measurement is not claimed
yet. The focused Pages HMR WASM test binary compiles, but the package-wide
browser runner also builds the pre-existing
`route_loader_navigation_wasm_test`, which currently fails before this feature
test runs because its router and macro imports are unavailable. The local
`chromedriver` is version 148 while the discoverable Chrome is version 150, so
the existing browser harness cannot be used safely for this result.

## Environment

- Date: 2026-07-19 JST
- Host: Apple Silicon ARM64, macOS 26.5.2 / Darwin 25.5.0
- Rust: `rustc 1.96.0 (ac68faa20 2026-05-25)`
- Cargo: `cargo 1.96.0 (30a34c682 2026-05-25)`
- Watcher debounce for the existing runserver path: 120 ms default
- Browser execution: not run; Chrome 150 is installed, but the available
  chromedriver is 148 and `wasm-pack test` also reaches the unrelated route
  loader WASM compile failure

## Deterministic proxy

The new `incremental-pages-template-hot-patch` scenario builds
`page_hot_patch_bench` when needed and performs 30 warmed reads of the
Manouche `page!` manifest and wire descriptor serialization against
`crates/reinhardt-pages/tests/fixtures/spa_navigation_app/src/hot_patch.rs`.
It reports percentiles from the individual iterations rather than measuring a
single aggregate shell command.

Command:

```text
scripts/bench-build.sh --dry-run incremental-pages-template-hot-patch
CARGO_TARGET_DIR=/tmp/reinhardt-issue-5581-task11-target \
CARGO_BUILD_BUILD_DIR=/tmp/reinhardt-issue-5581-task11-build \
cargo run -q -p reinhardt-commands --features pages,autoreload,reinhardt-db \
  --example page_hot_patch_bench -- \
  crates/reinhardt-pages/tests/fixtures/spa_navigation_app/src/hot_patch.rs \
  --iterations 30
```

Observed output:

```text
iterations=30 p50_ms=0.031 p95_ms=0.049
```

These numbers measure manifest parsing, lowering, and descriptor serialization
only. They exclude file-watch debounce, WebSocket delivery, browser DOM
mutation, MutationObserver delivery, and paint. They must not be presented as
browser-visible latency.

## Required follow-up measurement

After the package-wide route-loader WASM test and matching browser driver are
available, run the detached Cocrea validation copy with the exact `runserver`
command and a real Chrome session. Record 30 source edits, the
MutationObserver-visible update time, preserved signal/handler/node identity,
fallback build time, and BuildRecovered time. Report p50 and p95 separately
for static class/text edits, structural edits, and fallback edits, including
the effective watcher debounce.
