# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.6](https://github.com/kent8192/reinhardt-web/compare/reinhardt-query-macros@v0.2.0-rc.5...reinhardt-query-macros@v0.2.0-rc.6) - 2026-06-13

### Documentation

- *(release)* finalize 0.2.0 changelog
- *(release)* refine 0.2.0 changelog narrative
- *(release)* fold crate rc6 changelogs into stable notes

## [0.2.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-query-macros@v0.1.3...reinhardt-query-macros@v0.2.0) - 2026-06-11

Stable release of `reinhardt-query-macros` for the Reinhardt 0.2.0 line. This
entry consolidates the 0.2.0 release-candidate series; the original
RC entries remain below as detailed history.

### Migration Notes

- Regenerate query macro output after replacing removed query aliases.
- See [`instructions/MIGRATION_0.2.md`](../../../instructions/MIGRATION_0.2.md) for the workspace migration checklist.

### Documentation

- *(release)* enforce public API doc coverage


## [0.2.0-rc.5](https://github.com/kent8192/reinhardt-web/compare/reinhardt-query-macros@v0.2.0-rc.4...reinhardt-query-macros@v0.2.0-rc.5) - 2026-06-11

### Documentation

- *(release)* enforce public API doc coverage

## [0.2.0-rc.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-query-macros@v0.1.3...reinhardt-query-macros@v0.2.0-rc.2) - 2026-06-03

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Fixed

- *(ci)* recover develop release-plz prerelease

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-query-macros@v0.1.0-rc.30...reinhardt-query-macros@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-query-macros` as part of the
reinhardt-web 0.1.0 release. `reinhardt-query-macros` provides the
`#[derive(Iden)]` procedural macro used by `reinhardt-query` to turn
plain Rust enums / structs into SQL identifier sources.

For the workspace-wide release narrative (Highlights, Breaking
Changes, Migration Guide), see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is preserved in the
[Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **`#[derive(Iden)]`** — Generates an `Iden` impl that the
  `reinhardt-query` AST consumes as a table or column reference.
  Works on both unit enums (one variant per column) and structs.
- **Struct-level `#[iden]` attributes** — `#[iden(rename = "...")]`
  and similar options are read from the struct level (not the first
  field), supporting `Meta::List` syntax so the attribute parses
  uniformly with other field-level attributes in the workspace.
- **Special `Table` variant handling** — A `Table` variant on an
  enum maps to the enum's own SQL identifier rather than the literal
  string `"Table"`, matching the SeaQuery convention that
  applications expect.
- **Identifier validation at compile time** — Identifier names are
  validated (and enum variants with data are rejected with a clear
  diagnostic) so invalid SQL identifiers cannot reach runtime.
- **Stable proc-macro toolchain** — Locked to workspace `syn` /
  `quote` / `proc-macro2` versions and Rust 1.94.0 MSRV with
  `heck`-driven case conversion.

### Notable Breaking Changes

This release does not introduce crate-level breaking changes. See the
[root Migration Guide](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22)
for workspace-wide changes that may affect callers of derived APIs.

### Migration Notes

- **Yanked `0.1.0-alpha.3`**: If your `Cargo.lock` still pins
  `0.1.0-alpha.3` (which was yanked), update to `0.1.0` or any
  later release. `0.1.0-alpha.4` was published specifically to
  restore publishability for dependents.
- **`#[iden]` attribute moved to the struct level**: If you were
  relying on the pre-stable behaviour of reading `#[iden]` from the
  first field, move the attribute to the struct itself. The macro
  now emits an error for invalid attribute arguments instead of
  silently ignoring them.
