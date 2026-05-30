# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3](https://github.com/kent8192/reinhardt-web/compare/reinhardt-di@v0.1.2...reinhardt-di@v0.1.3) - 2026-05-30

### Added

- *(di)* add DependsResult and DependsOption sugar type aliases

### Fixed

- *(di)* resolve DependsResult/DependsOption field injection from registry

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-di@v0.1.0-rc.30...reinhardt-di@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-di` as part of the reinhardt-web
0.1.0 release. Provides the framework's dependency-injection runtime:
the `Depends<T>` extractor, the `#[inject]` parameter attribute, and the
global `DependencyRegistry` that powers DI across HTTP handlers, server
functions, GraphQL/gRPC, and WebSocket consumers.

For the workspace-wide release narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is in the [Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **Unified `Depends<T>` extractor** — every `#[inject]` parameter is
  typed as `Depends<T>`; the framework caches resolution per request,
  detects cycles via a task-local guard, and surfaces typed metadata
  the legacy `Arc<T>` shape could not carry.
- **Scope-aware `InjectionContext`** — context forking for per-request
  scopes, a deep-cloned `request_scope`, a fallback path for
  pre-seeded types, and a protocol-agnostic `fork()` shared across
  HTTP, GraphQL, gRPC, and WebSocket entry points.
- **Middleware-contributed DI** — `Middleware::di_registrations()`
  lets middleware (admin, auth, session) attach DI bindings that the
  framework picks up at route-server construction time, removing a
  long-standing route-vs-server scope gap.
- **Typed errors with HTTP mapping** — `DiError::Authentication` /
  `Authorization` map to 401 / 403 responses through
  `ParamError::Authentication`; both enums are `#[non_exhaustive]`.
- **Optional and validated extraction** — blanket `Injectable` for
  `Option<T>` enables optional dependencies; `Validated<T>`
  auto-validates extracted payloads before handler dispatch.
- **Per-request and per-test isolation** — the `testing` feature
  exposes `register_override` and the RAII `OverrideGuard` so mocks
  can be installed for a single test without leaking into other
  threads.
- **Hardened proc-macro output** — the `#[injectable]` and
  `#[injectable_factory]` expansions auto-derive `Clone`, register
  qualified type names, route `async_trait` through reinhardt-core,
  enforce attribute ordering, and reject unknown arguments at compile
  time.
- **Security-hardened registry** — `RegistryValidator`'s
  `FrameworkTypeOverride` check rejects accidental shadowing of
  framework types; ReDoS-safe pattern length limits and body-size
  caps were added to parameter extractors during the alpha cycle.

### Notable Breaking Changes

- **`Arc<T>` → `Depends<T>` on `#[inject]`** — see
  [#3628](https://github.com/kent8192/reinhardt-web/discussions/3628).
- **`Injected<T>` deprecated** — replaced by `Depends<T>`; the
  auto-`Clone` bound is removed (see
  [#3631](https://github.com/kent8192/reinhardt-web/discussions/3631)).
- **`Middleware::di_registrations()` hook** introduced; non-auth
  `DiError` variants now map to `ParamError::Internal` so 500s are
  not silently relabeled.
- **`DiError` and `ParamError` are `#[non_exhaustive]`** — match arms
  on these enums must include a default fallback.

### Migration Notes

See the workspace-wide migration guide in the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#migration-guide)
for `Arc<T>` → `Depends<T>` and `Injected<T>` → `Depends<T>`
walkthroughs. The key per-site change is mechanical: replace
`#[inject] Arc<T>` with `#[inject] Depends<T>` and add an explicit
`#[derive(Clone)]` if your concrete type was relying on the previous
auto-`Clone` behaviour.
