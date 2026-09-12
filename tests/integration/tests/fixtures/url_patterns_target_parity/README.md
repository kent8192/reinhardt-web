# Shared URL Pattern Consumer Fixture

This fixture uses the real `reinhardt-web` facade and HTTP endpoint attributes.
The host integration test creates isolated Cargo consumers under a temporary
directory, substitutes either `reinhardt` or `framework` as the dependency
name, and shares build artifacts within that test. The owning guards remove
sources, build artifacts, logs, and child processes when the test exits.
Consumer dev and test profiles omit debug information to limit repeated
native linking and WASM binding-generation costs; debug assertions remain
enabled.

The calling crate declares `cfg(server)` in `build.rs` and enables it through
its own `server` feature. Native handler modules, injected types, and captured
values are absent on inactive targets. A native-only dependency deliberately
fails to compile for browser WASM; both Cargo's build artifacts and target
dependency tree must prove its absence there.

## Compile and Native Runtime Matrix

| Target | Caller `server` | `client-router` | Expected behavior |
| --- | --- | --- | --- |
| Native | Enabled | Disabled / enabled | Real HTTP handlers execute |
| Native | Disabled | Disabled / enabled | Server arguments are erased |
| `wasm32-unknown-unknown` | Disabled / enabled | Enabled | Client declarations compile; native references are erased |

Each configuration runs with both facade dependency names. Native tests
compare literal route paths, contract metadata, HTTP responses, middleware
enforcement, injected values, evaluation order, and temporary destruction
against handwritten builders. Both orders of `#[routes]` and
`#[url_patterns]` are linked and executed on native, and compiled for WASM.
Per-app attributes alone must contribute zero inventory registrations; each
selected root contributes exactly one.

The host matrix executes 64 Cargo commands, including 16 native test runs
containing 124 tests in total. Its exact Nextest override
reserves both worker slots and allows three 1200-second periods for nested
compilation; other integration tests retain their existing limits.

Negative controls require specific failures:

- Removing the attribute exposes unavailable native handler paths (`E0433`).
- Passing a non-handler to `.endpoint(...)` still fails in native server code
  (`E0277`).
- Unsupported bodies, methods, and nested server builders produce identical
  primary diagnostics and source spans on every target and cfg variant.

Install `wasm32-unknown-unknown` and populate Cargo's dependency cache before
the offline nested checks:

```sh
rustup target add wasm32-unknown-unknown
cargo fetch --target wasm32-unknown-unknown
cargo test -p reinhardt-integration-tests --test url_patterns_target_parity url_patterns_target_matrix -- --exact --nocapture
```

## Browser Runtime

The browser test is explicitly ignored during ordinary integration runs and
must be requested with `--ignored`. It requires Chrome, `chromedriver`, and
`wasm-pack`:

```sh
cargo test -p reinhardt-integration-tests --test url_patterns_target_parity url_patterns_browser_runtime -- --exact --ignored --nocapture
```

It runs 12 sequential browser configurations: two facade names, two caller
cfg states, and three root-registration states (none or either attribute
order). Four tests per configuration verify client route counts,
literal reverse URLs, missing native route names, unrelated `server` methods,
erased native side effects, and materialized inventory factories. Compile
success alone does not establish browser runtime behavior.

The final fixture run completed all 12 configurations in 56.32 seconds with
48 passing browser tests and a warm compiler cache, using Chrome
152.0.7977.84 and ChromeDriver 152.0.7977.82. An earlier cold run took 602.79
seconds, including 458.5 seconds for its initial build. Each run reused the
same test-owned artifact directories across its configurations.

## Public Example Coverage

`src/lib.rs` exercises the macro's rustdoc example, including the identical
function and macro name. `client_routes.rs` covers shared client composition;
the registration modules cover a single root mounting multiple annotated
apps. `build.rs` is the caller-cfg example used in the routing documentation.
Qualified macro and type paths are compiled under both dependency names.
Client page factories use the facade's existing `reinhardt_types::page::Page`
bridge to keep this routing fixture independent of the `pages` feature.
