# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.5](https://github.com/kent8192/reinhardt-web/compare/reinhardt-db@v0.2.0-rc.4...reinhardt-db@v0.2.0-rc.5) - 2026-06-10

### Added

- *(orm)* add Django-like lookup helpers
- *(orm)* support composite filter combinators

### Fixed

- *(orm)* address lookup review edge cases

## [0.2.0-rc.3](https://github.com/kent8192/reinhardt-web/compare/reinhardt-db@v0.2.0-rc.2...reinhardt-db@v0.2.0-rc.3) - 2026-06-05

### Fixed

- address CodeRabbit dependency gate review

### Performance

- atomize facade dependency feature gates
- trim standard facade feature dependencies

## [0.2.0-rc.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-db@v0.1.3...reinhardt-db@v0.2.0-rc.2) - 2026-06-03

### Added

- *(db)* introduce type-safe nullable field on FieldMetadata
- *(db,macros)* [**breaking**] unify custom managers with Model::objects() ([[#3984](https://github.com/kent8192/reinhardt-web/issues/3984)](https://github.com/kent8192/reinhardt-web/issues/3984))
- *(model)* [**breaking**] make new an alias for build

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Documentation

- *(reinhardt-db)* fix QuerySet doctests for single-argument filter() API
- *(reinhardt-db)* qualify Filter path in with_db doctests

### Fixed

- *(db)* address CodeRabbit review on CHANGELOG and with_param normalization
- *(ci)* recover develop release-plz prerelease
- *(db)* qualify Manager in rustdoc examples and add missing Objects type
- *(docs)* resolve remaining cross-crate intra-doc link errors
- repair release examples tests

### Maintenance

- *(examples)* remove examples-twitter

### Styling

- apply formatter fixes across workspace
- apply rustfmt to non-DSL files on develop/0.2.0

### Breaking Changes

- **`FieldMetadata` gains type-safe `nullable: bool` field** ([#4439](https://github.com/kent8192/reinhardt-web/issues/4439)).
  `is_nullable()` reads the struct field. `with_nullable()` sets it as
  the canonical source of truth. `with_param("null", ...)` still works
  (auto-syncs the struct field) but should migrate to `with_nullable()`.
  `to_model_state()` no longer copies `"null"` into `FieldState.params`.

### Removed

#### BREAKING CHANGES

- **`DatabaseConnection::get_database_url_from_env_or_settings(base_dir)`**
  (deprecated since `0.1.0-rc.29`) — removed per STABILITY_POLICY § SP-4
  and umbrella Issue [#4520](https://github.com/kent8192/reinhardt-web/issues/4520).
  The function reloaded `settings/<profile>.toml` from disk on every
  call, duplicating the framework's settings-loading logic. Use
  `DatabaseConnection::database_url_from(settings, env_override)` with
  a pre-built `ProjectSettings` instead.

In-tree test deleted: `crates/reinhardt-db/tests/database_url_loader_interpolation.rs`.

Note: this PR keeps the consumer in `reinhardt-commands/src/builtin.rs`
(`get_database_url_from_settings`) unchanged. That helper still
references the removed entry point and will be migrated in a
follow-up `chore(commands)!: replace get_database_url_from_settings with
database_url_from` PR.

See [`instructions/MIGRATION_0.2.md`](../../instructions/MIGRATION_0.2.md#reinhardt-db)
for the migration guide.

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-db@v0.1.0-rc.30...reinhardt-db@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-db` as part of the reinhardt-web
0.1.0 release. `reinhardt-db` is the Django-style database layer for
the framework, providing an async ORM, schema migrations, a connection
pool, and a unified backend abstraction over PostgreSQL, MySQL,
SQLite, and CockroachDB.

For the workspace-wide release narrative (Highlights, Breaking
Changes, Migration Guide), see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is preserved in the
[Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **Multi-backend support** — First-class support for PostgreSQL,
  MySQL, SQLite, and CockroachDB through a single async `sqlx`-backed
  connection abstraction. Backend-specific quirks (CockroachDB
  sentinel-row locks, SQLite `INTEGER PRIMARY KEY AUTOINCREMENT`,
  MySQL `MODIFY COLUMN`) are handled internally so application code
  stays portable.
- **Type-safe async ORM** — `#[model]`-driven model definitions with
  `QuerySet` filtering, `Manager` / `CustomManager` traits for custom
  query entry points, eager-loaded relations, and full Many-to-Many
  accessors with deterministic through-table naming. Built on top of
  `reinhardt-query` so no raw SQL strings appear in application code.
- **Django-style migrations** — `makemigrations` autodetector that
  diffs model state against the live database schema and emits typed
  migration operations including `CreateCompositePrimaryKey`,
  `SetAutoIncrementValue`, constraint diffs, and per-backend
  `ALTER COLUMN TYPE` dispatch. Migration conflict detection generates
  merge migration names automatically.
- **Connection pooling** — Pluggable pool implementation with
  `ManuallyDrop`-protected `PooledConnection` semantics, poison-safe
  lock recovery, and a `FilesystemSource` migration loader that warns
  rather than panics on missing migration directories.
- **NoSQL ODM scaffolding** — Optional `nosql` features expose a
  `Document` trait, `Repository<T>` for type-safe CRUD, and a builder-
  pattern `IndexModel`. Backends are gated behind individual features
  (`mongodb`, `redis`, `cassandra`, `dynamodb`, `neo4j`) so the
  default build remains SQL-only.
- **Configuration from `ProjectSettings`** — `database_url_from`
  accepts any type implementing `HasCoreSettings`, so connection
  strings are resolved through the typed-TOML settings pipeline
  instead of bare `env::var` calls. `DATABASE_URL` is opted into
  TOML interpolation explicitly.
- **SQL-injection-safe identifiers** — All identifier emission routes
  through `pg_escape`-style quoting (centralised in `crate::naming`),
  `BatchInsertBuilder` and `ON CONFLICT` quote column names, and
  `BatchUpdateBuilder`'s previously-injection-vulnerable methods were
  removed before stable.
- **UUID v7 by default** — Generated UUIDs use v7 across the crate
  for time-ordered primary keys.

### Notable Breaking Changes

- **`ProjectSettings` replaces `env::var`** ([#4295](https://github.com/kent8192/reinhardt-web/discussions/4295))
  — `commands` and `db` no longer read `DATABASE_URL` via `std::env`;
  pass a `&dyn HasCoreSettings` (or call `database_url_from`) so
  typed TOML interpolation governs the value.
- **`unique_together` propagates into `ModelMetadata`** ([#4027](https://github.com/kent8192/reinhardt-web/discussions/4027))
  — the autodetector now consumes `unique_together`, so existing
  migrations regenerate. Diff once, commit the regeneration.
- **SQL-injection-vulnerable `BatchUpdateBuilder` methods removed**
  — APIs that interpolated identifiers without quoting are gone.
  Switch to the parameterised builder methods.

### Migration Notes

- **DATABASE_URL via settings**: Replace
  `std::env::var("DATABASE_URL")` call sites with
  `reinhardt_db::database_url_from(settings)`; declare a
  `database_url` field on your settings struct (or rely on the
  default TOML key) and remove the explicit `env::var` import.
- **Regenerate migrations**: After upgrading, run `makemigrations`
  once on each app — `unique_together` and constraint diffs will
  appear as new migration files. Apply with `migrate`.
- **Audit raw-SQL call sites**: If you constructed `BatchUpdateBuilder`
  arguments by hand, replace removed methods with the parameterised
  builder; refer to the
  [reinhardt-query docs](https://docs.rs/reinhardt-query) for the
  current safe API.
