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

## Prevention Guidelines

- Keep proc-macro output thin; emit metadata and delegate repeated logic to
  runtime helper functions where public API compatibility allows it.
- Avoid large generic function bodies when a small generic shim can delegate to
  a non-generic inner function.
- Avoid adding dependencies to shared crates unless the dependency is needed on
  the shared hot path.
- Keep server-only, WASM-only, and pure UI/template hot-reload paths separate
  so unrelated targets are not rebuilt.
