# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3](https://github.com/kent8192/reinhardt-web/compare/reinhardt-pages-macros@v0.1.2...reinhardt-pages-macros@v0.1.3) - 2026-05-31

### Fixed

- *(di)* resolve DependsResult macro injection

## [0.1.1](https://github.com/kent8192/reinhardt-web/compare/reinhardt-pages-macros@v0.1.0...reinhardt-pages-macros@v0.1.1) - 2026-05-24

### Fixed

- *(ci)* resolve Rust 1.94 clippy failures

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-pages-macros@v0.1.0-rc.30...reinhardt-pages-macros@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-pages-macros` as part of the
reinhardt-web 0.1.0 release. This crate provides the procedural
macros that back `reinhardt-pages`: the `page!`, `head!`, and
`form!` declarative DSLs, plus the `#[server_fn]` attribute macro
that emits matched WASM client stubs and server-side handlers.

For the workspace-wide release narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is in the [Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **`page!` macro** — anonymous component DSL with closure-style
  props, 70+ HTML elements validated at compile time, `@event:
  handler` syntax, `if`/`else`/`for` control flow, `watch` blocks
  for `Signal<T>`-driven re-rendering, dynamic expressions for
  `img src`, and accessibility / XSS validation at expansion time.
- **`head!` macro** — title, meta, link, script, and style
  elements for SSR head metadata injection.
- **`form!` macro** — typed form fields (`CharField`, `EmailField`,
  `IntegerField`, `FileField`, `ImageField`, `SubmitButton`, …),
  widget customization, server / client validators, `derived`
  computed values, `FieldGroup`, custom wrapper and SVG icon
  elements, slots, two-way `Signal` binding, CSRF protection,
  `initial_loader`, dynamic `choices_loader`, `autocomplete`
  attribute, and `strip_arguments` for downstream `server_fn`
  shaping.
- **`#[server_fn]` attribute macro** — WASM client stub plus
  server-side handler, custom endpoint paths, codec selection
  (`json` / `url` / `msgpack`), `#[reinhardt::inject]` parameter
  auto-detection, `FromRequest` extractor support, per-request DI
  context forking, and DI error → HTTP status mapping (with
  redacted 500 bodies).
- **MSW-style mocking (`msw` feature)** — generates a
  `MockableServerFn` trait per `#[server_fn]` so tests can stub
  RPC endpoints with the same typed surface they see in
  production.
- **Closure-lift pipeline** — `form! { on_success: |value: T|
  ... }` closures with explicit type annotations lift to the
  outer scope so the body can capture enclosing locals (e.g., a
  `qid` route parameter); `success_url:` and inner `watch`
  closures observe the same lift semantics. Unannotated closures
  keep the historical inline emit.

### Notable Breaking Changes

- **`form! on_success:` lift requires `Send + Sync`** ([#4623](https://github.com/kent8192/reinhardt-web/issues/4623), [#4624](https://github.com/kent8192/reinhardt-web/issues/4624)) — type-annotated `on_success: |value: T| ...` closures are now lifted to the outer construction block and therefore must be `Send + Sync`; unannotated closures (`|value|`, `|_value|`) are unaffected.
- **`#[export_endpoints]` removed in favour of `define_views!` / `flatten_imports!`** ([#3768](https://github.com/kent8192/reinhardt-web/discussions/3768), [#3783](https://github.com/kent8192/reinhardt-web/discussions/3783)) — multi-file view modules use the renamed declarative macro for stable-Rust compatibility.
- **`is_safe_url` inlined into `pages-macros`** — the macro no longer pulls in `reinhardt-core` as a dependency; downstream code that imported the helper through this crate should source it from `reinhardt-core` directly.

### Migration Notes

- Annotate `on_success` closure parameters with explicit types
  (`|value: T| ...`) when you need to capture enclosing-scope
  locals from `form!` bodies; ensure captured types are `Send +
  Sync`.
- Replace any `#[export_endpoints]` attribute usage with the
  `define_views!` / `flatten_imports!` declarative macro from
  `reinhardt-pages`.
- For the workspace-wide migration narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
