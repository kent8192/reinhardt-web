# reinhardt-query

A type-safe SQL query builder for the Reinhardt framework.

## Overview

`reinhardt-query` provides a fluent API for constructing SQL queries targeting PostgreSQL, MySQL, and SQLite. It generates parameterized queries with proper identifier escaping and value placeholders for each backend.

## Features

- **Type-safe query construction** - SELECT, INSERT, UPDATE, DELETE
- **DDL operations** - CREATE TABLE, ALTER TABLE, DROP TABLE, CREATE INDEX, DROP INDEX
- **Multi-backend support** - PostgreSQL, MySQL, SQLite with proper dialect handling
- **Expression system** - Arithmetic, comparison, logical, and pattern matching operators
- **Advanced SQL** - JOINs, GROUP BY, HAVING, DISTINCT, UNION, CTEs, Window functions
- **Parameterized queries** - `$1` for PostgreSQL, `?` for MySQL/SQLite
- **CASE WHEN expressions** - Conditional expressions in queries
- **Subqueries** - EXISTS, IN, ALL, ANY, SOME operators

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
reinhardt-query = { version = "0.1.0-alpha.1" }
```

## Quick Start

```rust
use reinhardt_query::prelude::*;

// Build a SELECT query
let mut stmt = Query::select();
stmt.column("name")
    .column("email")
    .from("users")
    .and_where(Expr::col("active").eq(true))
    .order_by("name", Order::Asc)
    .limit(10);

// Generate SQL for PostgreSQL
let builder = PostgresQueryBuilder::new();
let (sql, values) = builder.build_select(&stmt);
// sql = r#"SELECT "name", "email" FROM "users" WHERE "active" = $1 ORDER BY "name" ASC LIMIT $2"#
```

## Usage Examples

### INSERT

```rust
use reinhardt_query::prelude::*;

let mut stmt = Query::insert();
stmt.into_table("users")
    .columns(["name", "email", "active"])
    .values(["Alice", "alice@example.com", "true"]);

let builder = PostgresQueryBuilder::new();
let (sql, values) = builder.build_insert(&stmt);
```

### UPDATE

```rust
use reinhardt_query::prelude::*;

let mut stmt = Query::update();
stmt.table("users")
    .set("name", "Bob")
    .set("active", false)
    .and_where(Expr::col("id").eq(1i32));

let builder = PostgresQueryBuilder::new();
let (sql, values) = builder.build_update(&stmt);
```

### DELETE

```rust
use reinhardt_query::prelude::*;

let mut stmt = Query::delete();
stmt.from_table("users")
    .and_where(Expr::col("active").eq(false));

let builder = PostgresQueryBuilder::new();
let (sql, values) = builder.build_delete(&stmt);
```

### CREATE TABLE

```rust
use reinhardt_query::prelude::*;
use reinhardt_query::types::{ColumnDef, ColumnType};

let mut stmt = Query::create_table();
stmt.table("users")
    .if_not_exists()
    .column(ColumnDef {
        name: "id".into_iden(),
        column_type: ColumnType::Integer,
        not_null: true,
        primary_key: true,
        auto_increment: true,
        ..Default::default()
    })
    .column(ColumnDef {
        name: "email".into_iden(),
        column_type: ColumnType::String(Some(255)),
        not_null: true,
        unique: true,
        ..Default::default()
    });

let builder = PostgresQueryBuilder::new();
let (sql, values) = builder.build_create_table(&stmt);
// sql = r#"CREATE TABLE IF NOT EXISTS "users" ("id" INTEGER NOT NULL PRIMARY KEY, "email" VARCHAR(255) NOT NULL UNIQUE)"#
```

### ALTER TABLE

```rust
use reinhardt_query::prelude::*;
use reinhardt_query::query::AlterTableOperation;
use reinhardt_query::types::{ColumnDef, ColumnType};

let mut stmt = Query::alter_table();
stmt.table("users")
    .add_column(ColumnDef {
        name: "created_at".into_iden(),
        column_type: ColumnType::Timestamp,
        not_null: true,
        ..Default::default()
    })
    .rename_column("email", "email_address");

let builder = PostgresQueryBuilder::new();
let (sql, values) = builder.build_alter_table(&stmt);
// sql = r#"ALTER TABLE "users" ADD COLUMN "created_at" TIMESTAMP NOT NULL, RENAME COLUMN "email" TO "email_address""#
```

### DROP TABLE

```rust
use reinhardt_query::prelude::*;

let mut stmt = Query::drop_table();
stmt.table("users")
    .if_exists()
    .cascade();  // PostgreSQL only

let builder = PostgresQueryBuilder::new();
let (sql, values) = builder.build_drop_table(&stmt);
// sql = "DROP TABLE IF EXISTS \"users\" CASCADE"
```

### CREATE INDEX

```rust
use reinhardt_query::prelude::*;

let mut stmt = Query::create_index();
stmt.name("idx_email")
    .table("users")
    .col("email")
    .unique()
    .if_not_exists();

let builder = PostgresQueryBuilder::new();
let (sql, values) = builder.build_create_index(&stmt);
// sql = r#"CREATE UNIQUE INDEX IF NOT EXISTS "idx_email" ON "users" ("email")"#
```

### DROP INDEX

```rust
use reinhardt_query::prelude::*;

let mut stmt = Query::drop_index();
stmt.name("idx_email")
    .if_exists();

let builder = PostgresQueryBuilder::new();
let (sql, values) = builder.build_drop_index(&stmt);
// sql = "DROP INDEX IF EXISTS \"idx_email\""
```

### JOINs

```rust
use reinhardt_query::prelude::*;
use reinhardt_query::types::{JoinType, JoinExpr, JoinOn};

let mut stmt = Query::select();
stmt.column("users.name")
    .column("orders.total")
    .from("users")
    .join(JoinExpr {
        join_type: JoinType::InnerJoin,
        table: "orders".into_table_ref(),
        on: Some(JoinOn::Condition(
            Expr::col("users.id").eq(Expr::col("orders.user_id")).into_condition()
        )),
    });
```

### Window Functions

```rust
use reinhardt_query::prelude::*;
use reinhardt_query::types::WindowStatement;

let mut stmt = Query::select();
stmt.column("name")
    .expr(Expr::col("salary").over(WindowStatement {
        partition_by: vec![Expr::col("department").into_simple_expr()],
        order_by: vec![],
        frame: None,
    }))
    .from("employees");
```

### Common Table Expressions (CTEs)

```rust
use reinhardt_query::prelude::*;
use reinhardt_query::query::CommonTableExpr;

let mut cte_query = Query::select();
cte_query.column("id").column("name").from("categories")
    .and_where(Expr::col("parent_id").is_null());

let mut stmt = Query::select();
stmt.column("name")
    .from("top_categories")
    .with_cte(CommonTableExpr {
        name: "top_categories".into_iden(),
        columns: vec![],
        query: cte_query,
    });
```

## Backend Differences

| Feature | PostgreSQL | MySQL | SQLite |
|---------|-----------|-------|--------|
| Identifier quoting | `"name"` | `` `name` `` | `"name"` |
| Placeholders | `$1, $2, ...` | `?, ?, ...` | `?, ?, ...` |
| NULLS FIRST/LAST | Native | Not supported | Native |
| DISTINCT ON | Supported | Not supported | Not supported |
| Window functions | Full | Full | Full |
| CTEs (WITH) | Supported | Supported | Supported |
| Recursive CTEs | Supported | Supported | Supported |
| `||` concatenation | Native | Not supported | Native |
| DROP TABLE CASCADE/RESTRICT | Supported | Not supported | Not supported |
| DROP INDEX CASCADE/RESTRICT | Supported | Not supported | Not supported |
| DROP INDEX requires table | No | Yes (ON clause) | No |

## Feature Flags

| Flag | Description |
|------|-------------|
| `thread-safe` | Use `Arc` instead of `Rc` for `DynIden` |
| `with-chrono` | Enable chrono date/time types in `Value` |
| `with-uuid` | Enable UUID type in `Value` |
| `with-json` | Enable JSON type in `Value` |
| `with-rust_decimal` | Enable Decimal type in `Value` |
| `with-bigdecimal` | Enable BigDecimal type in `Value` |
| `full` | Enable all optional features |

## License

See the repository root for license information.
