# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
