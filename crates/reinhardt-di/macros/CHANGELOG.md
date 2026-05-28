# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-di-macros@v0.1.2...reinhardt-di-macros@v0.2.0-rc.2) - 2026-05-28

### Added

- *(di)* [**breaking**] remove Injected and OptionalInjected (refs [[#4520](https://github.com/kent8192/reinhardt-web/issues/4520)](https://github.com/kent8192/reinhardt-web/issues/4520))

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates
- *(di)* delete deprecated Injected<T> and OptionalInjected<T> types

### Fixed

- *(ci)* recover develop release-plz prerelease

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-di-macros@v0.1.0-rc.30...reinhardt-di-macros@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-di-macros` as part of the
reinhardt-web 0.1.0 release. Provides the procedural macros that drive
the DI runtime: `#[inject]`, `#[injectable]`, `#[injectable_factory]`,
and `#[scope(...)]`.

For the workspace-wide release narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is in the [Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **`Depends<T>`-typed `#[inject]` parameters** — the macro accepts
  `#[inject] Depends<T>` (the unified shape) and emits the right
  resolver call; legacy `Arc<T>` parameters were removed during the
  rc cycle.
- **`#[injectable]` with auto-Clone** — derives `Clone` for
  injectable types so the runtime can hand out cached copies without
  callers spelling out the bound.
- **`#[injectable_factory]` with `Depends<T>` support** — generates
  an `Injectable` impl that participates in registry lookup,
  forwards to `Injectable::inject()` for non-`Depends` params, and
  wraps factory bodies in a cycle-detection scope.
- **Qualified type-name registration** — emitted code registers
  qualified names with the registry, enabling
  `FrameworkTypeOverride` validation and richer diagnostics.
- **Single-attribute scope argument** — `(scope = "request")` is the
  canonical form; the older `#[scope(...)]` literal alternation is
  documented as deprecated in the rc cycle.
- **Compile-time hygiene** — rejects unknown macro arguments,
  validates type paths, generates names safely, and routes
  `async_trait` through reinhardt-core so users do not need to add
  the dependency themselves.

### Notable Breaking Changes

- **`#[inject]` parameter type unification** — see
  [#3628](https://github.com/kent8192/reinhardt-web/discussions/3628).
- **`Injected<T>` deprecated** — see
  [#3631](https://github.com/kent8192/reinhardt-web/discussions/3631).

### Migration Notes

See the [root CHANGELOG migration guide](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#migration-guide).
Macro-side changes are mostly mechanical (`Arc<T>` → `Depends<T>` on
`#[inject]` parameters and on `#[injectable_factory]` arguments); the
attribute-ordering requirement is documented in the crate's compile-fail
tests.
