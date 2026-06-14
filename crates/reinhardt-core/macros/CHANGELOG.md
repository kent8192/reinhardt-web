# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-macros@v0.1.3...reinhardt-macros@v0.2.0) - 2026-06-11

Stable release of `reinhardt-macros` for the Reinhardt 0.2.0 line. This
entry consolidates the 0.2.0 release-candidate series; the original
RC entries remain below as detailed history.

### Migration Notes

- Drop removed `#[routes(...)]` compatibility flags and use plain `#[routes]` factories returning `UnifiedRouter`.
- Replace generated flat route accessors with explicit reverse lookups.
- See [`instructions/MIGRATION_0.2.md`](../../../instructions/MIGRATION_0.2.md) for the workspace migration checklist.

### Breaking Changes

- *(core)* [**breaking**] remove 0.1.0-rc deprecated URL resolver codegen (refs [[#4520](https://github.com/kent8192/reinhardt-web/issues/4520)](https://github.com/kent8192/reinhardt-web/issues/4520))
- *(macros)* [**breaking**] generate Info companion type via #[model] macro
- *(model)* [**breaking**] make new an alias for build

### Added

- *(orm)* allow builder overrides for generated fields
- *(settings)* generate embedded node schemas
- *(settings)* expose composed schema roots
- The `#[settings]` macro now generates typed embedded settings node schemas,
  supports `#[setting(node)]` and `#[setting(leaf)]` shape hints, and peels
  `Option`, `Vec`, `HashMap<String, _>`, `BTreeMap<String, _>`,
  `IndexMap<String, _>`, and `Box` wrappers for schema reference generation.
- *(core)* [**breaking**] remove 0.1.0-rc deprecated URL resolver codegen (refs [[#4520](https://github.com/kent8192/reinhardt-web/issues/4520)](https://github.com/kent8192/reinhardt-web/issues/4520))
- *(macros)* [**breaking**] generate Info companion type via #[model] macro
- *(model)* [**breaking**] make new an alias for build
- *(macros)* compile-time kebab-case URL-name warning; drop dead url-resolver codegen
- The HTTP route macros (`#[get]`, `#[post]`, `#[put]`, `#[patch]`,
  `#[delete]`) now emit a compile-time warning when an explicit `name = "..."`
  is not kebab-case, suggesting the kebab-case form to match ViewSet-generated
  names. Prefix the name with `!` to opt out, or set
  `REINHARDT_URL_NAME_WARNINGS=0` to silence it. Names that default to the
  function identifier are exempt. Refs
  [#4901](https://github.com/kent8192/reinhardt-web/issues/4901).

- *(macros)* expose model info companions to wasm

### Changed

- *(settings)* share schema macro analysis
- *(auth)* make CurrentUser canonical extractor

### Removed

- Removed the vestigial per-route URL-resolver metadata codegen
  (`generate_url_resolver_tokens` / `__url_resolver_meta_*`) from the HTTP route
  macros. Its consumer (`ResolvedUrls` / `__for_each_url_resolver`) was removed
  with the URL routing simplification (#4784), and the leftover codegen also
  rejected hyphenated (kebab-case) route names with a hard `compile_error!`.
  Route names passed to `#[get]` and friends may now be kebab-case. Refs
  [#4901](https://github.com/kent8192/reinhardt-web/issues/4901).

### Fixed

- *(settings)* harden schema macro parsing
- *(settings)* detect serde defaults in schema fields
- *(settings)* classify embedded config nodes
- *(settings)* require explicit nested settings nodes
- *(settings)* keep schema accessor compatibility
- *(settings)* simplify schema case conversion
- *(settings)* preserve cfg gates in schema generation
- stop implicit openapi schema macro output
- *(core)* drop leftover empty test definition in viewset_macro tests (refs [[#4520](https://github.com/kent8192/reinhardt-web/issues/4520)](https://github.com/kent8192/reinhardt-web/issues/4520))
- *(macros)* exclude pinned state params from builder impl generics
- *(macros)* propagate serde derives to Info companion struct via model_config
- *(macros)* remove unused has_derive_trait from model_derive
- *(macros)* suppress missing_docs on generated Info companion types
- *(macros)* keep unnamed EndpointMetadata.name None across codegen paths
- *(conf)* emit fragment self settings impls

### Documentation

- *(release)* enforce public API doc coverage
- *(settings)* document embedded schema nodes


## [0.2.0-rc.5](https://github.com/kent8192/reinhardt-web/compare/reinhardt-macros@v0.2.0-rc.4...reinhardt-macros@v0.2.0-rc.5) - 2026-06-11

### Added

- *(orm)* allow builder overrides for generated fields

### Documentation

- *(release)* enforce public API doc coverage

## [0.2.0-rc.4](https://github.com/kent8192/reinhardt-web/compare/reinhardt-macros@v0.2.0-rc.3...reinhardt-macros@v0.2.0-rc.4) - 2026-06-06

### Added

- *(settings)* generate embedded node schemas
- *(settings)* expose composed schema roots

### Changed

- *(settings)* share schema macro analysis
- *(auth)* make CurrentUser canonical extractor

### Documentation

- *(settings)* document embedded schema nodes

### Fixed

- *(settings)* harden schema macro parsing
- *(settings)* detect serde defaults in schema fields
- *(settings)* classify embedded config nodes
- *(settings)* require explicit nested settings nodes
- *(settings)* keep schema accessor compatibility
- *(settings)* simplify schema case conversion
- *(settings)* preserve cfg gates in schema generation

### Added

- The `#[settings]` macro now generates typed embedded settings node schemas,
  supports `#[setting(node)]` and `#[setting(leaf)]` shape hints, and peels
  `Option`, `Vec`, `HashMap<String, _>`, `BTreeMap<String, _>`,
  `IndexMap<String, _>`, and `Box` wrappers for schema reference generation.

## [0.2.0-rc.3](https://github.com/kent8192/reinhardt-web/compare/reinhardt-macros@v0.2.0-rc.2...reinhardt-macros@v0.2.0-rc.3) - 2026-06-05

### Fixed

- stop implicit openapi schema macro output

## [0.2.0-rc.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-macros@v0.1.3...reinhardt-macros@v0.2.0-rc.2) - 2026-06-03

### Added

- *(core)* [**breaking**] remove 0.1.0-rc deprecated URL resolver codegen (refs [[#4520](https://github.com/kent8192/reinhardt-web/issues/4520)](https://github.com/kent8192/reinhardt-web/issues/4520))
- *(macros)* [**breaking**] generate Info companion type via #[model] macro
- *(model)* [**breaking**] make new an alias for build
- *(macros)* compile-time kebab-case URL-name warning; drop dead url-resolver codegen

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Fixed

- *(core)* drop leftover empty test definition in viewset_macro tests (refs [[#4520](https://github.com/kent8192/reinhardt-web/issues/4520)](https://github.com/kent8192/reinhardt-web/issues/4520))
- *(core)* address Copilot review feedback on PR [[#4713](https://github.com/kent8192/reinhardt-web/issues/4713)](https://github.com/kent8192/reinhardt-web/issues/4713)
- *(ci)* recover develop release-plz prerelease
- *(macros)* exclude pinned state params from builder impl generics
- *(macros)* address CodeRabbit review on model Info generation
- *(macros)* propagate serde derives to Info companion struct via model_config
- *(macros)* remove unused has_derive_trait from model_derive
- *(macros)* suppress missing_docs on generated Info companion types
- *(ci)* update test snapshots and assertions for v0.2.0 breaking changes
- *(macros)* keep unnamed EndpointMetadata.name None across codegen paths
- *(conf)* emit fragment self settings impls

### Other

- resolve conflicts with develop/0.2.0

### Added

- The HTTP route macros (`#[get]`, `#[post]`, `#[put]`, `#[patch]`,
  `#[delete]`) now emit a compile-time warning when an explicit `name = "..."`
  is not kebab-case, suggesting the kebab-case form to match ViewSet-generated
  names. Prefix the name with `!` to opt out, or set
  `REINHARDT_URL_NAME_WARNINGS=0` to silence it. Names that default to the
  function identifier are exempt. Refs
  [#4901](https://github.com/kent8192/reinhardt-web/issues/4901).

### Removed

- Removed the vestigial per-route URL-resolver metadata codegen
  (`generate_url_resolver_tokens` / `__url_resolver_meta_*`) from the HTTP route
  macros. Its consumer (`ResolvedUrls` / `__for_each_url_resolver`) was removed
  with the URL routing simplification (#4784), and the leftover codegen also
  rejected hyphenated (kebab-case) route names with a hard `compile_error!`.
  Route names passed to `#[get]` and friends may now be kebab-case. Refs
  [#4901](https://github.com/kent8192/reinhardt-web/issues/4901).

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
