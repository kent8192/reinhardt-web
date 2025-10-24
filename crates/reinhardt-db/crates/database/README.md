# reinhardt-database

Database abstraction layer for Reinhardt framework with unified interfaces for SQL databases.

## Overview

`reinhardt-database` provides a unified database abstraction layer supporting multiple SQL databases through trait inheritance and dialect-specific implementations. This crate is designed for low-level database operations, working alongside `reinhardt-orm` for higher-level ORM functionality.

## Features Status

### Core Abstraction Layer ✓

#### Database Backend Trait (Implemented ✓)

- `DatabaseBackend` trait for unified database interface
- Async database operations (execute, fetch_one, fetch_all, fetch_optional)
- Database capability detection (RETURNING clause, ON CONFLICT support)
- Placeholder generation for parameterized queries
- Connection pooling support via sqlx

#### Query Builders (Implemented ✓)

- **SelectBuilder**: SELECT queries with WHERE, LIMIT clauses
- **InsertBuilder**: INSERT with optional RETURNING support
- **UpdateBuilder**: UPDATE with SET and WHERE clauses, NOW() function support
- **DeleteBuilder**: DELETE with WHERE conditions and IN operator support
- Type-safe parameter binding via `QueryValue` enum

#### Type System (Implemented ✓)

- `QueryValue` enum: Null, Bool, Int, Float, String, Bytes, Timestamp
- `Row` type for query results with type-safe column access
- `QueryResult` for tracking rows affected
- Automatic type conversions with error handling
- `DatabaseType` enum: Postgres, Sqlite, Mysql

#### Schema Editor System (Implemented ✓)

- `BaseDatabaseSchemaEditor` trait for DDL operations
- CREATE/DROP TABLE support
- ALTER TABLE operations (add/drop/rename columns, constraints)
- CREATE/DROP INDEX with unique and partial index support
- DDL statement types and references
- Factory pattern for database-specific editors

### SQL Database Support

#### PostgreSQL (Implemented ✓)

- **Connection Management**: Connection pooling via sqlx PgPool
- **Query Execution**: Full async query support with parameter binding
- **Type Mapping**: Comprehensive type conversion (primitives, timestamps, bytes, NULL)
- **Database Features**:
  - RETURNING clause support ✓
  - ON CONFLICT clause support ✓
  - Parameterized queries ($1, $2, ...) ✓
- **Schema Editor**:
  - Standard DDL operations ✓
  - CREATE/DROP INDEX CONCURRENTLY ✓
  - IDENTITY columns (ADD/DROP IDENTITY) ✓
  - Sequence operations (ALTER/DROP SEQUENCE) ✓
  - LIKE pattern indexes (varchar_pattern_ops, text_pattern_ops) ✓
  - IF EXISTS support for safer operations ✓

#### MySQL (Implemented ✓)

- **Connection Management**: Connection pooling via sqlx MySqlPool
- **Query Execution**: Full async query support with parameter binding
- **Type Mapping**: Comprehensive type conversion (primitives, timestamps, bytes, NULL)
- **Database Features**:
  - RETURNING clause support: No (MySQL limitation)
  - ON CONFLICT clause support: No (MySQL limitation)
  - Parameterized queries (?) ✓
- **Schema Editor**: Standard DDL operations ✓

#### SQLite (Implemented ✓)

- **Connection Management**: Connection pooling via sqlx SqlitePool
- **Query Execution**: Full async query support with parameter binding
- **Type Mapping**: Comprehensive type conversion (primitives, timestamps, bytes, NULL)
- **Database Features**:
  - RETURNING clause support ✓
  - ON CONFLICT clause support ✓
  - Parameterized queries (?) ✓
- **Schema Editor**: Standard DDL operations ✓

### Planned Features

#### PostgreSQL Advanced Features (Planned)

- Array field operations
- JSONB field operations and operators
- HStore field support
- Full-text search (tsvector, tsquery, search configurations)
- Range types (int4range, int8range, tsrange, etc.)
- Geometric types
- Network address types (inet, cidr, macaddr)
- UUID type support
- Custom types and domains

#### MySQL Advanced Features (Planned)

- JSON field operations and path expressions
- Full-text search (FULLTEXT index, MATCH AGAINST)
- Spatial data types and operations
- XA transaction support
- Partition management

#### SQLite Advanced Features (Planned)

- JSON1 extension operations
- FTS5 full-text search
- R\*Tree spatial index
- Virtual table support
- Common Table Expressions (CTE)

#### General Enhancements (Planned)

- Transaction management
- Connection pool configuration
- Query result streaming for large datasets
- Prepared statement caching
- Database migration support
- Connection health checks
- Retry logic for transient failures
- Database-specific error handling
- Query logging and metrics

## Installation

```toml
[dependencies]
# Default: PostgreSQL support only
reinhardt-database = "0.1.0"

# All SQL databases
reinhardt-database = { version = "0.1.0", features = ["all-databases"] }

# Custom combination
reinhardt-database = { version = "0.1.0", default-features = false, features = ["postgres", "mysql"] }

# SQLite only
reinhardt-database = { version = "0.1.0", default-features = false, features = ["sqlite"] }
```

## Usage Examples

### Basic Query Operations

```rust
use reinhardt_database::{DatabaseConnection, QueryValue};

// Connect to PostgreSQL
let conn = DatabaseConnection::connect_postgres("postgresql://localhost/mydb").await?;

// Insert data
let result = conn
    .insert("users")
    .value("name", "Alice")
    .value("email", "alice@example.com")
    .execute()
    .await?;

// Update data
conn.update("users")
    .set("email", "newemail@example.com")
    .where_eq("name", "Alice")
    .execute()
    .await?;

// Select data
let rows = conn
    .select()
    .columns(vec!["id", "name", "email"])
    .from("users")
    .where_eq("name", "Alice")
    .limit(10)
    .fetch_all()
    .await?;

// Delete data
conn.delete("users")
    .where_eq("id", QueryValue::Int(1))
    .execute()
    .await?;
```

### Schema Operations

```rust
use reinhardt_database::schema::factory::{SchemaEditorFactory, DatabaseType};

let factory = SchemaEditorFactory::new();
let editor = factory.create_for_database(DatabaseType::PostgreSQL);

// Generate CREATE TABLE SQL
let sql = editor.create_table_sql("users", &[
    ("id", "SERIAL PRIMARY KEY"),
    ("name", "VARCHAR(100) NOT NULL"),
    ("email", "VARCHAR(255) UNIQUE"),
    ("created_at", "TIMESTAMP DEFAULT NOW()"),
]);

// Generate CREATE INDEX SQL
let index_sql = editor.create_index_sql(
    "idx_users_email",
    "users",
    &["email"],
    false,
    None,
);
```

### PostgreSQL-Specific Features

```rust
use reinhardt_database::backends::postgresql::schema::PostgreSQLSchemaEditor;

let editor = PostgreSQLSchemaEditor::new();

// Create index without blocking writes
let sql = editor.create_index_concurrently_sql(
    "idx_email",
    "users",
    &["email"],
    false,
    None,
);

// Add IDENTITY column
let identity_sql = editor.add_identity_sql("users", "id");

// Create LIKE pattern index for text search
let like_index = editor.create_like_index_sql("users", "name", "varchar(100)");
```

### Multi-Database Support

```rust
use reinhardt_database::{DatabaseConnection, backend::DatabaseBackend};

// Connect to different databases
let pg_conn = DatabaseConnection::connect_postgres("postgresql://localhost/db").await?;
let mysql_conn = DatabaseConnection::connect_mysql("mysql://localhost/db").await?;
let sqlite_conn = DatabaseConnection::connect_sqlite("sqlite::memory:").await?;

// Use unified interface
async fn insert_user(conn: &DatabaseConnection, name: &str) -> Result<()> {
    conn.insert("users")
        .value("name", name)
        .execute()
        .await?;
    Ok(())
}
```

## Feature Flags

| Feature         | Description              | Default |
| --------------- | ------------------------ | ------- |
| `postgres`      | PostgreSQL support       | ✅      |
| `mysql`         | MySQL support            | ❌      |
| `sqlite`        | SQLite support           | ❌      |
| `all-databases` | Enable all SQL databases | ❌      |

## Architecture

### Trait-Based Design

```
DatabaseBackend (trait)
├── PostgresBackend (PostgreSQL implementation)
├── MySqlBackend (MySQL implementation)
└── SqliteBackend (SQLite implementation)

BaseDatabaseSchemaEditor (trait)
├── PostgreSQLSchemaEditor (with PG-specific operations)
├── MySQLSchemaEditor (standard DDL)
└── SQLiteSchemaEditor (standard DDL)
```

### Component Layers

1. **Type System**: `QueryValue`, `Row`, `QueryResult`, `DatabaseType`
2. **Backend Abstraction**: `DatabaseBackend` trait for database operations
3. **Connection Management**: `DatabaseConnection` wrapper with connection pooling
4. **Query Builders**: `SelectBuilder`, `InsertBuilder`, `UpdateBuilder`, `DeleteBuilder`
5. **Schema Editors**: `BaseDatabaseSchemaEditor` trait and database-specific implementations
6. **Dialect Implementations**: PostgreSQL, MySQL, SQLite backends

## Design Philosophy

- **Low-Level Operations**: Focused on database abstraction, not ORM functionality
- **Type Safety**: Leverages Rust's type system for compile-time guarantees
- **Async-First**: Built on async/await with sqlx for efficient I/O
- **Database Agnostic**: Unified interface with database-specific extensions
- **Extensible**: Easy to add new databases or extend existing ones

## Relationship with Other Crates

- **reinhardt-orm**: Uses `reinhardt-database` for low-level operations, provides high-level ORM features
- **reinhardt-migrations**: Uses schema editors for database migrations
- **reinhardt**: Main framework that integrates all components

## Performance Considerations

- Connection pooling via sqlx for efficient resource usage
- Parameterized queries to prevent SQL injection and enable prepared statement caching
- Type conversions are zero-cost where possible
- Async operations allow for concurrent database access

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! This crate is part of the Reinhardt framework. Please refer to the main [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.
