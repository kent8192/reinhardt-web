# reinhardt-query-sqlx

The PostgreSQL dense-vector codec uses a private SQLx-version-local binary
encoder as a [tracked workaround](https://github.com/kent8192/reinhardt-web/issues/6526).
pgvector 0.4.2's broad SQLx range can select incompatible traits in fresh consumers.
This encoder checks the dense-vector dimension and finite elements before binding,
including native nullable vector parameters. Replace it with the upstream codec
once its feature/version selection or a coordinated SQLx upgrade guarantees
compatible traits independently of the workspace lockfile.

Owned SQLx arguments for the exact `(String, Values)` returned by
`reinhardt-query`. This crate does not execute queries, select pools, own
transactions, decode rows, rewrite placeholders or infer binds from columns.
Consumers execute the pair using SQLx's `query_with`/`query_as_with` functions.

The adapter is native-only (API parity P0); its symbols and native dependencies
are absent on browser WASM. `reinhardt-query` remains executor-independent and
portable. Default features are empty. Applications retain the existing
`reinhardt-db`/facade backend selections; consuming crates forward those selections
to adapter features. Standalone conf's `dynamic-database` enables its existing
Any path without introducing a dependency on db.

## Encoding contract

Values are consumed in renderer order. Every supplied Value creates one argument,
including a typed NULL if supplied. Renderers normally emit literal NULL and omit
it from Values; the adapter never reconstructs that omitted argument. Errors
identify the backend, one-based argument position, type and redacted reason.
PostgreSQL arrays accept the canonical `Value::Int(None)` NULL carrier using
the declared array element type, alongside matching typed NULL elements.
Unsupported types, mismatched non-NULL array elements and integer overflow fail before
execution; no debug-string fallback or floating-point decimal coercion exists.

| Value | PostgreSQL | MySQL | SQLite | SQLx Any |
|---|---|---|---|---|
| Signed integers, bool, floats, text, bytes | Native | Native | Native | Native |
| Unsigned integers | Lossless widening; u64 checked against i64 | Native unsigned | u64 checked against i64 | Lossless widening; u64 checked against i64 |
| Character | Single-character text | Single-character text | Single-character text | Single-character text |
| Chrono, UUID, JSON | Native with matching feature | Native; Local/fixed-offset datetime rejected | Native with matching feature | Explicit text compatibility function only |
| Decimal/BigDecimal | Native with matching feature | Native with matching feature | Rejected | Explicit text compatibility function only |
| Typed array | Native nullable elements, declared type validated | Rejected | Rejected | Rejected |
| pgvector | Native vector with pgvector feature | Rejected | Rejected | Rejected |

`prepare_any` permits native Any scalar codecs only.
`prepare_any_with_text_codecs` deliberately opts into schemas using UUID, JSON,
decimal and chrono text. UUID uses canonical Display, JSON uses JSON serialization,
decimals retain their decimal Display precision, dates/times/naive datetimes use
chrono Display and timezone datetimes use RFC 3339. PostgreSQL native columns
may require explicit typed casts in the caller's AST. The adapter leaves SQL
unchanged; it does not assume a text argument can target every native column.
Array/vector compatibility remains the responsibility of an explicitly typed
caller rather than silently serializing an unsupported argument here.

Optional `with-*` features enable both the query Value variant and its SQLx codec.
If another workspace consumer enables a query variant without enabling the
adapter's matching codec, adaptation returns a redacted error instead of breaking
feature-unified compilation.

The native `prepare_mysql_with_text_uuid` and
`prepare_sqlite_with_text_uuid` functions explicitly preserve existing UUID text
columns used by db's raw QueryValue codecs. They consume the original UUID Value
as canonical hyphenated text; an explicitly supplied UUID NULL remains a NULL
argument. Other values retain native encoding. The ordinary prepare functions
continue to use SQLx's native UUID codec, including SQLite's binary UUID format.
No schema or SQL text is inferred or rewritten.
