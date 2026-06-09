# Issue #5218 Incremental Check Measurements

These measurements compare `origin/main` at
`3016d326de63950ac553cd2246d02a0315bd7f06` with PR #5220 at the current
branch after reverting the experimental dev/test profile tuning back to
`debug = 1`.

Environment:

- Host: macOS on `aarch64-apple-darwin`
- Rust: `rustc 1.94.1 (e408947bf 2026-03-25)`
- Tool: `hyperfine --warmup 1 --runs 2`

## Results

### Direct Cargo Loops

| Scenario | `origin/main` mean | PR branch mean | Change |
|---|---:|---:|---:|
| `incremental-leaf-check` | 0.389s | 0.512s | 31.4% slower |
| `incremental-core-check` | 50.022s | 31.875s | 36.3% faster |
| `incremental-pages-wasm-check` | 1.528s | 1.354s | 11.4% faster |
| `incremental-pages-wasm-build` | 1.933s | 2.032s | 5.1% slower |
| `incremental-server-build` | 0.947s | 1.035s | 9.3% slower |

Raw command shapes:

```bash
touch crates/reinhardt-throttling/src/lib.rs && cargo check -p reinhardt-throttling
touch crates/reinhardt-core/src/lib.rs && cargo check --workspace
touch crates/reinhardt-pages/src/component.rs && cargo check -p reinhardt-pages --target wasm32-unknown-unknown --features pages-full
touch crates/reinhardt-pages/src/component.rs && cargo build -p reinhardt-pages --target wasm32-unknown-unknown --features pages-full
touch crates/reinhardt-server/src/server.rs && cargo build -p reinhardt-server
```

### Hot-Reload Target Selection Loops

These measurements compare the legacy hot-reload work shape, where a
target-specific edit still paid for both WASM and native server builds, against
the PR branch targeted rebuild shape.

| Scenario | Legacy both-target mean | Targeted mean | Change |
|---|---:|---:|---:|
| Pages client edit | 2.402s | 1.863s | 22.4% faster |
| Server-only edit | 1.253s | 0.854s | 31.9% faster |

Raw command shapes:

```bash
touch crates/reinhardt-pages/src/component.rs && cargo build -p reinhardt-pages --target wasm32-unknown-unknown --features pages-full && cargo build -p reinhardt-server
touch crates/reinhardt-pages/src/component.rs && cargo build -p reinhardt-pages --target wasm32-unknown-unknown --features pages-full
touch crates/reinhardt-server/src/server.rs && cargo build -p reinhardt-server && cargo build -p reinhardt-pages --target wasm32-unknown-unknown --features pages-full
touch crates/reinhardt-server/src/server.rs && cargo build -p reinhardt-server
```

## Interpretation

The shared/core edit loop currently lands inside the expected 30-60% reduction
range. The leaf package check does not: it is slower in this local run, so PR
#5220 must not claim a universal incremental-check improvement.

The targeted server-only hot-reload work shape lands just inside the expected
30-60% reduction range. The targeted Pages/WASM client-edit work shape does
not: it improves by 22.4%, below the expected 40-70% range. Direct WASM build
also did not improve, so further work must reduce the WASM build/bindgen path
itself rather than only skipping unrelated native server work.

The earlier experimental profile change (`line-tables-only`,
`split-debuginfo = "unpacked"`, `codegen-units = 256`, explicit
`incremental = true`) produced worse local numbers for this repository and was
reverted. Keep the default `debug = 1` profile unless a future benchmark proves
a different profile is faster across the target scenarios.

These measurements do not prove compile-free `page!` editing. Inline `page!`
macro edits still compile as Rust; compile-free template edits require a
separate dev-mode runtime/template architecture.
