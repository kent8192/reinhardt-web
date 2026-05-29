# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-db-macros@v0.1.2...reinhardt-db-macros@v0.2.0-rc.2) - 2026-05-29

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Fixed

- *(ci)* recover develop release-plz prerelease

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-db-macros@v0.1.0-rc.30...reinhardt-db-macros@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-db-macros` as part of the
reinhardt-web 0.1.0 release. `reinhardt-db-macros` provides the
procedural macros that back `reinhardt-db`'s NoSQL ODM layer (the
`#[derive(Document)]` family) and supports the SQL-side `#[model]`
field-attribute parser.

For the workspace-wide release narrative (Highlights, Breaking
Changes, Migration Guide), see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is preserved in the
[Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **Strict field-attribute validation** — Unknown attribute names on
  derive inputs emit a compile-time error pointing at the offending
  span, instead of being silently ignored. Typos no longer
  short-circuit derives.
- **`missing_docs` lint hygiene** — The crate compiles under
  `#![warn(missing_docs)]`, so every public macro and helper carries
  rustdoc and contributes to `docs.rs` coverage.
- **Stable proc-macro toolchain** — Locked to the workspace-pinned
  `syn` / `quote` / `proc-macro2` versions and Rust 1.94.0 MSRV, so
  proc-macro recompilation is deterministic across CI and downstream
  users.

### Notable Breaking Changes

This release does not introduce crate-level breaking changes. See the
[root Migration Guide](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22)
for workspace-wide changes that may affect callers of derived APIs.

### Migration Notes

- **Fix unknown field attributes**: If an upgrade fails with
  `unknown field attribute ...`, remove the offending attribute or
  rename it to one of the documented forms — pre-stable releases
  accepted it silently.
