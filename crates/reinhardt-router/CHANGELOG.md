# Changelog

All notable changes to this crate are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.5](https://github.com/kent8192/reinhardt-web/compare/reinhardt-router@v0.2.0-rc.4...reinhardt-router@v0.2.0-rc.5) - 2026-06-11

### Documentation

- *(release)* enforce public API doc coverage

## [0.2.0-rc.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-router@v0.1.3...reinhardt-router@v0.2.0-rc.2) - 2026-06-03

### Fixed

- *(ci)* recover develop release-plz prerelease

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-router@v0.1.0-rc.30...reinhardt-router@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-router` as part of the
reinhardt-web 0.1.0 release. This is a brand-new crate introduced
during the 0.1.0 RC phase: it exposes a minimal `VersionedRouter`
trait surface so that `reinhardt-urls` (which owns the concrete
router implementations) and `reinhardt-rest` (which needs to read
namespace / path information out of a router to drive its versioning
strategies) can share an abstraction without forming a circular
crate dependency.

For the workspace-wide release narrative (Highlights, Breaking
Changes, Migration Guide), see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is preserved in the
[Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **`VersionedRouter` trait** — the single trait this crate exposes.
  Concrete router types in `reinhardt-urls` implement it so that
  `reinhardt-rest::versioning` can operate generically without
  knowing about URL-pattern internals ([#4321](https://github.com/kent8192/reinhardt-web/issues/4321)).
- **`RouteVersionInfo` value type** — a small `Clone`-cheap value
  describing namespace, version, and path metadata for a matched
  route, designed to be passed across the `reinhardt-urls` /
  `reinhardt-rest` boundary without re-exporting URL-pattern types.
- **Zero runtime dependencies** — the crate ships trait definitions
  and value types only; it has no external dependencies, no
  `std::sync` machinery, and no async glue, which keeps it cheap
  to depend on from both server and WASM targets.

### Notable Breaking Changes

This crate is new in 0.1.0; it has no breaking changes against a
previous release. Workspace-level breaking changes that introduced
this crate are tracked at the [root CHANGELOG Breaking Changes section](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#breaking-changes).

### Migration Notes

See the [root Migration Guide](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#migration-guide)
for project-wide guidance. Consumers writing custom routers should
implement `VersionedRouter` on their router type so that
`reinhardt-rest`'s versioning machinery can consume it.
