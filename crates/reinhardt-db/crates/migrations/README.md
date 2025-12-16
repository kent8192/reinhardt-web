# reinhardt-migrations

Database schema migration tools inspired by Django's migration system

## Overview

Database migration system for managing schema changes across PostgreSQL, MySQL, and SQLite. Automatically generates migrations from model changes, supports forward and backward migrations, and handles schema versioning with dependency management.

## Features

### Implemented ✓

#### Core Migration System

- **Migration Operations**: Comprehensive set of operations for schema changes
  - Model operations: `CreateModel`, `DeleteModel`, `RenameModel`
  - Field operations: `AddField`, `RemoveField`, `AlterField`, `RenameField`
  - Special operations: `RunSQL`, `RunCode` (Rust equivalent of Django's RunPython)
  - PostgreSQL-specific: `CreateExtension`, `DropExtension`, `CreateCollation`

- **State Management**: Track schema state across migrations
  - `ProjectState`: Maintains complete database schema state
  - `ModelState`: Represents individual model definitions
  - `FieldState`: Tracks field configurations
  - Support for indexes and constraints

- **Autodetection**: Automatically detect schema changes
  - `MigrationAutodetector`: Detects differences between states
  - Model creation/deletion detection
  - Field addition/removal/modification detection
  - Smart rename detection for models and fields
  - Index and constraint change detection

- **Migration Execution**
  - `MigrationExecutor`: Apply migrations to SQLite databases
  - `DatabaseMigrationExecutor`: Multi-database support (PostgreSQL, MySQL, SQLite)
  - Transaction support and rollback capability
  - Migration recorder for tracking applied migrations

- **Migration Management**
  - `MigrationLoader`: Load migrations from disk
  - `MigrationWriter`: Generate Rust migration files
  - Migration file serialization (JSON format)
  - Dependency tracking and validation

- **CLI Commands**
  - `makemigrations`: Generate migrations from model changes
    - Dry-run mode for previewing changes
    - Custom migration naming
    - App-specific migration generation
  - `migrate`: Apply migrations to database
    - Fake migrations support
    - Migration plan preview

- **Migration State Management**
  - `MigrationStateLoader`: Django-style state reconstruction from migration history
    - Build `ProjectState` by replaying applied migrations in topological order
    - Avoid direct database introspection for change detection
    - Ensure schema state consistency with migration files

- **Database Backend Support**
  - SQLite support via sqlx
  - PostgreSQL support via reinhardt-database
  - MySQL support via reinhardt-database
  - SQL dialect abstraction for cross-database compatibility

- **Dependency Injection Integration**
  - `MigrationService`: DI-compatible service for migrations
  - `MigrationConfig`: Configuration management
  - Integration with reinhardt-di

#### Advanced Features

- **Migration Graph**: Complete dependency resolution system (graph.rs skeleton exists)
- **Migration Squashing**: Combine multiple migrations into one for performance
- **Data Migrations**: Built-in support for complex data transformations
- **Zero-downtime Migrations**: Safe schema changes without service interruption
- **Migration Optimization**: Automatic operation reordering and combining
- **Atomic Operations**: Better transaction handling for complex migrations
- **Schema History Visualization**: Graphical representation of migration history

#### Enhanced Autodetection

- **Field Default Detection**: Automatically detect default value changes
- **Constraint Detection**: Better support for CHECK, UNIQUE, and FOREIGN KEY constraints
- **Index Optimization**: Suggest index additions based on model relationships

#### Database-Specific Features

- **PostgreSQL**: Advanced types (JSONB, Arrays, Custom types)
- **MySQL**: Storage engine management, partition support
- **SQLite**: Better handling of ALTER TABLE limitations

#### Developer Experience

- **Interactive Mode**: Guided migration creation
- **Conflict Resolution**: Automatic handling of migration conflicts
- **Migration Testing**: Built-in tools for testing migrations
- **Performance Profiling**: Measure migration execution time and identify bottlenecks

## Usage

### Basic Example

```rust
use reinhardt_migrations::{
    MigrationAutodetector, ProjectState, ModelState, FieldState,
    MakeMigrationsCommand, MakeMigrationsOptions,
};

// Define your models
let mut to_state = ProjectState::new();
let mut user_model = ModelState::new("myapp", "User");
user_model.add_field(FieldState::new("id".to_string(), "INTEGER".to_string(), false));
user_model.add_field(FieldState::new("email".to_string(), "VARCHAR(255)".to_string(), false));
to_state.add_model(user_model);

// Detect changes
let from_state = ProjectState::new(); // Empty state for initial migration
let detector = MigrationAutodetector::new(from_state, to_state);
let migrations = detector.generate_migrations();

// Generate migration files
let options = MakeMigrationsOptions {
    app_label: Some("myapp".to_string()),
    dry_run: false,
    ..Default::default()
};
let command = MakeMigrationsCommand::new(options);
let created_files = command.execute();
```

### Composite Primary Keys

Migrations support composite primary keys through the `CreateModel` operation:

```rust
use reinhardt_migrations::{
    operations::{CreateModel, FieldDefinition},
    Migration,
};

// Create a migration with composite primary key
let migration = Migration::new("0001_initial", "myapp")
    .add_operation(
        CreateModel::new(
            "post_tags",
            vec![
                FieldDefinition::new("post_id", "INTEGER", true, false, None),
                FieldDefinition::new("tag_id", "INTEGER", true, false, None),
                FieldDefinition::new("description", "VARCHAR(200)", false, false, None),
            ],
        )
        .with_composite_primary_key(vec!["post_id".to_string(), "tag_id".to_string()])
    );

// This generates SQL like:
// CREATE TABLE post_tags (
//     post_id INTEGER NOT NULL,
//     tag_id INTEGER NOT NULL,
//     description VARCHAR(200) NOT NULL,
//     PRIMARY KEY (post_id, tag_id)
// );
```

When using the `#[model(...)]` attribute macro with multiple `primary_key = true` fields, migrations will automatically detect and generate composite primary key constraints:

> **Note**: The `#[model(...)]` attribute automatically applies `#[derive(Model)]`, so you should use only `#[model(...)]` without explicitly adding `#[derive(Model)]`.

```rust
use reinhardt_macros::Model;
use serde::{Deserialize, Serialize};

#[model(app_label = "myapp", table_name = "post_tags")]
struct PostTag {
    #[field(primary_key = true)]
    post_id: i64,

    #[field(primary_key = true)]
    tag_id: i64,

    #[field(max_length = 200)]
    description: String,
}

// The migration autodetector will recognize this as a composite primary key
// and generate the appropriate CreateModel operation with composite_primary_key
```

### Applying Migrations

```rust
use reinhardt_migrations::{
    MigrationExecutor, Migration, Operation, ColumnDefinition,
};
use sqlx::SqlitePool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = SqlitePool::connect("sqlite::memory:").await?;
    let mut executor = MigrationExecutor::new(pool);

    let migration = Migration::new("0001_initial", "myapp")
        .add_operation(Operation::CreateTable {
            name: "users".to_string(),
            columns: vec![
                ColumnDefinition::new("id", "INTEGER PRIMARY KEY"),
                ColumnDefinition::new("email", "VARCHAR(255) NOT NULL"),
            ],
            constraints: vec![],
        });

    let result = executor.apply_migrations(&[migration]).await?;
    println!("Applied migrations: {:?}", result.applied);

    Ok(())
}
```

## Integration with Reinhardt Framework

This crate is part of the Reinhardt framework and integrates with:

- `reinhardt-database`: Database abstraction layer
- `reinhardt-di`: Dependency injection system
- `reinhardt-orm`: Object-relational mapping (future integration)

## Advanced Features

### Schema Editor

The `SchemaEditor` provides low-level control over database schema changes with transaction support:

```rust
use reinhardt_migrations::schema_editor::{SchemaEditor, EditorMode};

// Atomic mode: all operations in a single transaction
let mut editor = SchemaEditor::new(pool.clone(), EditorMode::Atomic).await?;

// Execute schema changes
editor.execute("CREATE TABLE users (id INTEGER PRIMARY KEY)").await?;
editor.execute("CREATE INDEX idx_users_email ON users(email)").await?;

// Commit all changes atomically
editor.finish().await?;

// Or rollback if needed
// editor.rollback().await?;
```

#### Editor Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| `Atomic` | All operations in single transaction | Production migrations |
| `Deferred` | Queue operations, execute on finish | Batch operations |

#### Methods

- `execute(sql)` - Execute SQL immediately (Atomic) or queue (Deferred)
- `defer(sql)` - Queue SQL for later execution
- `finish()` - Commit transaction or execute deferred statements
- `rollback()` - Rollback all changes
- `is_atomic()` - Check if database supports transactions
- `database_type()` - Get database dialect (SQLite, PostgreSQL, MySQL)

### Foreign Key Actions

Define how foreign key constraints behave on parent row changes:

```rust
use reinhardt_migrations::ForeignKeyAction;

Operation::CreateTable {
    name: "posts".to_string(),
    columns: vec![
        ColumnDefinition::new("id", "INTEGER PRIMARY KEY"),
        ColumnDefinition::new("author_id", "INTEGER"),
    ],
    constraints: vec![
        Constraint::ForeignKey {
            columns: vec!["author_id".to_string()],
            references_table: "users".to_string(),
            references_columns: vec!["id".to_string()],
            on_delete: ForeignKeyAction::Cascade,
            on_update: ForeignKeyAction::Restrict,
        }
    ],
}
```

#### Available Actions

| Action | Description | Behavior |
|--------|-------------|----------|
| `Cascade` | Propagate changes | Delete/update child rows automatically |
| `Restrict` | Prevent changes | Raise error if child rows exist |
| `SetNull` | Nullify references | Set foreign key column to NULL |
| `SetDefault` | Use default value | Set foreign key column to its default |
| `NoAction` | No automatic action | Similar to Restrict (database-dependent) |

#### Examples

```rust
// Cascade delete: deleting a user deletes all their posts
on_delete: ForeignKeyAction::Cascade

// Prevent deletion if posts exist
on_delete: ForeignKeyAction::Restrict

// Set author_id to NULL when user is deleted
on_delete: ForeignKeyAction::SetNull

// Set author_id to default value when user is deleted
on_delete: ForeignKeyAction::SetDefault
```

## Testing

### Prerequisites

Migration tests require **Docker** for TestContainers integration:

```bash
# Verify Docker is running
docker version
docker ps
```

**Note**: Docker Desktop must be installed and running. See [Database Testing Guide](../../README.md#testing) for detailed setup instructions.

### Running Migration Tests

```bash
# Run all migration tests (requires Docker)
cargo test --package reinhardt-migrations --all-features

# Run specific test suite
cargo test --package reinhardt-migrations --test migration_integration_tests

# Run with custom database URL
TEST_DATABASE_URL=postgres://postgres@localhost:5432/postgres cargo test
```

### TestContainers Usage

Migration tests automatically use TestContainers to:
- Start PostgreSQL 17 Alpine container before tests
- Provide isolated database instance for migration operations
- Clean up containers after tests complete

```rust
use reinhardt::test::fixtures::postgres_container;
use rstest::*;

#[rstest]
#[tokio::test]
async fn test_migrations(
    #[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
    let (_container, pool, _port, _database_url) = postgres_container.await;

    // Run migrations on the test database
    let migrator = Migrator::new(pool.clone());
    migrator.run_all().await.unwrap();

    // Container automatically cleaned up after test
}
```

For comprehensive testing standards, see:
- [Parent Database Testing Guide](../../README.md#testing)
- [Testing Standards](../../../../docs/TESTING_STANDARDS.md)

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT license

at your option.
