# Issue #5218 Incremental Check Measurements

These measurements compare `origin/main` at
`3016d326de63950ac553cd2246d02a0315bd7f06` with PR `#5220` at the current
branch. The PR branch prunes Pages/WASM dependencies through workspace feature
boundaries, gates dev/test/native-only Pages modules out of browser WASM
builds, and uses a narrower dev profile (`debug = 0`, `codegen-units = 16`)
for generated debug artifacts while keeping `profile.test.debug = 1`.

Environment:

- Host: macOS on `aarch64-apple-darwin`
- Rust: `rustc 1.94.1 (e408947bf 2026-03-25)`
- Tool: `hyperfine`; leaf/core scenarios used `--warmup 1 --runs 2`, latest
  Pages/WASM and server scenarios used `--warmup 2` with 5-8 runs; static
  `page!` hot-patch fixture scenarios used `--warmup 3 --runs 12` initially
  and `--warmup 2 --runs 8` after the dev-profile update; cold standard
  workspace builds used `/usr/bin/time -p` with one clean run per branch

## Results

### Cold Workspace Builds

| Scenario | `origin/main` wall time | PR branch wall time | Change |
|---|---:|---:|---:|
| `cold-build-standard` | 283.30s | 234.83s | 17.1% faster |

Raw command shape:

```bash
cargo clean && cargo build --workspace
```

### Direct Cargo Loops

| Scenario | `origin/main` mean | PR branch mean | Change |
|---|---:|---:|---:|
| `incremental-leaf-check` | 0.389s | 0.512s | 31.4% slower |
| `incremental-core-check` | 50.022s | 31.875s | 36.3% faster |
| `incremental-pages-wasm-check` | 1.528s | 1.174s | 23.2% faster |
| `incremental-pages-wasm-build` | 1.933s | 1.512s | 21.8% faster |
| `incremental-server-build` | 0.947s | 0.710s | 25.0% faster |

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
| Server-only edit | 1.019s | 0.710s | 30.3% faster |

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
| Static `page!` hot patch | 0.005s | 99.4% faster |
| Fixture WASM build | 0.564s | 31.6% faster |
| Legacy fixture WASM + server build | 0.825s | baseline |

Raw command shapes:

```bash
touch crates/reinhardt-pages/tests/fixtures/spa_navigation_app/src/client.rs && ./target/debug/examples/page_hot_patch_probe crates/reinhardt-pages/tests/fixtures/spa_navigation_app/src/client.rs >/dev/null
touch crates/reinhardt-pages/tests/fixtures/spa_navigation_app/src/client.rs && cargo build --manifest-path crates/reinhardt-pages/tests/fixtures/spa_navigation_app/Cargo.toml --target wasm32-unknown-unknown
touch crates/reinhardt-pages/tests/fixtures/spa_navigation_app/src/client.rs && cargo build --manifest-path crates/reinhardt-pages/tests/fixtures/spa_navigation_app/Cargo.toml --target wasm32-unknown-unknown && cargo build -p reinhardt-server
```

## Interpretation

The shared/core edit loop lands inside the expected 30-60% reduction range.
The leaf package check does not: it is slower in this local run, so PR `#5220`
must not claim a universal incremental-check improvement.

The cold standard workspace build improves by 17.1%. That is a real reduction,
but it is below the original 25-40% cold build/check target, so the cold target
should remain tracked separately from this hot-reload-focused PR.

The targeted server-only hot-reload work shape now reaches the lower end of
the expected 30-60% range. The app-side fixture WASM build improves from the
pre-profile 1.725-1.739s range to 0.564s, which lands in the expected 40-70%
WASM-side Rust range for downstream-style app code. Direct framework
`reinhardt-pages` WASM builds improve to 21.8%, so framework-runtime edits are
still below the broader Pages/WASM target.

The latest Pages/WASM dependency pruning keeps internal Reinhardt dependencies
on `workspace = true`, inherits `reqwest` through the workspace dependency
table, and moves `reinhardt-core`'s heavy image validator and documentation
diagrams behind explicit features. This removes `image`, `ravif`, `tiff`,
`exr`, and `aquamarine` from the Pages/WASM normal dependency tree, but the
warmed incremental build loop is still dominated by recompiling the WASM crate
artifact.

The latest Pages module gating keeps `testing` behind the existing `testing`
feature/test cfg and compiles `SsrRenderer`/`SsrOptions` only on native targets.
Hydration-shared `SsrState` and marker types remain available to browser WASM.
Combined with the dev-profile update, direct framework WASM builds improve to
21.8% faster versus `origin/main`, while downstream-style fixture WASM builds
land in the expected 40-70% range.

The earlier experimental profile change (`line-tables-only`,
`split-debuginfo = "unpacked"`, `codegen-units = 256`, explicit
`incremental = true`) produced worse local numbers for this repository and was
reverted. The narrower dev profile (`debug = 0`, `codegen-units = 16`) is
measured separately and improves server-only and app-side WASM build loops.
The test profile keeps `debug = 1` so test debugging is not degraded.

Static `page!(|| { ... })` edits now have a compile-free development path when
the changed file is owned by the Pages client side (`src/client.rs`,
`src/client/**`, or `src/apps/*/client/**`) and the parser can render the page
body without dynamic Rust. The HMR payload replaces `#app` contents so the
development HMR script and page shell stay mounted. That path reaches the
original 80-98% pure `page!` edit target in the fixture run, measuring 99.4%
faster than the legacy both-target rebuild shape. Dynamic Rust expressions,
event handlers, control flow, components, and shared/server-owned files
intentionally fall back to normal rebuilds, so the broader WASM-side Rust logic
target remains unmet by this hot-patch path.
