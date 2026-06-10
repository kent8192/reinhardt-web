# Migration Guide: 0.1.x to 0.2.0

Umbrella tracker: [#4520](https://github.com/kent8192/reinhardt-web/issues/4520).
Auth companion: [#4652](https://github.com/kent8192/reinhardt-web/issues/4652).

This guide covers only the delta from the public 0.1.x line to the final
0.2.0 line. It does not repeat migrations that were already required before
0.1.0 stabilized.

0.2.0 is a major-version upgrade. The main work is:

- remove APIs that were already deprecated in 0.1.x,
- update ORM/query call sites for the 0.2 contracts,
- move configuration to typed settings fragments,
- regenerate and review database migrations,
- verify facade feature flags for `default-features = false` users.

## Recommended order

1. Update `Cargo.toml` to the target 0.2 release.
2. Fix removed API references listed in the table below.
3. Update ORM/query call sites.
4. Move touched configuration code to settings fragments.
5. Regenerate migrations and review the diff.
6. Run the verification commands at the end of this guide.

## Removed API (already deprecated)

These APIs were deprecated during the 0.1.x line and are removed in 0.2.0.
Keep this section mechanical: replace the symbol, then let `cargo check`
surface any surrounding type changes.

| Crate | Removed API | Replacement |
|---|---|---|
| `reinhardt-core` / macros | flat `#[routes]` and `#[viewset]` reverse accessors such as `urls.article_detail(id)` and `urls.article_list()` | namespaced accessors such as `urls.server().blog().article_detail(id)` |
| `reinhardt-core` / `reinhardt-urls` | `UrlResolverUnprefixed` | remove the bound/import; use namespaced reverse accessors |
| `reinhardt-urls` | `reverse_single_pass` | `try_reverse_single_pass` |
| `reinhardt-urls` | `reverse_with_aho_corasick` | `try_reverse_with_aho_corasick` |
| `reinhardt-urls` | `ClientRouter::route_pathN` / `named_route_pathN` | `route_path` / `named_route_path` |
| `reinhardt-query` | `SeaRc<T>` | `SharedRc<T>` |
| `reinhardt-di` | `Injected<T>` | `Depends<T>` |
| `reinhardt-di` | `OptionalInjected<T>` | `Option<Depends<T>>` |
| `reinhardt-conf` | `Settings`, `AdvancedSettings` | explicit composed settings structs built with `SettingsBuilder` |
| `reinhardt-conf` | `JsonFileSource`, `auto_source` | `TomlFileSource` |
| `reinhardt-conf` | mutable interpolation setters and related legacy source helpers | `TomlFileSource::with_interpolation()` / `without_interpolation()` |
| `reinhardt-db` | `DatabaseConnection::get_database_url_from_env_or_settings` | `DatabaseConnection::database_url_from(settings, env_override)` |
| `reinhardt-middleware` | `SessionStoreRef` | `Depends<SessionStore>` |
| `reinhardt-auth` | `DefaultUser` | application-owned `#[user]` model |
| `reinhardt-auth` | old `User` trait, `SimpleUser`, `AnonymousUser` | `AuthIdentity`, `BaseUser` / `FullUser`, `PermissionsMixin` |
| `reinhardt-auth` | old `CurrentUser` compatibility shape | final `CurrentUser<U>` tuple extractor |
| `reinhardt-rest` | `OpenApiConfig` | `OpenApiSettings` |
| `reinhardt-pages` | router relocation items such as `PathError`, `RouterError`, `ClientRouteMatch`, `ClientRoute`, `ClientRouter`, `NavigationSubscription`, `ClientPathPattern`, `Path` | `reinhardt_urls::routers` equivalents |
| `reinhardt-pages` | `watch { ... }` in `page!` bodies | inline the body; reactive wrapping is automatic |
| `reinhardt-test` / `reinhardt-pages` | `use_action_state` compatibility API | current action/form state APIs |
| `reinhardt-test` | `MockFetch`, `mock_server_fn` | `MockServiceWorker` |
| `reinhardt-test` | built-in `TestUser` fixture | test-local user type plus `ForceLoginUser` |
| `reinhardt-testkit` | `APIClient::force_authenticate` | `client.auth().session(...)` or `client.auth().jwt(...)` |
| `reinhardt-testkit` | `APIRequestFactory::force_authenticate` | fluent auth API |
| `reinhardt-testkit` | `ServerFnTestContext::with_authenticated_user` | `.auth().session(&user).done()` |
| `reinhardt-testkit` | old global-registry migration fixtures | `postgres_with_migrations_from_dir(...)` |
| `reinhardt-admin` | `reinhardt_admin::core::vendor` shim | `reinhardt_utils::staticfiles::vendor` |

Quick scan:

```bash
rg -n "UrlResolverUnprefixed|reverse_single_pass|reverse_with_aho_corasick|route_path[0-9]|named_route_path[0-9]" src crates examples
rg -n "SeaRc|Injected|OptionalInjected|AdvancedSettings|JsonFileSource|auto_source|OpenApiConfig" src crates examples
rg -n "SessionStoreRef|DefaultUser|SimpleUser|AnonymousUser|get_database_url_from_env_or_settings|MockFetch|force_authenticate|with_authenticated_user" src crates examples
rg -n "watch \\{|use_action_state|reinhardt_pages::router::(Path|ClientRouter|ClientRoute|ClientRouteMatch|PathError|RouterError)" src crates examples
```

## Auth extractor contract

The final 0.2 line uses `CurrentUser<U>` as the canonical authenticated-user
extractor. `AuthUser<U>` remains as a deprecated tuple-struct compatibility
wrapper during the 0.2 cycle and is scheduled for removal in 0.3.

```rust
// Before
use reinhardt_auth::AuthUser;

async fn profile(AuthUser(user): AuthUser<MyUser>) -> Response {
    let id = user.id();
    // ...
}

// After
use reinhardt_auth::CurrentUser;

async fn profile(CurrentUser(user): CurrentUser<MyUser>) -> Response {
    let id = user.id();
    // ...
}
```

If code used the old 0.1.x `CurrentUser` shape directly, recheck
destructuring. The 0.2 extractor is a tuple struct with the same pattern shape
as `AuthUser<U>`.

## ORM and query changes

### Filter API

`Manager::filter`, `QuerySet::filter`, and `CustomManager::filter` now accept
a single value that converts into `Filter`.

```rust
// Before
let users = User::objects().filter("email", "contains", "example.com");

// After
let users = User::objects().filter(User::fields().email.contains("example.com"));
```

### Custom managers

Custom managers unify under `Model::objects()` through
`#[model(manager = MyManager)]`. Remove separate `HasCustomManager` call paths
and use the model's normal manager entry point.

### Model construction

`Model::new()` is a zero-argument alias for the typestate builder. If code
passed field values to `new`, move to builder setters or direct struct
construction according to the generated API.

```rust
// Before
let user = User::new(username, email);

// After
let user = User::build()
    .username(username)
    .email(email)
    .build();
```

The `#[model]` macro also generates a `{Model}Info` companion DTO with
bidirectional `From` conversions. Prefer that DTO over hand-maintained mirror
structs when moving model data across API boundaries.

### Reverse SQL

`Operation::to_reverse_sql` returns `Vec<String>` because some backends require
multiple rollback statements.

```rust
// Before
let rollback_sql: String = operation.to_reverse_sql(&builder)?;

// After
let rollback_sql: Vec<String> = operation.to_reverse_sql(&builder)?;
```

## Pages and forms

These are 0.2 behavior changes, not the removed-deprecated list above:

- closure-taking hooks such as `use_effect`, `use_layout_effect`, `use_memo`,
  and `use_callback` take an explicit dependency tuple; pass `()` for
  mount-only behavior,
- hook closures do not auto-subscribe from `Signal::get`; listed deps drive
  subscription,
- `page!` auto-wraps expressions and control-flow blocks reactively,
- `page!` body identifiers must be declared as closure parameters or qualified
  as paths such as `self::helper`,
- bare identifier shorthand in element bodies is removed; write `{name}`,
- `form!` fields can carry typed generic parameters for server function values.

Use the project codemod for mechanical page-body rewrites when available:

```bash
cargo make migrate-manouche-v2
```

## Settings fragments

0.2 adds settings fragments and deprecates ad-hoc config structs across
feature crates. For touched code, prefer the settings-first API.

| Old shape | 0.2 shape |
|---|---|
| `TemplateConfig` | `TemplateSettings` |
| `RateLimitConfig` | `RateLimitSettings` |
| `GrpcServerConfig` | `GrpcServerSettings` |
| `DeeplinkConfig` | `DeeplinkSettings` |
| `CorsConfig` | `CorsSettings` |
| `SmtpConfig` | `EmailSettings` |
| websocket `XxxConfig` structs | websocket settings fragments |
| task queue config structs | task settings fragments |
| storage config structs | `StorageSettings` and provider settings |

Example composed settings:

```rust
use reinhardt::settings;
use reinhardt_conf::{CoreSettings, TemplateSettings};
use reinhardt_conf::settings::builder::SettingsBuilder;
use reinhardt_conf::settings::openapi::OpenApiSettings;
use reinhardt_conf::settings::sources::TomlFileSource;

#[settings]
pub struct ProjectSettings {
    pub core: CoreSettings,
    pub templates: TemplateSettings,
    pub openapi: OpenApiSettings,
}

let settings = SettingsBuilder::new()
    .add_source(TomlFileSource::new("settings/development.toml"))
    .build_composed::<ProjectSettings>()?;
```

Database URL resolution should use the same settings value:

```rust
let url = DatabaseConnection::database_url_from(&settings, None)?;
```

Management commands should read database configuration through
`CommandContext::settings`, not by reloading settings files.

## Database migrations

After the code compiles, regenerate migrations once and inspect the diff.
0.2 may emit constraint and field-nullability changes that were not visible in
0.1.x metadata.

```bash
cargo run --bin manage -- makemigrations
cargo run --bin manage -- migrate
git diff -- migrations
```

Do not hand-edit migrations to hide metadata drift. If the model definition is
correct, let `makemigrations` produce the migration and review the generated
operations.

## Facade feature flags

0.2 adds root facade feature flags for surfaces that previously required
depending on implementation crates directly:

- `auth-social`
- `commands-server`

If a downstream crate uses `default-features = false`, audit its feature list
after the version bump. Missing feature flags often appear as unresolved facade
imports.

## Verification

Run focused scans until they are empty or only hit deliberate compatibility
uses:

```bash
rg -n "UrlResolverUnprefixed|reverse_single_pass|reverse_with_aho_corasick|route_path[0-9]|named_route_path[0-9]" src crates examples
rg -n "SeaRc|Injected|OptionalInjected|AdvancedSettings|JsonFileSource|auto_source|OpenApiConfig" src crates examples
rg -n "SessionStoreRef|DefaultUser|SimpleUser|AnonymousUser|get_database_url_from_env_or_settings|MockFetch|force_authenticate|with_authenticated_user" src crates examples
rg -n "watch \\{|use_action_state|reinhardt_pages::router::(Path|ClientRouter|ClientRoute|ClientRouteMatch|PathError|RouterError)" src crates examples
rg -n "Operation::to_reverse_sql|HasCustomManager|\\.filter\\([^)]*,[^)]*,|AuthUser" src crates examples
```

Then run normal checks:

```bash
cargo check --workspace --all --all-features
cargo test --workspace --all --all-features
cargo test --doc
cargo make fmt-check
cargo make clippy-check
```

For applications with WASM pages, also run the project's WASM build/test
command. Native `cargo check` does not prove client-side macro expansion or
MSW transport behavior.
