# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-routers-macros@v0.1.2...reinhardt-routers-macros@v0.2.0-rc.2) - 2026-05-31

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Fixed

- *(ci)* recover develop release-plz prerelease

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-routers-macros@v0.1.0-rc.30...reinhardt-routers-macros@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-routers-macros` as part of the
reinhardt-web 0.1.0 release. Procedural-macro companion to
`reinhardt-urls` that performs compile-time validation of route paths
and emits the typed URL helpers consumed by `#[routes]` and
`#[url_patterns]`.

For the workspace-wide release narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is in the [Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **Compile-time path validation** — Route paths, SQL fragments, and
  crate references are validated when the macro expands, so a
  malformed `#[routes]` path is a compile error rather than a runtime
  panic.
- **Wildcard and parameter safety** — Path validation rejects
  ambiguous parameters and unsafe wildcard patterns; route paths and
  SQL expressions go through the same input-validation pipeline.
- **Clippy- and rustfmt-clean expansion** — Generated code replaces
  never-looping `for` constructs with `if let` (per
  `clippy::never_loop`) and is formatted via `rustfmt` so consumers
  see clean diagnostics in their own crates.

### Notable Breaking Changes

This is a proc-macro crate consumed exclusively by `reinhardt-urls`;
breaking changes flow through that crate. See the [Breaking Changes
Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/breaking-changes).

### Migration Notes

This is the first stable release, so there is no prior stable version
to migrate from. See the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22)
for the cross-crate migration guide.
