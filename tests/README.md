# reinhardt-test-support

Integration tests for Reinhardt framework

## Overview

Comprehensive integration test suite for testing interactions between multiple Reinhardt crates. Tests real-world scenarios with actual infrastructure using TestContainers.

## Features

- Integration tests for multiple crate interactions
- Real infrastructure testing with TestContainers
- Database integration tests (PostgreSQL, MySQL, SQLite)
- HTTP server integration tests
- Authentication and authorization flow tests
- Serializer and ORM integration tests
- Template rendering integration tests
- End-to-end API workflow tests

## Facade Feature Regression Tests

The `reinhardt-facade-tests` package checks downstream feature contracts without
building the full integration-test dependency graph. Its temporary consumers live
outside the framework workspace and depend only on the facade (plus `uuid` for
the UUID suite), so workspace feature unification cannot hide missing facade APIs
or enable a missing extractor implementation.

```bash
cargo test -p reinhardt-facade-tests --test uuid_path
cargo test -p reinhardt-facade-tests --test middleware_features
cargo test -p reinhardt-facade-tests --test test_fixture_features
```

The UUID suite checks single and tuple path injection with `di`, `minimal`, and
`api-only`, optional DI activation on native targets, and the WASM dependency
boundary. Consumer Cargo commands run offline; populate the dependency cache
before running the suite in a fresh environment.

The middleware suite checks component-only CORS, compression, security, rate-limit,
and JWT imports, CORS with API and umbrella presets, and the WASM dependency
boundary. Each consumer disables default features and has no direct middleware
dependency. Cargo commands run offline first and retry once online only when a
registry dependency is missing from the cache, including WASM-only dependencies.

The fixture suite checks native WebSocket and GraphQL fixture imports with
`websockets,test`, `graphql,test`, and both protocols together. It also verifies
that protocol features alone do not enable the optional test dependency and that
native fixture dependencies stay outside the WASM graph. Each consumer disables
default features and depends only on the facade. Cargo commands use the same
bounded registry-cache retry as the middleware suite.
