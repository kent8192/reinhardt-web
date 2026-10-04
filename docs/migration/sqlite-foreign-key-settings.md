# Typed SQLite foreign-key settings

`Query::sqlite_foreign_keys(enabled)` represents the connection-local SQLite
foreign-key setting. `SqliteForeignKeysStatement::build_sqlite_checked` renders
the SQLite setting. Its checked PostgreSQL, MySQL, and CockroachDB methods return
`QueryBuildError::UnsupportedBackendFeature` before execution. Unchecked
rendering with arbitrary builders is unavailable.
ON/OFF are SQL keywords, so this operation produces no bound parameters.
SQL generation has P2 native/WASM parity and performs no database I/O.

SQLite applies this setting only outside a transaction or savepoint. Execute
it on the same connection used for table reconstruction. The schema editor
retains its existing dedicated connection and RAII cleanup: rollback precedes
restoration, and a connection whose cleanup fails is closed instead of returned
to the pool. See [SQLite's setting documentation](https://sqlite.org/pragma.html#pragma_foreign_keys).

## Setting-site inventory

The setting capability gap tracked by [#6499](https://github.com/kent8192/reinhardt-web/issues/6499)
and the runtime SQL migration in [#5895](https://github.com/kent8192/reinhardt-web/issues/5895)
cover these sites. Each now uses the checked typed setting; no foreign-key
setting workaround remains at these sites.

| Site | Purpose | Representation |
| --- | --- | --- |
| `SchemaEditor::disable_foreign_keys` | Suspend enforcement before reconstruction | Typed OFF |
| `SchemaEditor::enable_foreign_keys` | Restore enabled enforcement after reconstruction | Typed ON |
| `SqliteRecreationSession::drop`, previous state ON | Restore after cancellation or abandoned work | Typed ON |
| `SqliteRecreationSession::drop`, previous state OFF | Preserve disabled enforcement after cancellation | Typed OFF |
| `MigrationSqlPlan::render`, before reconstruction | Render the existing SQL script wrapper | Typed OFF |
| `MigrationSqlPlan::render`, after reconstruction | Render the existing SQL script wrapper | Typed ON |

The SQL script retains its existing OFF/ON policy. Runtime reconstruction
instead restores the exact prior setting, whether enabled or disabled.
Read-only PRAGMA inspection and foreign-key integrity checks are separate
capabilities and are outside this setting inventory.

## Regression coverage

- Exact ON/OFF SQL and empty parameter lists.
- Rejection of all three other built-in backends, and a compile-fail example
  proving that generic unchecked rendering is unavailable.
- Atomic and non-atomic table reconstruction with the prior setting ON/OFF,
  both on success and on an error after reconstruction; original data and
  rollback state are checked independently.
- Atomic and non-atomic cancellation with the prior setting ON/OFF; connection
  identity is verified through retained in-memory tables and rows.
- Existing panic, cleanup, and SQL planning regressions remain applicable.
