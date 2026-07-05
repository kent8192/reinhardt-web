# Typed Generated Column Expressions

**Issue:** [#5523](https://github.com/kent8192/reinhardt-web/issues/5523)
**Date:** 2026-07-01
**Status:** Design approved

## Context

Generated columns are currently exposed at the model layer as raw SQL strings:

```rust
#[field(
    max_length = 201,
    generated = "first_name || ' ' || last_name",
    generated_stored = true,
)]
pub full_name: String,
```

That API embeds backend-specific SQL into model attributes and bypasses the
identifier quoting and expression rendering already owned by `reinhardt-query`.
The lower-level `ColumnDef` type has typed expression support for `default` and
`check`, but it has no generated-column metadata. The main migration path also
uses `ColumnDefinition` and `FieldState.params`, which currently do not carry a
structured generated-column expression.

Issue #5523 moves generated columns into `reinhardt-query` DDL and makes typed
generated expressions the default model API. Backward compatibility for
`generated = "raw SQL"` is intentionally not preserved. Raw SQL remains
available only through an explicit `generated_sql = "..."` escape hatch.

## Goals

- Add a typed generated-column API to `reinhardt-query::types::ddl::ColumnDef`.
- Keep generated-column DDL rendering backend-aware for PostgreSQL,
  CockroachDB, MySQL, and SQLite.
- Replace model-level `generated = "..."` with `generated = SchemaExpr::...`.
- Preserve raw generated SQL only behind explicit `generated_sql = "..."`.
- Freeze typed generated expressions into generated migration Rust tokens, not
  backend-specific SQL strings.
- Detect generated-expression and storage-mode changes in migration diffs.

## Non-Goals

- General-purpose functional-dependency declarations.
- Support for arbitrary `SimpleExpr` in generated columns.
- Support for user-defined helper functions inside `#[field(generated = ...)]`.
- Version-aware PostgreSQL virtual generated columns in the first
  implementation.
- Full SQL expression coverage. The initial typed subset should be small and
  expanded deliberately.

## Design Decisions

| Area | Decision | Rationale |
|---|---|---|
| Expression type | Add `SchemaExpr` instead of reusing `SimpleExpr` | `SimpleExpr` can represent subqueries, raw SQL, aliases, and window functions that are not safe generated-column inputs. |
| Raw SQL | Use `generated_sql = "..."` only | Raw SQL remains possible but is visibly unsafe and backend-specific. |
| `generated = "..."` | Reject at compile time | The new API intentionally breaks the ambiguous string form. |
| Migration storage | Emit Rust DSL tokens | Migrations remain backend-neutral and render SQL at execution time. |
| Default API surface | `ColumnDef::generated_stored` and `ColumnDef::generated_virtual` | Matches the existing builder style and keeps storage explicit. |
| PostgreSQL virtual columns | Reject initially | PostgreSQL support depends on server version, and Reinhardt does not yet have version-aware DDL generation. |

## Query API

Add DDL-safe expression types in `crates/reinhardt-query/src/types/ddl.rs` or a
small sibling module re-exported from `types::ddl`:

```rust
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SchemaExpr {
    Column(DynIden),
    Value(SchemaValue),
    Unary {
        op: SchemaUnOp,
        expr: Box<SchemaExpr>,
    },
    Binary {
        left: Box<SchemaExpr>,
        op: SchemaBinOp,
        right: Box<SchemaExpr>,
    },
    Function {
        func: SchemaFunc,
        args: Vec<SchemaExpr>,
    },
    Cast {
        expr: Box<SchemaExpr>,
        ty: ColumnType,
    },
    Case(SchemaCase),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GeneratedStorage {
    Stored,
    Virtual,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedColumn {
    pub expr: Option<SchemaExpr>,
    pub raw_sql: Option<String>,
    pub storage: GeneratedStorage,
}
```

`GeneratedColumn` keeps typed and raw SQL bodies as separate fields so callers
and migration code can distinguish safe generated expressions from explicit raw
SQL. Validation requires exactly one of `expr` or `raw_sql`.

The initial `SchemaExpr` constructor surface should stay narrow:

```rust
impl SchemaExpr {
    pub fn col(name: impl IntoIden) -> Self;
    pub fn val(value: impl Into<SchemaValue>) -> Self;
    pub fn concat<I>(items: I) -> Self
    where
        I: IntoIterator<Item = SchemaExpr>;
    pub fn coalesce<I>(items: I) -> Self
    where
        I: IntoIterator<Item = SchemaExpr>;
    pub fn cast(self, ty: ColumnType) -> Self;
}
```

`SchemaExpr` must not expose raw SQL, subqueries, window expressions, aliases,
table-qualified mutable query context, or placeholder-based parameter binding.
The DDL renderer writes inline SQL literals because generated-column DDL has no
runtime bind parameters.

`ColumnDef` gains generated-column metadata:

```rust
impl ColumnDef {
    pub fn generated(mut self, expr: SchemaExpr, storage: GeneratedStorage) -> Self;
    pub fn generated_stored(self, expr: SchemaExpr) -> Self;
    pub fn generated_virtual(self, expr: SchemaExpr) -> Self;
    pub fn generated_sql(mut self, sql: impl Into<String>, storage: GeneratedStorage) -> Self;
}
```

`generated_sql` should keep the existing raw SQL validator in the model layer
and add a query-layer validation hook for direct `ColumnDef` callers.

## Backend Rendering

Create a shared column-rendering helper per backend so `CREATE TABLE`,
`ALTER TABLE ADD COLUMN`, and supported `ALTER TABLE MODIFY COLUMN` paths do not
drift.

Generated column order should be:

```text
<name> <type> GENERATED ALWAYS AS (<expr>) <storage> [NOT NULL] [UNIQUE] ...
```

Backend-specific rendering rules:

| Backend | Stored | Virtual | `SchemaExpr::concat` |
|---|---|---|---|
| PostgreSQL | `STORED` | Validation error initially | `a \|\| b \|\| c` |
| CockroachDB | `STORED` through PostgreSQL renderer | Validation error initially | `a \|\| b \|\| c` |
| MySQL | `STORED` | `VIRTUAL` | `CONCAT(a, b, c)` |
| SQLite | `STORED` | `VIRTUAL` | `a \|\| b \|\| c` |

SQLite requires extra validation for `ALTER TABLE ADD COLUMN`: adding stored
generated columns is not portable across SQLite versions and should be rejected
by the backend validator unless the implementation has a table-recreation path.

## Model Macro API

The model macro accepts only typed generated expressions through `generated`:

```rust
#[field(
    max_length = 201,
    generated = SchemaExpr::concat([
        SchemaExpr::col("first_name"),
        SchemaExpr::val(" "),
        SchemaExpr::col("last_name"),
    ]),
    generated_stored = true,
)]
pub full_name: String,
```

Raw SQL must use `generated_sql`:

```rust
#[field(
    max_length = 201,
    generated_sql = "first_name || ' ' || last_name",
    generated_stored = true,
)]
pub full_name: String,
```

`generated = "..."` is a compile error with guidance to use either typed
`SchemaExpr` or `generated_sql`.

The macro should parse `generated = ...` as a restricted expression grammar, not
as arbitrary Rust. Accepted forms are `SchemaExpr` constructor calls, arrays or
vectors of nested accepted expressions, and primitive literals accepted by
`SchemaExpr::val`. The macro canonicalizes accepted input into absolute
`SchemaExpr` constructor tokens for migration output. This avoids migrations
depending on user-defined helper functions, closures, local imports, or runtime
state.

Macro validation rejects:

- `generated` with `default`.
- `generated` with `generated_sql`.
- Missing storage mode for either typed or raw generated columns.
- Both `generated_stored = true` and `generated_virtual = true`.
- `generated = "..."`.
- Constructor calls outside the accepted `SchemaExpr` grammar.

## Migration And State Flow

Add structured generated metadata alongside existing string params:

- `FieldMetadata` gains `generated: Option<GeneratedFieldMetadata>`.
- `FieldState` gains `generated: Option<GeneratedFieldState>`.
- `ColumnDefinition` gains `generated: Option<GeneratedColumnDefinition>` with
  `#[serde(default)]` for old migrations.

`FieldState.params` remains for legacy scalar field attributes such as
`default`, `max_length`, and compatibility keys, but typed generated expressions
must not be serialized into that string map.

`GeneratedColumnDefinition` stores either canonical typed tokens plus an
evaluated `SchemaExpr`, or explicit raw SQL plus storage mode:

```rust
pub struct GeneratedColumnDefinition {
    pub expr: Option<SchemaExpr>,
    pub expr_tokens: Option<String>,
    pub raw_sql: Option<String>,
    pub storage: GeneratedStorage,
}
```

`expr_tokens` is the source used by `ToTokens` when writing migration files.
`expr` is the runtime AST used by migration execution and diffing. If adding
serde support directly to `SchemaExpr` would force a new core dependency in
`reinhardt-query`, implement serde at the `reinhardt-db` field boundary instead.

`ColumnDefinition::from_field_state` copies generated metadata into the
canonical column definition. Existing canonical diff logic then detects changes
to expression body or storage mode through `PartialEq`.

Generated migration output should look like:

```rust
ColumnDefinition {
    name: "full_name".to_string(),
    type_definition: FieldType::VarChar(201),
    not_null: true,
    unique: false,
    primary_key: false,
    auto_increment: false,
    default: None,
    generated: Some(GeneratedColumnDefinition::typed(
        SchemaExpr::concat([
            SchemaExpr::col("first_name"),
            SchemaExpr::val(" "),
            SchemaExpr::col("last_name"),
        ]),
        GeneratedStorage::Stored,
    )),
}
```

## Error Handling

The new generated-column API should prefer validation errors over panics. The
current query builders still use panics for some unsupported DDL operations, so
implementation should introduce explicit validation helpers that callers invoke
before rendering generated-column clauses.

Error messages should name the unsupported construct and backend:

- `PostgreSQL generated columns do not support Virtual storage in this API yet`
- `SQLite cannot add a STORED generated column with ALTER TABLE ADD COLUMN`
- `generated = "..." is no longer supported; use generated_sql = "..." for raw SQL`
- `generated expressions cannot include user-defined helper calls`

## Tests

Required coverage:

- `reinhardt-query` create-table generated DDL for PostgreSQL, CockroachDB,
  MySQL, and SQLite.
- `reinhardt-query` add-column generated DDL where the backend supports it.
- Backend validation failures for unsupported storage modes and SQLite stored
  add-column behavior.
- `SchemaExpr::concat` renders as `||` on PostgreSQL, CockroachDB, and SQLite,
  and as `CONCAT(...)` on MySQL.
- `SchemaExpr` typed API has no raw SQL, subquery, window, or alias constructor.
- `ColumnDefinition` token serialization includes generated metadata.
- `ColumnDefinition` serde default keeps old migration/state data readable.
- Autodetector emits an `AlterColumn` when generated expression or storage mode
  changes.
- `model_derive` accepts typed `generated = SchemaExpr::...`.
- `model_derive` accepts `generated_sql = "..."`.
- `model_derive` rejects `generated = "..."`.
- `model_derive` rejects typed/raw/default/storage conflicts.
- SQLite migration e2e covers a typed generated column in a fresh create-table
  migration.

## Documentation

Update:

- `README.md` field attribute list.
- `crates/reinhardt-db/README.md` model field attribute section.
- `crates/reinhardt-query/README.md` DDL examples.
- Relevant crate-level docs for `SchemaExpr`, `GeneratedStorage`, and
  `ColumnDef::generated_*`.

Documentation should present typed `SchemaExpr` as the normal API and
`generated_sql` as a backend-specific escape hatch.

## Stability And Release Notes

This is a public API addition plus an intentional breaking model macro change.
Because the current branch is in the 0.4 release-candidate line, the PR must
cite issue #5523 as the API-change record and explain why the old string form
blocks the typed generated-column feature. The PR body should reference the
RC stability process in `instructions/STABILITY_POLICY.md` and call out the
migration path from `generated = "..."` to either typed `SchemaExpr` or
`generated_sql = "..."`.

## References

- [PostgreSQL generated columns](https://www.postgresql.org/docs/current/ddl-generated-columns.html)
- [MySQL generated columns](https://dev.mysql.com/doc/en/create-table-generated-columns.html)
- [SQLite generated columns](https://sqlite.org/gencol.html)
- [CockroachDB computed columns](https://www.cockroachlabs.com/docs/stable/computed-columns)
