# Build Performance

## Purpose

Build-performance work must be measured before and after each optimization.
Do not claim compile-time or hot-reload latency improvements from intuition,
successful builds, or isolated code review alone.

## Measurement Rules

1. Use `scripts/bench-build.sh` or `cargo make bench-builds` for standard
   workspace measurements.
2. Record the generated JSON report and adjacent `.env.txt` file when a PR
   claims a performance improvement.
3. Keep local development incremental-friendly: prefer Cargo incremental
   compilation over a local Rust `sccache` wrapper unless a measured setup
   proves otherwise.
4. Treat proc-macro and shared-crate edits as separate scenarios because they
   can invalidate a much larger part of the dependency graph than leaf edits.
5. For Pages hot-reload work, verify browser-visible behavior with runtime
   evidence. A successful compile or HTTP 200 response is not enough.
6. Count `page!` edits as compile-free only for the development `hmr` path's
   conservative static boundary: literal text and literal attributes in
   WASM-owned templates. Dynamic expressions, handlers, bindings, control
   flow, components, callsite changes, and shared/SSR-visible edits must be
   measured as rebuilds.
7. Record the effective `runserver --watch-delay` value for browser-visible
   hot-reload measurements. The debounce window is part of the observed tail
   latency even when no Rust or WASM rebuild is required.

## Prevention Guidelines

- Keep proc-macro output thin; emit metadata and delegate repeated logic to
  runtime helper functions where public API compatibility allows it.
- Avoid large generic function bodies when a small generic shim can delegate to
  a non-generic inner function.
- Avoid adding dependencies to shared crates unless the dependency is needed on
  the shared hot path.
- Keep server-only, WASM-only, and pure UI/template hot-reload paths separate
  so unrelated targets are not rebuilt.
- Keep browser reload notifications success-gated: failed rebuilds should not
  force browsers to reload into stale or missing artifacts.

## Template hot-patch measurement boundary

The compile-free path uses the Manouche page-template manifest and a typed HMR
patch batch. It does not replace the application root with an HTML string.
Measure the deterministic parser/descriptor proxy with
`incremental-pages-template-hot-patch`, then measure the browser path
separately with 30 warmed edits. Browser-visible measurements must record the
effective `runserver --watch-delay`, WebSocket delivery, DOM mutation,
MutationObserver delivery, and paint boundary.

The browser check should assert that a static edit does not invoke the WASM
builder and that signals, reactive ranges, event handlers, keyed instances,
and bound elements remain usable. Also record the fallback path for an ABI or
transaction rejection, including diagnostics, retained old application, and
the subsequent successful-build recovery. Focus, selection, scroll position,
and uncontrolled input state are not guaranteed when a static node is replaced.
