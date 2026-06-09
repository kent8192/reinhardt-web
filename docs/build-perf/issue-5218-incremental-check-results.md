# Issue #5218 Incremental Check Measurements

These measurements compare `origin/main` at
`3016d326de63950ac553cd2246d02a0315bd7f06` with PR #5220 at the current
branch after reverting the experimental dev/test profile tuning back to
`debug = 1`, pruning Pages/WASM dependencies through workspace feature
boundaries, and gating dev/test/native-only Pages modules out of browser WASM
builds.

Environment:

- Host: macOS on `aarch64-apple-darwin`
- Rust: `rustc 1.94.1 (e408947bf 2026-03-25)`
- Tool: `hyperfine`; leaf/core scenarios used `--warmup 1 --runs 2`, latest
  Pages/WASM and server scenarios used `--warmup 2 --runs 5`; static
  `page!` hot-patch fixture scenarios used `--warmup 3 --runs 12`

## Results

### Direct Cargo Loops

| Scenario | `origin/main` mean | PR branch mean | Change |
|---|---:|---:|---:|
| `incremental-leaf-check` | 0.389s | 0.512s | 31.4% slower |
| `incremental-core-check` | 50.022s | 31.875s | 36.3% faster |
| `incremental-pages-wasm-check` | 1.528s | 1.174s | 23.2% faster |
| `incremental-pages-wasm-build` | 1.933s | 1.720s | 11.0% faster |
| `incremental-server-build` | 0.947s | 0.920s | 2.9% faster |

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
| Pages client edit | 1.957s | 1.753s | 10.4% faster |
| Server-only edit | 1.202s | 0.912s | 24.1% faster |

Raw command shapes:

```bash
touch crates/reinhardt-pages/src/component.rs && cargo build -p reinhardt-pages --target wasm32-unknown-unknown --features pages-full && cargo build -p reinhardt-server
touch crates/reinhardt-pages/src/component.rs && cargo build -p reinhardt-pages --target wasm32-unknown-unknown --features pages-full
touch crates/reinhardt-server/src/server.rs && cargo build -p reinhardt-server && cargo build -p reinhardt-pages --target wasm32-unknown-unknown --features pages-full
touch crates/reinhardt-server/src/server.rs && cargo build -p reinhardt-server
```

### App-Side `page!` Fixture Loops

These measurements use the detached `spa_navigation_app` browser fixture after
moving its `page!` bodies under `src/client.rs`, which matches the watcher
ownership boundary for WASM-only client source.

| Scenario | Mean | Change vs legacy both-target |
|---|---:|---:|
| Static `page!` hot patch | 0.005s | 99.8% faster |
| Fixture WASM build | 2.292s | 19.1% faster |
| Legacy fixture WASM + server build | 2.832s | baseline |

Raw command shapes:

```bash
touch crates/reinhardt-pages/tests/fixtures/spa_navigation_app/src/client.rs && ./target/debug/examples/page_hot_patch_probe crates/reinhardt-pages/tests/fixtures/spa_navigation_app/src/client.rs >/dev/null
touch crates/reinhardt-pages/tests/fixtures/spa_navigation_app/src/client.rs && cargo build --manifest-path crates/reinhardt-pages/tests/fixtures/spa_navigation_app/Cargo.toml --target wasm32-unknown-unknown
touch crates/reinhardt-pages/tests/fixtures/spa_navigation_app/src/client.rs && cargo build --manifest-path crates/reinhardt-pages/tests/fixtures/spa_navigation_app/Cargo.toml --target wasm32-unknown-unknown && cargo build -p reinhardt-server
```

## Interpretation

The shared/core edit loop currently lands inside the expected 30-60% reduction
range. The leaf package check does not: it is slower in this local run, so PR
#5220 must not claim a universal incremental-check improvement.

The targeted server-only hot-reload work shape is below the expected 30-60%
reduction range in the latest local run. The targeted
Pages/WASM client-edit work shape does not land in the expected 40-70% range:
it improves by 10.4% in the latest same-branch legacy-vs-targeted work-shape
run. Direct WASM build improves 11.0%, so further work
must reduce the WASM build/bindgen path itself rather than only skipping
unrelated native server work.

The latest Pages/WASM dependency pruning keeps internal Reinhardt dependencies
on `workspace = true` and moves `reinhardt-core`'s heavy image validator and
documentation diagrams behind explicit features. This removes `image`,
`ravif`, `tiff`, `exr`, and `aquamarine` from the Pages/WASM normal dependency
tree, but the warmed incremental build loop is still dominated by recompiling
the WASM crate artifact.

The latest Pages module gating keeps `testing` behind the existing `testing`
feature/test cfg and compiles `SsrRenderer`/`SsrOptions` only on native targets.
Hydration-shared `SsrState` and marker types remain available to browser WASM.
This improves direct WASM build from 5.9% to 11.0% faster versus `origin/main`,
but it still does not approach the expected 40-70% Pages/WASM reduction range.

The earlier experimental profile change (`line-tables-only`,
`split-debuginfo = "unpacked"`, `codegen-units = 256`, explicit
`incremental = true`) produced worse local numbers for this repository and was
reverted. Keep the default `debug = 1` profile unless a future benchmark proves
a different profile is faster across the target scenarios.

Static `page!(|| { ... })` edits now have a compile-free development path when
the changed file is owned by the Pages client side (`src/client.rs`,
`src/client/**`, or `src/apps/*/client/**`) and the parser can render the page
body without dynamic Rust. That path reaches the original 80-98% pure
`page!` edit target in the fixture run, measuring 99.8% faster than the legacy
both-target rebuild shape. Dynamic Rust expressions, event handlers, control
flow, components, and shared/server-owned files intentionally fall back to
normal rebuilds, so the broader WASM-side Rust logic target remains unmet by
this hot-patch path.
