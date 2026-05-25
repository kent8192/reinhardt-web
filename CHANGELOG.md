# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-web@v0.1.1...reinhardt-web@v0.2.0-rc.2) - 2026-05-25

### Fixed

- address CodeRabbit review comments
- address remaining CodeRabbit comments
- address Copilot review comments
- address follow-up CodeRabbit comments
- *(ci)* recover develop release-plz prerelease

### Maintenance

- forward merge main v0.1.1 changes into develop 0.2.0
- include all main v0.1.1 PR changes

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-web@v0.1.0-rc.30...reinhardt-web@v0.1.0) - 2026-05-22

First stable release of `reinhardt-web`, after 19 alpha and 30 rc
prereleases dating back to 2026-01-23. The entry below is a curated
feature-level summary; per-prerelease commit history is available in
the [Release-category Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Highlights

- **Type-safe, convention-driven framework**: reinhardt-web brings
  Django-like ergonomics to Rust with fully typed URL routing,
  dependency injection, and form validation that fail at compile time,
  not runtime.
- **Full-stack WASM SPA support**: build reactive client UIs with
  Server Functions that auto-serialize across the network boundary,
  complete with CSRF protection and pluggable JWT / session
  authentication.
- **Pragmatic admin interface**: the built-in admin panel supports
  role-based permissions, type-safe query filters, and integrated ORM
  operations without separate admin declarations — just `#[model]` and
  opt in.
- **Async-first, runtime-agnostic**: every public API is async by
  default and integrates cleanly with Tokio, with TestContainers-backed
  integration tests for PostgreSQL, MySQL, SQLite, and CockroachDB.

### Breaking Changes

The list below names every breaking change introduced across the
alpha / rc lifecycle. Each entry links to its announcement in the
[Breaking Changes Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/breaking-changes);
follow the **Migration Guide** below for the most disruptive moves.

- **Dependency injection unifies on `Depends<T>`** ([#3628](https://github.com/kent8192/reinhardt-web/discussions/3628)) — `#[inject]` no longer accepts `Arc<T>` directly; use `Depends<T>` so the framework can cache resolution, detect cycles, and surface DI metadata.
- **`Injected<T>` deprecated** ([#3631](https://github.com/kent8192/reinhardt-web/discussions/3631)) — replaced by `Depends<T>`; the auto-`Clone` bound is removed.
- **`#[url_patterns]` becomes typed** ([#3770](https://github.com/kent8192/reinhardt-web/discussions/3770)) — accepts `InstalledApp::*` identifiers with `mode = server|client|unified`; pattern functions are renamed accordingly.
- **`#[viewset]` macro and route mounting** ([#4476](https://github.com/kent8192/reinhardt-web/discussions/4476)) — per-app `server_fn` and client UI now live under `apps/<app>/`; `commands/templates/...` no longer carries handler code.
- **URL resolver directory layout** ([#3918](https://github.com/kent8192/reinhardt-web/discussions/3918)) — `ws_urls.rs` and friends move under `src/apps/<app>/urls/`.
- **`AdminUser` trait signature** ([#3615](https://github.com/kent8192/reinhardt-web/discussions/3615)) — `ModelAdmin` permission methods accept `&dyn AdminUser` instead of `&(dyn Any + Send + Sync)`.
- **`admin_routes_with_di()`** ([#3626](https://github.com/kent8192/reinhardt-web/discussions/3626)) — the new entry point applies middleware-contributed DI registrations; legacy `admin_routes()` is removed.
- **`AdminRoute` is `#[non_exhaustive]`** — match arms must include a default fallback; the `Login` variant moved position.
- **OAuth2 `exchange_code` redirect URI** ([#3609](https://github.com/kent8192/reinhardt-web/discussions/3609)) — fourth argument is the callback URL.
- **Typed TOML interpolation** ([#4241](https://github.com/kent8192/reinhardt-web/discussions/4241), [#4229](https://github.com/kent8192/reinhardt-web/discussions/4229)) — `${VAR}` placeholders in TOML coerce to the destination type; opt out with `SettingsBuilder::with_typed_coercion(false)`.
- **`ProjectSettings` replaces `env::var`** in commands/db ([#4295](https://github.com/kent8192/reinhardt-web/discussions/4295)) and `VersioningSettings` in rest ([#4294](https://github.com/kent8192/reinhardt-web/discussions/4294)).
- **`ClientLauncher::on_navigate`** ([#4117](https://github.com/kent8192/reinhardt-web/discussions/4117)) — SPA navigation routes through `Router::on_navigate` instead of relaunching the client per navigation.
- **`client_router::history` dedup** ([#4219](https://github.com/kent8192/reinhardt-web/discussions/4219)) — the duplicate `history` module under `client_router` is removed; consume `pages::router::history` instead.
- **`#[routes]` is async-capable** — route handler signatures may be `async fn`; sync handler ABIs remain supported but no longer represent the canonical shape.
- **`define_views!` → `flatten_imports!`** ([#3783](https://github.com/kent8192/reinhardt-web/discussions/3783)) — multi-file view modules use the renamed declarative macro; `#[export_endpoints]` is removed.
- **`JsonFileSource` / `auto_source` deprecated** ([#4120](https://github.com/kent8192/reinhardt-web/discussions/4120)) — prefer `TomlFileSource` (default interpolation enabled).
- **`#[user(...)]` requires explicit `LABEL`** on `AppLabel` implementors, and emits a `BaseUserManager` impl in 0.1.0.
- **`unique_together` propagates into `ModelMetadata`** ([#4027](https://github.com/kent8192/reinhardt-web/discussions/4027)) — autodetector consumes it; existing migrations regenerate.
- **Admin form widget HTML elements** ([#3771](https://github.com/kent8192/reinhardt-web/discussions/3771)) — `TextArea`, `Select`, and `MultiSelect` render as their semantic HTML elements rather than `<input>`.
- **`manouche` IR / IRVisitor removed** ([#3900](https://github.com/kent8192/reinhardt-web/discussions/3900)) — internal codegen path no longer exposes the IR layer.
- **`examples-rest` layout** ([#4476](https://github.com/kent8192/reinhardt-web/discussions/4476)) — `standalone` flag and `USE_VIEWSET` toggle dropped; `urls/` restructured for `ResolvedUrls`; Bruno collection split per ViewSet.
- **`infra/repository` reviewer lookup** ([#4152](https://github.com/kent8192/reinhardt-web/discussions/4152)) — Terraform module replaces the `data "github_user"` lookup with a numeric user-ID variable.

### Migration Guide

The 0.1.0 stable release consolidates 19 alpha and 30 rc prereleases.
Notable breaking changes since 0.1.0-rc.1 are summarized below; the
complete list is in the **Breaking Changes** section above.

- **Typed URL routing** ([#3770](https://github.com/kent8192/reinhardt-web/discussions/3770)):
  replace string-based `#[url_patterns]` with
  `#[url_patterns(InstalledApp::app_name, mode = server|client|unified)]`
  and rename functions to `server_url_patterns()` /
  `client_url_patterns()` / `unified_url_patterns()`.
- **Dependency injection unification**
  ([#3628](https://github.com/kent8192/reinhardt-web/discussions/3628)):
  replace `#[inject] Arc<T>` with `#[inject] Depends<T>` across every
  injection site. `Depends<T>` adds caching, circular-dependency
  detection, and metadata that bare `Arc<T>` lacked.
- **Deprecate `Injected<T>`**
  ([#3631](https://github.com/kent8192/reinhardt-web/discussions/3631)):
  migrate `Injected<T>` and `OptionalInjected<T>` to `Depends<T>` and
  `Option<Depends<T>>`. Add an explicit `#[derive(Clone)]` if your type
  was relying on the previous auto-Clone behaviour.
- **`AdminUser` trait signature**
  ([#3615](https://github.com/kent8192/reinhardt-web/discussions/3615)):
  update `ModelAdmin` permission methods to accept `&dyn AdminUser`
  instead of `&(dyn std::any::Any + Send + Sync)`.
- **OAuth2 `exchange_code` redirect URI**
  ([#3609](https://github.com/kent8192/reinhardt-web/discussions/3609)):
  `exchange_code()` now requires a `redirect_uri` parameter; pass the
  callback URL as the fourth argument.
- **Typed TOML interpolation**
  ([#4241](https://github.com/kent8192/reinhardt-web/discussions/4241)):
  environment-variable interpolation in TOML (e.g.,
  `${REINHARDT_DB_PORT}`) now coerces to the target type. Opt out with
  `SettingsBuilder::with_typed_coercion(false)`.
- **URL resolver restructuring**
  ([#3918](https://github.com/kent8192/reinhardt-web/discussions/3918)):
  move `src/apps/<app>/ws_urls.rs` to `src/apps/<app>/urls/ws_urls.rs`
  and declare it in the `urls` submodule.
- **`define_views!` replaces `#[export_endpoints]`**
  ([#3768](https://github.com/kent8192/reinhardt-web/discussions/3768)):
  use the `define_views!` declarative macro for multi-file view
  modules — the attribute form was removed for stable-Rust
  compatibility (later renamed to `flatten_imports!`).
- **Apps relocate per-app handlers**
  ([#4476](https://github.com/kent8192/reinhardt-web/discussions/4476)):
  per-app `server_fn` and client UI moved from `commands/templates/...`
  into `apps/<app>/`. Update existing apps by relocating the matching
  source files; `reinhardt new` already emits the new layout.
- **`ClientLauncher::on_navigate`**
  ([#4117](https://github.com/kent8192/reinhardt-web/discussions/4117)):
  client SPA navigation now hooks through `Router::on_navigate` rather
  than launching a fresh router per navigation; remove any manual
  `ClientLauncher::launch` wiring tied to the old model.
- **`admin_routes_with_di()`**
  ([#3626](https://github.com/kent8192/reinhardt-web/discussions/3626)):
  use `admin_routes_with_di()` instead of the deprecated
  `admin_routes()` so middleware-contributed DI registrations are
  applied.

For the complete per-PR change list, see the
[Release-category Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Added

#### ORM & Database

- `#[model]` derives Django-style model traits, including
  `Model::build()` typestate constructor, `ForeignKey<T>` and
  `ManyToMany<T>` relations, and `#[field(skip = true)]` for non-DB
  fields.
- Multi-backend support: PostgreSQL, MySQL, SQLite, and CockroachDB
  via TestContainers in tests; per-backend `ALTER COLUMN` dispatch and
  CockroachDB sentinel-row locks replace generic advisory-lock paths.
- Schema autodetection covers unique constraints, M2M relations, and
  qualified `to_model = "app::Model"` references; M2M autodetection
  keys on `table_name` and snake-cases through-tables consistently.
- `reinhardt-query`: in-house SQL builder wrapping SeaQuery, used in
  place of raw SQL across the workspace.
- Prefetch and `filter_by_target` honour canonical M2M naming rules.

#### URL Routing & Apps

- `#[url_patterns]` macro generates typed `urls::*` helpers per app
  with compile-time path verification; `#[routes]` consumes them.
- `#[viewset]` macro for REST-style endpoint groups; named, mount,
  and unified routing modes.
- `VersionedRouter` trait and `RouteVersionInfo` value type share a
  router abstraction between `reinhardt-urls` and `reinhardt-rest`
  without circular crate dependencies (introduced via
  `reinhardt-router`).
- Middleware DI registrations propagate through `with_middleware`,
  `group()`, and `pending_di` queues so the router rebuilds DI no
  matter the builder order.

#### Dependency Injection

- `Depends<T>` smart wrapper with caching, cycle detection, and
  metadata; `#[inject]` parameters express dependencies declaratively.
- `Middleware::di_registrations` hook + type-erased DI APIs allow
  middleware to contribute its own dependencies (e.g., `SessionStore`
  auto-registered by `SessionMiddleware`).
- `DependencyRegistration` is const-compatible for Rust 2024 edition.
- Testing helpers: `register_override` with `OverrideGuard`, and the
  `with_di_overrides!` macro shipped from `reinhardt-testkit-macros`.

#### Pages, WASM, and Server Functions

- `page!` declarative DSL for component bodies (70+ HTML elements
  validated at compile time) with `if`/`else`, `for`, `watch`, and
  reactive `Signal<T>` bindings.
- `head!` macro for SSR head metadata.
- `form!` macro: typed form fields, widgets, validators (client &
  server), CSRF protection, two-way Signal binding, computed values,
  field groups, slots, and dynamic `choices_loader`.
- `#[server_fn]` macro generates the WASM client stub and the
  server-side handler; codec selection (`json`, `url`, `msgpack`).
- `use_router()` hook, `RouterHandle`, `navigate()` free function,
  and `try_with_spa_router` for graceful SPA fallback.
- HMR (`reinhardt-pages` feature flag): WebSocket-driven CSS / DOM
  patching, scheme selected automatically based on
  `window.location.protocol`.

#### HTTP, REST, and Middleware

- `reinhardt-http` request / response abstractions; sanitization
  helpers (`validate_html_attr_name`, `is_safe_url`, anchor-link
  support, XSS prevention) and resource-limit configuration.
- `reinhardt-rest`: ViewSets with operation-level OpenAPI attributes,
  versioning via `VersioningSettings`, OpenAPI macros under
  `reinhardt-openapi-macros`.
- `reinhardt-middleware`: session middleware, CORS, gzip, etc.

#### Authentication & Authorization

- JWT + session-cookie auth, OAuth2 (`GenericOidcProvider` for
  arbitrary OIDC IdPs), and pluggable user managers.
- `SuperuserInit` trait and `SuperuserCreator` registry; auto-register
  via `inventory` for `#[user(full = true)]` + `#[model]` types.
- `AuthProtection` enum + `EndpointMetadata` propagate auth
  requirements; route macros detect auth parameters and produce the
  metadata automatically.

#### Admin Interface

- `#[model]`-driven admin pages with role-based permissions,
  type-safe query filters, and form-based CRUD.
- `reinhardt-admin-cli`: CLI for scaffolding admin pages,
  `cargo install --path crates/reinhardt-admin-cli` ships with the
  workspace.
- `admin_routes_with_di()` entry point + `AdminRoute` (now
  `#[non_exhaustive]`).

#### Configuration & Settings

- `SettingsBuilder` layered configuration with `TomlFileSource`,
  `EnvFileSource`, and typed interpolation enabled by default.
- `#[settings(...)]` macro requires explicit `CoreSettings` and emits
  `HasSettings<F>` impls for composed settings; `#[setting(...)]`
  attribute drives `field_policies()` generation.

#### Testing

- `reinhardt-test` fixtures for TestContainers-backed Postgres, MySQL,
  SQLite, CockroachDB, Redis, and Kafka instances.
- Module-scoped Kafka fixture with `KafkaConfig.partitions`.
- `reinhardt-testkit` and `reinhardt-testkit-macros` provide
  `with_di_overrides!` and `DiOverrideBuilder` for testing-only DI
  overrides.

#### Other Integrations

- GraphQL handler via `reinhardt-graphql` (+ macros).
- gRPC handler via `reinhardt-grpc` (+ macros).
- WebSocket router via `reinhardt-websockets`.
- Mail delivery via `reinhardt-mail` (pluggable transports).
- i18n via `reinhardt-i18n` (fluent-style messages).
- Throttling via `reinhardt-throttling`.
- Streaming integration via `reinhardt-streaming` (Kafka).
- Deep-link helpers via `reinhardt-deeplink`.
- Keyboard shortcuts via `reinhardt-shortcuts`.

#### Tooling & Examples

- `reinhardt new` scaffold emits the per-app layout matching
  [#4476](https://github.com/kent8192/reinhardt-web/discussions/4476).
- `examples-tutorial-basis` (Django-tutorial parity), `examples-rest`
  (REST-API tutorial with Bruno collections), and additional samples
  cover the most common entry points.

### Changed

- Cross-cutting refactor of the URL/routing macro internals for a
  smaller, easier-to-read generated TokenStream.
- ORM accessors and autodetector consolidate on a single
  `crate::naming::to_snake_case` source.
- Setting-related `glob` imports replaced with explicit `pub use`
  re-exports; explicit `rayon` trait imports.
- WASM compilation: client URL accessors namespaced per app on
  `wasm32`, with `#[cfg]` gating in generated tokens to avoid stale
  symbols.
- Migrations now dispatch `ALTER COLUMN TYPE` per backend (MySQL
  `MODIFY`, SQLite recreate, PostgreSQL / CockroachDB direct ALTER).

### Deprecated

- `Injected<T>` and `OptionalInjected<T>` — use `Depends<T>` /
  `Option<Depends<T>>` ([#3631](https://github.com/kent8192/reinhardt-web/discussions/3631)).
- `admin_routes()` — use `admin_routes_with_di()` ([#3626](https://github.com/kent8192/reinhardt-web/discussions/3626)).
- `JsonFileSource`, `auto_source` — use `TomlFileSource` ([#4120](https://github.com/kent8192/reinhardt-web/discussions/4120)).
- `define_views!` — renamed to `flatten_imports!` ([#3783](https://github.com/kent8192/reinhardt-web/discussions/3783)).
- `#[export_endpoints]` — removed in favour of `define_views!` /
  `flatten_imports!` ([#3768](https://github.com/kent8192/reinhardt-web/discussions/3768)).

### Fixed

- WASM compatibility for the reactive runtime: `Signal<T>` is `Sync`
  on native via `Arc<RwLock<T>>`; `flush_updates` unifies pending
  effects.
- `#[routes]` no longer panics on `extern` identifiers; reserved-ident
  set excludes `extern`.
- `form!` macro: `on_success` and `success_url` correctly hoist
  before navigation; `watch` and `initial` capture outer scope.
- `#[user]` integration tests opt out of auto-manager when the
  fixture supplies one.
- Migrations: split multi-statement reverse SQL; emit `ALTER COLUMN`
  reverse as a single comma-separated statement.
- Static-files handler disables immutable caching for bundle assets
  in debug builds so HMR can replace them.
- Per-app `installed_apps` state file is namespaced by
  `CARGO_CRATE_NAME` so concurrent macro expansion does not race.
- Type-erased proc-macro closures use an `if false { ... }`
  dead-code branch with `unreachable!()` arguments to unify the
  user-supplied generic at compile time (#4624 / #4627).

### Security

- HTML attribute / URL sanitization helpers shipped to prevent the
  most common XSS sinks.
- CSRF protection wired into `form!` for non-GET methods.
- OAuth2 `exchange_code` requires an explicit `redirect_uri` so
  attacker-controlled callbacks cannot be substituted post-issue.

### Performance

- Single-lock manager operations in core-macros generated code.
- Per-app endpoint inventory keyed on `CARGO_CRATE_NAME` removes a
  contention point during macro expansion.

For per-prerelease detail, see the
[Release-category Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release)
(rc.1 through rc.30 plus the alpha.7 / alpha.8 / alpha.9 announcement
posts).
