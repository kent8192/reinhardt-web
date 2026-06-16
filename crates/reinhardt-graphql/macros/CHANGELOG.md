# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-graphql-macros@v0.2.0...reinhardt-graphql-macros@v0.3.0) - 2026-06-16

### Added

- feat!(di): introduce keyed injectable provider outputs

## [0.2.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-graphql-macros@v0.1.3...reinhardt-graphql-macros@v0.2.0) - 2026-06-11

Stable release of `reinhardt-graphql-macros` for the Reinhardt 0.2.0 line. This
entry consolidates the 0.2.0 release-candidate series; the original
RC entries remain below as detailed history.

### Migration Notes

- Review the breaking changes listed below before upgrading from 0.1.x.
- See the root CHANGELOG and `instructions/MIGRATION_0.2.md` for cross-crate migration guidance.

### Documentation

- *(release)* enforce public API doc coverage


## [0.2.0-rc.5](https://github.com/kent8192/reinhardt-web/compare/reinhardt-graphql-macros@v0.2.0-rc.4...reinhardt-graphql-macros@v0.2.0-rc.5) - 2026-06-11

### Documentation

- *(release)* enforce public API doc coverage

## [0.2.0-rc.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-graphql-macros@v0.1.3...reinhardt-graphql-macros@v0.2.0-rc.2) - 2026-06-03

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Fixed

- *(ci)* recover develop release-plz prerelease

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-graphql-macros@v0.1.0-rc.30...reinhardt-graphql-macros@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-graphql-macros` as part of the
reinhardt-web 0.1.0 release. Provides the procedural macros that bind
GraphQL handlers and subscriptions to the framework's DI runtime.

For the workspace-wide release narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is in the [Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **Per-request DI context** — handler macros fork the
  `InjectionContext` on each request, so resolvers see a clean
  request scope without sharing mutable state between concurrent
  requests.
- **Compile-time `skip_if` validation** — invalid `skip_if`
  expressions produce a proper compile error instead of expanding to
  code that fails at runtime.
- **Resilient subscription codegen** — the subscription macro
  propagates stream errors to clients rather than dropping them, and
  uses proper error handling in place of `expect()`.
- **Strict input validation** — macro-generated code applies the
  same resource limits the runtime enforces, surfaces clear errors
  on crate-resolution failures, and hardens generated names.

### Notable Breaking Changes

- **`Injected<T>` deprecated** ([#3631](https://github.com/kent8192/reinhardt-web/discussions/3631))
  — resolver attributes accept `Depends<T>` rather than the
  deprecated extractor.

### Migration Notes

See the [root CHANGELOG migration guide](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#migration-guide).
Move resolver injection sites from `Injected<T>` to `Depends<T>`; no
other macro syntax change is required.
