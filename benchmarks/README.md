# Framework Comparison Benchmarks

This directory defines the Reinhardt framework comparison suite. The suite
tracks the same scenario matrix for four targets:

- Reinhardt
- Axum
- Actix Web
- Loco

The suite is an independent Cargo workspace so the normal Reinhardt workspace
checks do not pull comparison-framework dependencies unless this suite is
invoked explicitly.

## Commands

List the matrix:

```bash
cd benchmarks
CARGO_TARGET_DIR=target cargo run --locked --manifest-path suite/Cargo.toml -- list
```

Validate the matrix and scenario manifests:

```bash
cd benchmarks
CARGO_TARGET_DIR=target cargo run --locked --manifest-path suite/Cargo.toml -- check
```

Preview declared runners, metrics, units, and target coverage:

```bash
cd benchmarks
CARGO_TARGET_DIR=target cargo run --locked --manifest-path suite/Cargo.toml -- dry-run
```

Measure scenario coverage and manifest validation overhead:

```bash
cd benchmarks
CARGO_TARGET_DIR=target cargo run --locked --manifest-path suite/Cargo.toml -- measure
```

Run the concrete runtime HTTP benchmark executors:

```bash
cd benchmarks
CARGO_TARGET_DIR=target cargo bench --locked --bench runtime_http -- --noplot
```

The `runtime_http` benchmark currently executes `hello_world`, `json_echo`,
`path_params`, and `query_params` against Reinhardt, Axum, Actix Web, and Loco
using loopback HTTP servers and a shared HTTP client.

The `suite` checker package intentionally has no third-party dependencies and
is independent from the runtime benchmark package, so list/check/dry-run/measure
commands can validate the committed scenario manifests without resolving the
runtime benchmark framework graph.

## Categories

Runtime scenarios measure request handling behavior. Database
scenarios measure app data-path behavior using the same fixture data shape per
target. Compile-time scenarios measure scaffolded application build loops.
Contract scenarios measure introspection and deployment-contract generation
work. Admin scenarios measure list/detail/form/search surfaces against the
same row shapes and result sizes.

Every scenario manifest must include all four target identifiers. Frameworks
without a built-in feature for a scenario should use the smallest documented
native application fixture for that framework and keep the adapter scope
recorded in the scenario implementation.
