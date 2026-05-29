# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-macros@v0.1.0-rc.30...reinhardt-macros@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-macros` as part of the
reinhardt-web 0.1.0 release. This crate ships the procedural macros
that power Reinhardt's "Django-like ergonomics" — `#[model]`,
`#[user]`, `#[routes]`, `#[viewset]`, `#[url_patterns]`, `#[settings]`,
`#[websocket]`, `#[dto]`, and the `flatten_imports!` declarative
macro. All other Reinhardt crates load their public API from these
expansions.

For the workspace-wide release narrative (Highlights, Breaking
Changes, Migration Guide), see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is preserved in the
[Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **`#[model]` with typestate `Model::build()`** — models expose a typestate
  builder whose setters carry `ForeignKeyField<T>` for FK columns,
  doc-comments per generated setter, and a hardened reserved-ident
  set (notably excluding `extern`). `#[field(skip = true)]` lets
  non-DB fields opt out, and a `manager = ...` argument selects a
  custom default manager.
- **`#[url_patterns]` typed routing macro** ([#3770](https://github.com/kent8192/reinhardt-web/discussions/3770))
  — accepts `InstalledApp::*` identifiers with `mode = server | client | unified | ws`,
  emits the `urls::*` typed-helper module (with binding-name parameter
  pairing and tightened `ClientPath` checks), and projects WASM-only
  client URL accessors per app via `#[cfg(target_arch = "wasm32")]`
  in the generated tokens.
- **`#[routes]` + `#[viewset]` + `#[websocket]`** — async-capable
  `#[routes]` ([#3770](https://github.com/kent8192/reinhardt-web/discussions/3770)),
  `#[viewset]` with explicit `basename = "..."` for fn-form viewsets
  (the body-token fallback is deprecated for v0.2.0), and a new
  `#[websocket]` macro that codegens a `Consumer` implementation
  plus the URL-resolver tokens scanned by `url_patterns(mode = ws)`.
- **`#[user(...)]`** — emits a `BaseUserManager` impl, injects the
  `ManyToMany` relationships expected by built-in apps, and feeds the
  `SuperuserCreator` `inventory` registry consumed by
  `manage createsuperuser`.
- **`#[settings]` attribute macro** ([#3783](https://github.com/kent8192/reinhardt-web/discussions/3783)
  — built on a nom v8 parser that understands fragment composition,
  `{ field: policy }` override blocks, and `#[setting(...)]` attribute
  blocks. The macro requires an explicit `CoreSettings` fragment and
  emits `HasSettings<F>` impls and `field_policies()` automatically.
- **`#[dto]` (formerly `#[shared_model]` / `#[shared_schema]`)** —
  generates the `cfg_attr(native, ...)` DTO boilerplate shared
  between server and WASM client; `#[derive(Validate)]` provides
  field-level validation including `range(min, max)`, replacing the
  external `validator` crate in `pre_validate` codegen.
- **`flatten_imports!` declarative macro** ([#3783](https://github.com/kent8192/reinhardt-web/discussions/3783))
  — multi-file view modules use the renamed macro for stable-Rust
  compatibility; the original `define_views!` is deprecated and the
  attribute-form `#[export_endpoints]` is removed ([#3768](https://github.com/kent8192/reinhardt-web/discussions/3768)).

### Notable Breaking Changes

- **Typed `#[url_patterns]`** ([#3770](https://github.com/kent8192/reinhardt-web/discussions/3770))
  — string-literal app names are replaced by `InstalledApp::*`
  identifiers with `mode = ...`; named-variant patterns are deprecated.
- **`#[viewset]` and route mounting** ([#4476](https://github.com/kent8192/reinhardt-web/discussions/4476))
  — viewset basename moved from a token-walker fallback to an
  explicit `basename = "..."` argument (hard error in v0.2.0).
- **`ws_url_resolvers` relocated under `urls/`** — WebSocket
  resolvers live under `src/apps/<app>/urls/`; `#[routes]` rustdoc
  documents the migration path.
- **DI / `Injected<T>` deprecation** ([#3628](https://github.com/kent8192/reinhardt-web/discussions/3628),
  [#3631](https://github.com/kent8192/reinhardt-web/discussions/3631))
  — generated code uses `Depends<T>` instead of `Arc<T>` /
  `Injected<T>`, and the auto-`Clone` bound is removed.
- **`AppLabel` implementors require explicit `LABEL`** ([#4476](https://github.com/kent8192/reinhardt-web/discussions/4476))
  — `#[app_config]` no longer derives `LABEL` from the type name.
- **`DependencyRegistration` is const-compatible** for Rust 2024
  edition; the macro emits the new const form.
- **`define_views!` deprecation** ([#3783](https://github.com/kent8192/reinhardt-web/discussions/3783))
  and **`#[export_endpoints]` removal** ([#3768](https://github.com/kent8192/reinhardt-web/discussions/3768)).

### Migration Notes

See the [root Migration Guide](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#migration-guide)
for the full per-feature migration steps. Macro-specific moves:

- Rewrite every `#[url_patterns("app_name")]` invocation as
  `#[url_patterns(InstalledApp::app_name, mode = ...)]` and rename
  the corresponding pattern functions.
- Replace `define_views! { ... }` with `flatten_imports! { ... }`
  and convert any remaining `#[export_endpoints]` modules.
- Pass `basename = "..."` explicitly on every fn-form `#[viewset]`.
