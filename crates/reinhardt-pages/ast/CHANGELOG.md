# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-pages-ast@v0.1.3...reinhardt-pages-ast@v0.2.0-rc.2) - 2026-06-01

### Added

- *(pages)* support keyed page list rendering

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Fixed

- *(ci)* recover develop release-plz prerelease

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-pages-ast@v0.1.0-rc.30...reinhardt-pages-ast@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-pages-ast` as part of the
reinhardt-web 0.1.0 release. This crate hosts the shared AST
definitions for the `page!` / `head!` / `form!` macro DSLs,
consumed by both the procedural macros in `reinhardt-pages-macros`
and out-of-tree tooling (notably the `reinhardt-admin` CLI
formatter).

For the workspace-wide release narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is in the [Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **Page / element AST** — `syn`-based parser for the `page!`
  DSL with bounded recursion depth, strict allowlists for tag
  names (including wrapper / icon elements), and rejection of
  unsafe `img src` URL schemes at parse time.
- **Form AST** — `FormMacro`, `FormField`, `FormFieldProperty`,
  and supporting types model field declarations, widgets,
  validators, and the `autocomplete` attribute; duplicate-property
  detection runs in the parser.
- **`on_success_ref` mirror field** — pages-side AST mirrors the
  manouche-side `on_success_ref` field so the closure-lift
  pipeline in `reinhardt-pages-macros` can route through a shared
  surface without re-parsing.
- **Hardened error paths** — `parse_if_node` returns `syn::Error`
  rather than `unreachable!()`, `FormFieldProperty::name` returns
  `Option` instead of panicking, and the SVG icon parser has a
  bounded nesting depth.
- **Re-exports from `reinhardt-manouche`** — the canonical macro
  IR primitives live in `reinhardt-manouche` and are re-exported
  here so downstream proc-macro and formatter crates depend on a
  single AST surface.

### Notable Breaking Changes

`reinhardt-pages-ast` is primarily an internal AST surface
consumed by `reinhardt-pages-macros` and the `reinhardt-admin`
CLI. Breaking changes that affect end-users surface through those
crates; see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22)
for the workspace-wide list.

### Migration Notes

End-users do not normally depend on `reinhardt-pages-ast`
directly. Tooling authors should follow the [root migration guide](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22)
and pin against the canonical `reinhardt-manouche` IR exposed
through this crate.
