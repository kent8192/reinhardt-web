# Reinhardt Benchmarks

This directory contains benchmark tests for the Reinhardt framework.

## Usage

Run all benchmarks:

```bash
cargo bench -p reinhardt-benchmarks
```

Run a specific benchmark:

```bash
cargo bench -p reinhardt-benchmarks --bench performance_benchmarks
```

Current benchmark targets are `performance_benchmarks`, `auth_benchmarks`, `settings_benchmarks`, and `concurrent_benchmarks`.
`performance_benchmarks` includes the DB pool acquire/release hot path so
connection wrapper overhead can be tracked alongside the existing framework
utility benchmarks.

The cross-framework benchmark matrix lives under `benchmarks/` and compares
Reinhardt with Axum, Actix Web, and Loco across runtime, database,
compile-time, contract, and admin scenarios. Validate that matrix with:

```bash
cargo make benchmark-suite-check
```

Run the concrete runtime HTTP benchmark executors with:

```bash
cargo make benchmark-runtime-http
```

Run the request allocation probe:

```bash
cargo run --release -p reinhardt-benchmarks --bin request_alloc_probe
```

## Adding New Benchmarks

1. Create a new file in the `benches/` directory
2. Add a `[[bench]]` entry to `Cargo.toml`
3. Use the `criterion` crate for benchmarking
