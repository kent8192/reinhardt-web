# reinhardt-query

A type-safe SQL query builder for the Reinhardt framework.

## Overview

`reinhardt-query` provides a fluent API for constructing SQL queries targeting PostgreSQL, MySQL, and SQLite. It generates parameterized queries with proper identifier escaping and value placeholders for each backend.

## Features

- **Type-safe query construction** - SELECT, INSERT, UPDATE, DELETE
- **DCL (Data Control Language) support** - GRANT and REVOKE statements
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

### GRANT (DCL)

```rust
use reinhardt_query::prelude::*;

let stmt = Query::grant()
    .privilege(Privilege::Select)
    .privilege(Privilege::Insert)
    .on_table("users")
    .to("app_user")
    .with_grant_option(true);

let builder = PostgresQueryBuilder::new();
let (sql, values) = builder.build_grant(&stmt);
// sql = r#"GRANT SELECT, INSERT ON TABLE "users" TO "app_user" WITH GRANT OPTION"#
```

### REVOKE (DCL)

```rust
use reinhardt_query::prelude::*;

let stmt = Query::revoke()
    .privilege(Privilege::Insert)
    .from_table("users")
    .from("app_user")
    .cascade(true);

let builder = PostgresQueryBuilder::new();
let (sql, values) = builder.build_revoke(&stmt);
// sql = r#"REVOKE INSERT ON TABLE "users" FROM "app_user" CASCADE"#
```

### GRANT Role Membership (DCL)

```rust
use reinhardt_query::dcl::{GrantRoleStatement, RoleSpecification};

let stmt = GrantRoleStatement::new()
    .role("developer")
    .to(RoleSpecification::new("alice"))
    .with_admin_option();

let builder = PostgresQueryBuilder::new();
let (sql, values) = builder.build_grant_role(&stmt);
// sql = r#"GRANT "developer" TO alice WITH ADMIN OPTION"#
```

### REVOKE Role Membership (DCL)

```rust
use reinhardt_query::dcl::{RevokeRoleStatement, RoleSpecification};

let stmt = RevokeRoleStatement::new()
    .role("developer")
    .from(RoleSpecification::new("alice"))
    .cascade();

let builder = PostgresQueryBuilder::new();
let (sql, values) = builder.build_revoke_role(&stmt);
// sql = r#"REVOKE "developer" FROM alice CASCADE"#
```

### Extended Object Types (PostgreSQL)

PostgreSQL supports additional object types beyond tables and databases:

```rust
use reinhardt_query::prelude::*;

// Grant EXECUTE on function
let stmt = Query::grant()
    .privilege(Privilege::Execute)
    .on_function("calculate_total")
    .to("app_user");

let builder = PostgresQueryBuilder::new();
let (sql, values) = builder.build_grant(&stmt);
// sql = r#"GRANT EXECUTE ON FUNCTION "calculate_total" TO "app_user""#

// Grant USAGE on type
let stmt = Query::grant()
    .privilege(Privilege::Usage)
    .on_type("custom_type")
    .to("app_user");

let (sql, values) = builder.build_grant(&stmt);
// sql = r#"GRANT USAGE ON TYPE "custom_type" TO "app_user""#

// Grant SET on parameter
let stmt = Query::grant()
    .privilege(Privilege::Set)
    .on_parameter("work_mem")
    .to("app_user");

let (sql, values) = builder.build_grant(&stmt);
// sql = r#"GRANT SET ON PARAMETER "work_mem" TO "app_user""#
```

Supported object types: Function, Procedure, Routine, Type, Domain, ForeignDataWrapper, ForeignServer, Language, LargeObject, Tablespace, Parameter

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
| DCL (GRANT/REVOKE) | Full support | Full support | Not supported (panics) |

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
