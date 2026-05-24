# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/kent8192/reinhardt-web/compare/reinhardt-shortcuts@v0.1.0...reinhardt-shortcuts@v0.1.1) - 2026-05-24

### Documentation

- update version references to v0.1.1

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-shortcuts@v0.1.0-rc.30...reinhardt-shortcuts@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-shortcuts` as part of the
reinhardt-web 0.1.0 release. Provides Django-style shortcut functions
(`redirect`, `render`, `get_object_or_404`) on top of `reinhardt-http`
and `reinhardt-views`.

For the workspace-wide release narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is in the [Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **Django-style shortcuts** — `redirect`, `render`, `render_to_string`,
  and `get_object_or_404` shortcuts that compose with typed URL
  resolvers and the template/page rendering stack, removing
  boilerplate from view handlers.
- **Hardened redirect and rendering** — Open-redirect attacks are
  blocked at the shortcut layer, hardcoded headers use
  `HeaderValue::from_static`, and `render_to_string` preserves data
  integrity while sanitizing 404 error output.
- **XSS-safe template rendering** — `render_html` documents its XSS
  contract and routes user input through `reinhardt_core`'s
  `escape_html` so shortcut callers cannot accidentally bypass
  escaping.
- **Optional ORM shortcut helpers** — Behind the `database` feature
  flag, helpers like `get_object_or_404` integrate with `reinhardt-db`
  without leaking database error messages into HTTP responses.
- **TemplateContext with capacity guard** — Template contexts are
  bounded by a configurable capacity to prevent runaway accumulation
  inside long-lived render pipelines.

### Notable Breaking Changes

`reinhardt-shortcuts` did not introduce its own framework-wide
breaking changes in 0.1.0. Workspace-level breaking changes are
tracked at the [Breaking Changes Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/breaking-changes)
and summarized in the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).

### Migration Notes

See the workspace-level [Migration Guide](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22)
for the full upgrade flow. Crate-specific notes:

- The `database` feature no longer pins the PostgreSQL backend; enable
  the desired backend feature on `reinhardt-db` explicitly.
- `From` conversions that previously allowed unvalidated URLs into
  `redirect()` were removed ([#726](https://github.com/kent8192/reinhardt-web/issues/726));
  validated `Url` types are now required at the boundary.
