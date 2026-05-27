# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3](https://github.com/kent8192/reinhardt-web/compare/reinhardt-admin-cli@v0.1.2...reinhardt-admin-cli@v0.1.3) - 2026-05-27

### Fixed

- *(ci)* merge main and fix DSL formatting for examples-twitter common.rs
- *(admin-cli)* run rustfmt on DSL-skipped files in fmt command
- *(admin-cli)* pass ignore-all files through rustfmt in fmt command
- *(admin-cli)* traverse past sub-crate Cargo.toml when searching for rustfmt config

### Changed

- *(admin-cli)* replace the imperative AST formatter with a tree-sitter and Topiary pipeline for `page!`, `form!`, and `head!` DSL macros.

## [0.1.1](https://github.com/kent8192/reinhardt-web/compare/reinhardt-admin-cli@v0.1.0...reinhardt-admin-cli@v0.1.1) - 2026-05-24

### Added

- *(admin-cli)* scaffold migrate-manouche-v2 subcommand
- *(admin-cli)* codemod pipeline scaffolding (walker + rule trait + review fixes)
- *(admin)* add form! macro DSL formatting support

### Fixed

- *(admin)* preserve blank lines inside page! macro DSL when formatting
- *(admin)* preserve comments and blank lines in codemod rewriting
- *(admin)* use text-based item search for codemod formatting preservation
- *(admin-cli)* address CodeRabbit review on form! detection, char/lifetime scan, temp file, and codemod rules
- *(admin-cli)* use unique temp filename in target directory for atomic rename
- *(admin-cli)* ensure temp file cleanup runs on rename failure
- *(admin)* clean up temp file when std::fs::write fails in write_developer_file
- *(admin-cli)* revert version to 0.1.0
- *(admin-cli)* resolve formatting issues in fmt-all output
- *(admin-cli)* remove invalid callbacks wrapper from formatter
- *(admin-cli)* skip rustfmt for closures containing page!/form! macros
- *(admin-cli)* use rustfmt directly for closures with page!/form! macros
- *(reinhardt-admin-cli)* add page! macro protection in closure and handler expression formatting
- *(reinhardt-admin-cli)* emit form! DSL syntax for wrapper, icon, icon_position fields
- *(reinhardt-admin-cli)* add form! token preprocessing to convert internal AST to DSL
- *(reinhardt-admin)* strip trailing commas from Icon attrs before merging children
- *(reinhardt-admin-cli)* fix off-by-one bounds check in parse_wrapper_inner and parse_icon_inner
- *(ci)* resolve Rust 1.94 clippy failures

### Styling

- *(admin)* fix indentation in write_developer_file write-error handler
- *(admin-cli)* apply rustfmt to migrate_v2 codemod tests

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-admin-cli@v0.1.0-rc.30...reinhardt-admin-cli@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-admin-cli` as part of the
reinhardt-web 0.1.0 release. This crate ships the `reinhardt-admin`
binary — the project's scaffolding, formatter, and plugin
manager — driven from a single `clap`-based entry point.

For the workspace-wide release narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is in the [Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **Project and app scaffolding** — `startproject` generates a
  new Reinhardt project (matching the per-app layout from
  [#4476](https://github.com/kent8192/reinhardt-web/discussions/4476)), `startapp` adds an application module, and both
  accept `--template-dir` to override the embedded templates.
- **Template selection via `ArgGroup`** — `--template /
  --with-pages / --with-rest` replaces the previous
  `--template-type` flag; the selection is enforced by a `clap`
  `ArgGroup` so invalid combinations fail at parse time.
- **AST-based formatter** — `fmt` recursively walks nested
  `page!` macros, formats them through `prettyplease`, and
  hardens the parser against false-positive `page!`-shaped
  substrings inside strings and comments. Files without a
  `page!` macro are a no-op (not a skip). Writes are atomic
  with permission preservation and automatic backup cleanup.
- **Plugin manager** — `plugin install / remove / list / search /
  enable / disable / update / info` subcommands provide a single
  workflow for managing third-party extensions during the RC
  phase.
- **Secure defaults** — atomic file writes, recursion-depth guard
  on the AST formatter, sanitised error messages (no leaked
  paths), input validation on mutation paths, secret zeroing on
  drop via `zeroize`, and `.env` parsing with TOCTOU
  mitigations.

### Notable Breaking Changes

- **`--template-type` removed** — replaced by `--template /
  --with-pages / --with-rest` enforced via a `clap` `ArgGroup`.
  Scripts that pass `--template-type` must be updated to one of
  the new flags.
- **Per-app layout in `startproject`** ([#4476](https://github.com/kent8192/reinhardt-web/discussions/4476)) — scaffolding emits the new layout where per-app `server_fn` and client UI live under `apps/<app>/` instead of `commands/templates/...`.

### Migration Notes

- Replace `reinhardt-admin startproject --template-type <kind>`
  invocations with `--template <kind>` (or the matching
  `--with-pages` / `--with-rest` flag).
- Existing projects that pre-date [#4476](https://github.com/kent8192/reinhardt-web/discussions/4476) should
  relocate their per-app handlers manually; the scaffold emits
  the new layout automatically for newly created apps.
- For the workspace-wide migration narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
