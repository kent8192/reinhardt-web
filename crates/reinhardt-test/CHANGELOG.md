# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-test@v0.1.1...reinhardt-test@v0.1.2) - 2026-05-25

### Documentation

- update version references to v0.1.2

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-test@v0.1.0-rc.30...reinhardt-test@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-test` as part of the
reinhardt-web 0.1.0 release. `reinhardt-test` is the user-facing
testing facade: it bundles `rstest` fixtures for TestContainers-
backed databases (PostgreSQL, MySQL, SQLite, CockroachDB), message
brokers (Redis, Kafka, RabbitMQ), and WASM / E2E browser harnesses
on top of the lower-level `reinhardt-testkit`.

For the workspace-wide release narrative (Highlights, Breaking
Changes, Migration Guide), see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is preserved in the
[Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **TestContainers fixtures for every supported backend** — `rstest`
  fixtures bring up PostgreSQL 17, MySQL, SQLite, CockroachDB, Redis
  (single + cluster), Kafka, and RabbitMQ with deterministic
  lifecycles. Containers use module-scoped or per-test scopes as
  appropriate, with random UUID v7 suffixes to avoid collisions
  under `cargo nextest` parallelism.
- **WASM SPA test harness** — Feature-gated `wasm` / `wasm-full` /
  `msw` stacks wire `wasm-bindgen-test`, `web-sys`, `js-sys`, and
  `gloo-timers` into an integration suite that drives `reinhardt-
  pages` SPA UIs in a real browser. The MSW-style network-level
  request interceptor replaces the deprecated `MockFetch` /
  `mock_server_fn` mocks for new code.
- **E2E browser fixtures (fantoccini & CDP)** — `e2e` (WebDriver via
  `fantoccini`) and `e2e-cdp` (Chrome DevTools Protocol via
  `chromiumoxide`, paired with a containerised Chrome) provide two
  complementary E2E paths. The CDP fixture documents the Docker
  Engine 20.10 `host-gateway` requirement and resolves
  `host.docker.internal` for tests that target the host loopback.
- **Admin & auth integration fixtures** — Optional `admin` feature
  provisions Postgres + ORM + auth in one fixture so admin-panel
  permission tests can run end-to-end. Auth fixtures inject
  `is_staff` / `is_superuser` JWT claims and propagate handler-side
  session ID rotation through `Set-Cookie`.
- **Delegation to `reinhardt-testkit`** — Native targets re-export
  from `reinhardt-testkit`, so a test file consumes a single facade
  (`reinhardt_test::fixtures::*`) regardless of which underlying
  capability it touches.
- **Security-hardened helpers** — Path-traversal guards on
  `temp_file_url`, cookie-header injection prevention, URL encoding
  on query parameters, and `escape_html_content` /
  `escape_css_selector` on every WASM string-rendering path.

### Notable Breaking Changes

This crate does not introduce crate-level breaking changes at the
0.1.0 boundary beyond the deprecations listed below. See the
[root Migration Guide](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22)
for workspace-wide changes (e.g., `Depends<T>`, typed URL routing)
that affect the application code under test.

### Migration Notes

- **`MockFetch` / `mock_server_fn` → MSW interceptor**: Both are
  `#[deprecated(since = "0.1.0-rc.16")]`. Migrate to the
  `reinhardt_test::msw` module: it intercepts network requests at
  the boundary instead of stubbing function pointers, which keeps
  Server Function tests aligned with what the browser actually
  sends.
- **`target_arch = "wasm32"` → `target_family` + `target_os`**: If
  your downstream tests gated code on `target_arch = "wasm32"`,
  switch to `all(target_family = "wasm", target_os = "unknown")` —
  this matches the workspace-wide cfg layout and avoids future drift
  to WASIp1 / WASIp2.
- **Feature renamed `static` → `staticfiles`**: The `staticfiles`
  feature flag tracks the upstream `reinhardt-utils::staticfiles`
  module rename ([#114](https://github.com/kent8192/reinhardt-web/issues/114)).
  Update `Cargo.toml` feature lists accordingly.
