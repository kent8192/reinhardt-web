# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.5](https://github.com/kent8192/reinhardt-web/compare/reinhardt-graphql@v0.2.0-rc.4...reinhardt-graphql@v0.2.0-rc.5) - 2026-06-11

### Documentation

- update version references to v0.2.0-rc.5

## [0.2.0-rc.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-graphql@v0.1.3...reinhardt-graphql@v0.2.0-rc.2) - 2026-06-03

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Fixed

- *(ci)* recover develop release-plz prerelease

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-graphql@v0.1.0-rc.30...reinhardt-graphql@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-graphql` as part of the
reinhardt-web 0.1.0 release. Wires an async-graphql-based server into
reinhardt-web's routing, dependency injection, and per-request scope
model.

For the workspace-wide release narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is in the [Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **GraphQL handlers with DI** — handler macros fork the
  `InjectionContext` per request, so resolvers can declare
  `Depends<T>` parameters and obtain request-scoped services without
  smuggling state through `async_graphql::Context`.
- **Query complexity & depth limits** — `QueryLimits` enforces field
  counts (with inline-fragment and block-string awareness),
  validates names by Unicode scalar count, and exits counting paths
  early on multi-byte input.
- **Subscription backpressure** — subscriptions propagate stream
  errors to GraphQL clients instead of silently dropping them, and
  carry backpressure on subscription channels.
- **Resilient runtime locks** — `RwLock` use was migrated off raw
  `unwrap()` to a poison-recovery pattern centralised in a logging
  helper, so a single panicked task no longer poisons the server.
- **Reinhardt facade re-exports** — base async-graphql types are
  re-exported through the reinhardt facade, so user crates can take
  a single workspace dependency.
- **UUID v7 by default** — generated identifiers in graphql-touching
  paths follow the workspace-wide migration to UUID v7 (with v4
  retained for security-sensitive tokens).

### Notable Breaking Changes

- **`Injected<T>` deprecated** ([#3631](https://github.com/kent8192/reinhardt-web/discussions/3631))
  — resolver injection sites move to `Depends<T>` and lose the
  implicit `Clone` bound.

### Migration Notes

See the [root CHANGELOG migration guide](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#migration-guide)
for the DI changes. GraphQL-specific resolvers require no further
rewrite: existing async-graphql schemas continue to work; only
`Injected<T>` → `Depends<T>` at resolver entry points needs to
be applied.
