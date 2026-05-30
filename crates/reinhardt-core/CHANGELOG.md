# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-core@v0.1.2...reinhardt-core@v0.2.0-rc.2) - 2026-05-30

### Added

- *(core)* [**breaking**] remove 0.1.0-rc deprecated URL resolver codegen (refs [[#4520](https://github.com/kent8192/reinhardt-web/issues/4520)](https://github.com/kent8192/reinhardt-web/issues/4520))
- *(pages)* support keyed page list rendering

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Documentation

- *(core)* scaffold MIGRATION_0.2.md and document BREAKING CHANGES (refs [[#4520](https://github.com/kent8192/reinhardt-web/issues/4520)](https://github.com/kent8192/reinhardt-web/issues/4520))

### Fixed

- *(core)* drop leftover empty test definition in viewset_macro tests (refs [[#4520](https://github.com/kent8192/reinhardt-web/issues/4520)](https://github.com/kent8192/reinhardt-web/issues/4520))
- *(core)* address Copilot review feedback on PR [[#4713](https://github.com/kent8192/reinhardt-web/issues/4713)](https://github.com/kent8192/reinhardt-web/issues/4713)
- *(ci)* recover develop release-plz prerelease
- *(auth)* replace InternalUser in UserManager public API with ManagedUser
- *(ci)* resolve all pre-existing compilation failures on develop/0.2.0
- *(macros)* propagate serde derives to Info companion struct via model_config
- *(macros)* remove unused has_derive_trait from model_derive
- *(macros)* suppress missing_docs on generated Info companion types
- *(ci)* update test snapshots and assertions for v0.2.0 breaking changes
- *(core)* dispose Memo only on last clone drop
- *(core)* drop disposed-flag clone from Memo compute closure
- *(core)* drop unused mut on Memo::new parameter

### Other

- resolve conflicts with develop/0.2.0

### Added

- `reactive::deps` module with `Trackable` trait, `Deps` opaque container, and
  `IntoDeps` for tuples arity 0..=12. Enables the React-aligned
  `(closure, deps)` hook signatures in `reinhardt-pages` (#4195).
- `Effect::new_with_deps` and `Effect::new_with_deps_and_timing` constructors
  with Option A semantics (closure runs without active Observer; only listed
  deps subscribe) and optional `FnOnce` cleanup return.
- `Memo::new_with_deps` constructor mirroring the same Option A semantics for
  derived values. Adds an internal `MEMO_DIRTY` thread-local for type-agnostic
  invalidation by a hidden Layout-timing Effect that subscribes to the deps.
- `impl Trackable for Signal<T>` and `impl Trackable for Memo<T>`, enabling
  these primitives to participate in hook deps tuples.

### Removed

#### BREAKING CHANGES

All `reinhardt-core` public APIs deprecated during the `0.1.0-rc.*`
cycle have been removed per STABILITY_POLICY § SP-4 ("APIs deprecated
during RC MUST survive until the next major version"). Refs umbrella
Issue [#4520](https://github.com/kent8192/reinhardt-web/issues/4520).

`reinhardt-core` removals (8 macro-emitted items):

- **`#[routes]` deprecated 2-level URL accessor codegen** (rc.16) —
  `urls.<app>()` is removed. Use the namespaced gateway
  `urls.server().<app>()` instead. Affects every project that depends
  on `#[routes]` and called the 2-level accessor.
- **`#[routes]` deprecated 2-level client URL accessor codegen**
  (rc.16) — `urls.<app>_client()` is removed. Use
  `urls.client().<app>()` instead.
- **`#[get(name = "...")]` / `#[post(name = "...")]` deprecated per-route
  resolver-trait codegen** (rc.16) — the legacy `Resolve<Name>` trait
  blanket-impl that produced flat `urls.<name>(...)` calls is removed.
  Use the namespaced accessors `urls.server().<app>().<name>(...)`
  emitted by the same macros.
- **`#[viewset]` flat ViewSet accessor codegen** (rc.29, Issue
  [#4507](https://github.com/kent8192/reinhardt-web/issues/4507)) —
  the `Resolve<Pascal>List` / `Resolve<Pascal>Detail` traits and the
  matching `urls.<basename>_list()` / `urls.<basename>_detail(id)`
  flat accessors are removed (4 generated items). Use
  `urls.server().<app>().<basename>_list()` /
  `urls.server().<app>().<basename>_detail(id)` instead.
- **`impl UrlResolverUnprefixed for ResolvedUrls`** override emitted by
  `#[routes]` — removed because the flat ViewSet accessor that
  required namespace-iterating fallback no longer exists. The
  `UrlResolverUnprefixed` trait itself is removed in
  `reinhardt-urls` PR.

See [`instructions/MIGRATION_0.2.md`](../../instructions/MIGRATION_0.2.md#reinhardt-core)
for the full migration guide.

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-core@v0.1.0-rc.30...reinhardt-core@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-core` as part of the reinhardt-web
0.1.0 release. This crate is the foundation of the framework: it owns
the cross-cutting type system, the reactive signal runtime, the request
dispatch surface that route / action / WebSocket macros expand into,
and the security primitives (sanitization, validation, resource limits)
that every other Reinhardt crate consumes.

For the workspace-wide release narrative (Highlights, Breaking
Changes, Migration Guide), see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is preserved in the
[Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **Reactive signal runtime** — `Signal<T>`, `Effect`, and `Resource`
  power the reinhardt-pages reactive layer. Signals are `Sync` on
  native via `Arc<RwLock<T>>`, are WASM-compatible, and the runtime
  exposes `#[doc(hidden)]` diagnostic accessors (`debug_subscribers`,
  `debug_dependencies`, `debug_observer_stack`, `debug_pending_updates`)
  for cross-crate WASM tests ([#4088](https://github.com/kent8192/reinhardt-web/issues/4088)).
- **Request dispatch primitives for route / action / WebSocket macros**
  — sets the task-local resolve context, forks the per-request DI
  context, surfaces async-capable `#[routes]` handlers, and exposes
  `AuthProtection` plus `EndpointMetadata` so route macros can detect
  auth parameters and propagate the resulting metadata automatically.
- **`use_endpoint!` and `flatten_imports!`** — multi-file view modules
  expose their endpoints through `use_endpoint!` for resolver re-export,
  and `flatten_imports!` (renamed from `define_views!`) replaces the
  removed `#[export_endpoints]` attribute for stable-Rust compatibility
  ([#3783](https://github.com/kent8192/reinhardt-web/discussions/3783)).
- **Auth scaffolding (`SuperuserInit`, `SuperuserCreator`)** — the
  registry-backed `SuperuserCreator` is auto-populated via `inventory`
  whenever a `#[user(full = true)]` + `#[model]` type is declared,
  enabling `manage createsuperuser` to bootstrap any user model.
- **Compile-time security primitives** — `validate_html_attr_name`,
  `is_safe_url` (with anchor-link support), redirect-URL validation,
  HTML / CSS / script escaping, multipart body limits, decompression-
  bomb prevention, HMAC-SHA256 cursor integrity, and a runtime resource-
  limits configuration shared by `reinhardt-http` / `reinhardt-pages` /
  `reinhardt-rest`.
- **Settings primitives backing `#[settings]`** — `CoreSettings` is the
  required base fragment, and the macro now generates `HasSettings<F>`
  impls and `field_policies()` from `#[setting(...)]` attribute blocks
  so consumers can compose fragments without losing per-field policy
  data.
- **OpenAPI / REST hooks** — operation-level `#[rest::*]` route
  attributes contribute OpenAPI metadata to `reinhardt-rest` without
  forcing a hard dependency on the REST crate.
- **Workspace-wide invariants** — UUIDs are emitted as v7 throughout
  the codebase, glob imports have been replaced with explicit `pub use`
  re-exports across the validators / rayon preludes, and all relative
  paths beyond `../` are eliminated per project policy.

### Notable Breaking Changes

- **`#[url_patterns]` becomes typed** ([#3770](https://github.com/kent8192/reinhardt-web/discussions/3770))
  — accepts `InstalledApp::*` identifiers and `mode = server|client|unified|ws`;
  pattern functions are renamed accordingly. `reinhardt-core`'s
  dispatch macros consume the typed form.
- **DI unifies on `Depends<T>`** ([#3628](https://github.com/kent8192/reinhardt-web/discussions/3628))
  and **`Injected<T>` is deprecated** ([#3631](https://github.com/kent8192/reinhardt-web/discussions/3631))
  — `#[inject]` no longer accepts `Arc<T>` directly; `Depends<T>`
  adds caching, cycle detection, and DI metadata. The auto-`Clone`
  bound is removed.
- **`#[routes]` is async-capable** — handler signatures may be
  `async fn`; synchronous handlers remain supported.
- **`DependencyRegistration` is const-compatible** for Rust 2024
  edition.
- **`#[settings]` requires explicit `CoreSettings`** and emits
  `HasSettings<F>` impls in both attribute forms.
- **`flatten_imports!` replaces `define_views!`** ([#3783](https://github.com/kent8192/reinhardt-web/discussions/3783)),
  which itself replaced `#[export_endpoints]` ([#3768](https://github.com/kent8192/reinhardt-web/discussions/3768)).

### Migration Notes

See the [root Migration Guide](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#migration-guide)
for the full per-feature migration steps. The high-value moves for
`reinhardt-core` consumers are:

- Switch every `#[inject] Arc<T>` site to `#[inject] Depends<T>` and
  drop redundant `#[derive(Clone)]` bounds.
- Replace `Injected<T>` / `OptionalInjected<T>` with `Depends<T>` /
  `Option<Depends<T>>`.
- Add an explicit `CoreSettings` fragment to any `#[settings]` block
  that previously relied on the implicit one, and migrate
  `#[export_endpoints]` views to `flatten_imports!`.
