# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-openapi@v0.1.3...reinhardt-openapi@v0.2.0-rc.2) - 2026-06-01

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Fixed

- *(ci)* recover develop release-plz prerelease

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-openapi@v0.1.0-rc.30...reinhardt-openapi@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-openapi` as part of the
reinhardt-web 0.1.0 release. Extracted from `reinhardt-rest` to break
the urls ↔ rest circular dependency, this crate hosts the
`OpenApiRouter` wrapper that mounts Swagger UI, Redoc, and the
generated OpenAPI JSON document onto any router.

For the workspace-wide release narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is in the [Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **`OpenApiRouter` wrapper** — Wraps any router with automatic OpenAPI
  documentation endpoints; implements both the framework `Handler` and
  `Router` traits so it composes inside `#[routes]` functions.
- **Swagger UI, Redoc, and JSON endpoints** — Ships
  `/api/docs` (Swagger UI), `/api/redoc` (Redoc), and
  `/api/openapi.json` (the generated spec) out of the box.
- **Opt-in documentation exposure** — An `enabled` flag plus an
  optional auth guard control whether the docs endpoints are mounted
  in a given environment, so production deployments can hide the spec.
- **Hardened response surface** — Documentation endpoints carry the
  framework's security-header set, and `OpenApiRouter::wrap` returns a
  `Result` instead of panicking when configuration is invalid.

### Notable Breaking Changes

This crate's API surface stabilized incrementally during the rc
cycle. Cross-crate breaking changes are catalogued in the [Breaking
Changes Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/breaking-changes).

### Migration Notes

This is the first stable release, so there is no prior stable version
to migrate from. See the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22)
for the cross-crate migration guide.
