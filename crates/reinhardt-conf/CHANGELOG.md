# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Removed

#### BREAKING CHANGES

Partial removal of `0.1.0-rc.*` deprecated APIs per STABILITY_POLICY § SP-4
(umbrella Issue [#4520](https://github.com/kent8192/reinhardt-web/issues/4520)).
This PR removes the **4 items** that have no `Settings`-struct dependents
in the workspace; the remaining 4 (`Settings` struct itself and its
`add_app` / `with_validated_apps` methods) require coordinated migration
of `reinhardt-apps` and `reinhardt-middleware` and are handled in a
follow-up PR.

`reinhardt-conf` removals in this PR (4 items):

- **`AdvancedSettings` struct** (`src/settings/advanced.rs`, deprecated
  since `0.1.0-rc.16`) — use the individual fragment types
  (`CacheSettings`, `SessionSettings`, etc.) composed via
  `ProjectSettings` instead. The fragment types themselves are
  unchanged.
- **`TomlFileSource::set_interpolation(bool)`** (`src/settings/sources.rs`,
  deprecated since `0.1.0-rc.27`, refs Issue
  [#4224](https://github.com/kent8192/reinhardt-web/issues/4224)) — use
  `with_interpolation()` / `without_interpolation()` builder methods.
- **`JsonFileSource` struct + `ConfigSource` impl** (`src/settings/sources.rs`,
  deprecated since `0.1.0-rc.26`, refs Issue
  [#4087](https://github.com/kent8192/reinhardt-web/issues/4087)) — TOML
  is the canonical configuration format. Construct
  [`TomlFileSource`](src/settings/sources.rs) directly.
- **`auto_source(path)`** (`src/settings/sources.rs`, deprecated since
  `0.1.0-rc.26`, refs Issue
  [#4087](https://github.com/kent8192/reinhardt-web/issues/4087)) — call
  `TomlFileSource::new(path)` directly so the configuration format is
  explicit at the call site.

In-tree call sites:
- `crates/reinhardt-conf/tests/file_sources.rs` (whole file — JsonFileSource
  + auto_source tests) deleted
- `crates/reinhardt-conf/tests/source_priority.rs` (whole file — JsonFileSource
  priority tests) deleted
- `crates/reinhardt-conf/tests/settings_builder.rs` (whole file — deprecated
  `Settings` tests, removed in preparation for the follow-up PR) deleted
- `crates/reinhardt-conf/tests/profile_switching.rs` (whole file — deprecated
  `Settings` tests, removed in preparation for the follow-up PR) deleted
- `crates/reinhardt-conf/tests/interpolation.rs` — `deprecated_set_interpolation_still_works`
  test removed

See [`instructions/MIGRATION_0.2.md`](../../instructions/MIGRATION_0.2.md#reinhardt-conf)
for the migration guide.

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-conf@v0.1.0-rc.30...reinhardt-conf@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-conf` as part of the reinhardt-web
0.1.0 release. This crate is Reinhardt's Django-inspired settings
framework: it owns the `SettingsBuilder` layered configuration model,
the composable fragment system (`CoreSettings`, `SecuritySettings`,
`I18nSettings`, ...), the TOML / env source priority stack, and the
secrets / encryption primitives that protect sensitive values in
memory.

For the workspace-wide release narrative (Highlights, Breaking
Changes, Migration Guide), see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is preserved in the
[Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **Composable settings fragments** — `SettingsFragment` is the
  trait every fragment implements, with a `type Accessor` projection
  and a `field_policies()` hook. The crate ships `CoreSettings` as
  the required base plus Django-compat fragments (`I18nSettings`,
  `TemplateSettings`, `ContactSettings`, `OpenApiSettings`, ...).
  `ComposedSettings`, `HasSettings<F>`, and `HasCommonSettings`
  expose composition to user code.
- **Layered `SettingsBuilder` with priority sources** — the
  builder merges `TomlFileSource`, `EnvFileSource`,
  `DefaultSource`, `HighPriorityEnvSource`, and user-supplied
  sources by priority. `MergeStrategy::Deep` is the default
  (`build_composed`), and per-test override sources let
  TestContainers integration tests inject configuration without
  touching real files.
- **Typed TOML interpolation** ([#4241](https://github.com/kent8192/reinhardt-web/discussions/4241),
  [#4229](https://github.com/kent8192/reinhardt-web/discussions/4229))
  — TOML strings support `${VAR}`, `${VAR:-default}`, `${VAR:-}`
  (explicit empty), and `${VAR:?message}` placeholders. The
  interpolator walks the full TOML AST (including strings nested in
  tables and arrays), and placeholders coerce to the destination
  field's type (e.g., `${REINHARDT_DB_PORT}` becomes a `u16`). Opt
  out of coercion with `SettingsBuilder::with_typed_coercion(false)`
  or disable interpolation entirely with `without_interpolation()`.
- **Field policies via `#[setting(...)]`** — the
  `FieldRequirement` / `FieldPolicy` types drive
  `BuildError::MissingRequiredField` and feed `build_composed()` so
  required-field errors surface at boot rather than as `None`
  values mid-request.
- **Secrets & encryption primitives** — `DatabaseUrl` redacts
  passwords in `Debug` output, the secrets module uses
  `ZeroizeOnDrop` with `ManuallyDrop` to preserve drop safety
  through `into_inner`, and credentials are URL-encoded when
  reassembled. Encryption-key exposure via CLI arguments is
  prevented, and hot-reload uses `tokio::sync::Mutex` to keep async
  reload paths free of `parking_lot` blocking.
- **Database-URL scheme validation as public API** — consumers
  (notably `reinhardt-apps::ApplicationBuilder::build`) reuse the
  same validator that the crate uses internally.

### Notable Breaking Changes

- **TOML interpolation is on by default** — `TomlFileSource::new(path)`
  now enables `${VAR}` interpolation; the previous opt-in behavior
  caused silent failures when a literal `${DB_PASSWORD}` landed in
  the merged tree. `with_interpolation()` is a no-arg explicitness
  marker; `without_interpolation()` opts out (issue #4224).
- **`set_interpolation(bool)` is deprecated** — use
  `with_interpolation()` / `without_interpolation()` instead; the
  boolean setter will be removed in 0.2.0.
- **`JsonFileSource` and `auto_source` are deprecated** ([#4120](https://github.com/kent8192/reinhardt-web/discussions/4120))
  — TOML is the canonical Reinhardt configuration format. Migrate
  `.json` configuration files to `.toml` or implement
  `ConfigSource` against `serde_json` to keep JSON support
  out-of-tree.
- **`Settings.installed_apps` is deprecated** — installed apps
  flow through the `reinhardt-apps` registry; the legacy
  `Settings`-level field remains as a serde-flattened bridge but
  emits a deprecation warning.
- **Built-in fragments extracted from `AdvancedSettings`** —
  `AdvancedSettings` is deprecated in favour of explicit fragments
  (`SecuritySettings`, `I18nSettings`, `TemplateSettings`,
  `ContactSettings`, `OpenApiSettings`, `CoreSettings`).

### Migration Notes

See the [root Migration Guide](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#migration-guide)
for project-wide guidance. Settings-specific moves:

- If you relied on literal `${...}` strings in TOML, append
  `.without_interpolation()` to your `TomlFileSource` constructor.
- Replace `JsonFileSource::new(...)` / `auto_source(...)` with
  `TomlFileSource::new(...)` against a `.toml` file.
- Migrate from `AdvancedSettings` to the matching fragments
  (`SecuritySettings`, `I18nSettings`, ...) and add an explicit
  `CoreSettings` fragment to every composed-settings declaration.
