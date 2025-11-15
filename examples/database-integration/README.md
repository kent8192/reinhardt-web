# Database Integration Example

This example demonstrates database ORM, migrations, and database connection integration in the Reinhardt framework.

## Features

- **Django-style project structure**: Uses config/, settings/, apps.rs
- **Database configuration management**: Environment-specific database connection settings
- **Migration system**: Database schema version control
- **manage CLI**: Database management commands (makemigrations, migrate)

## Project Structure

```
src/
├── config/
│   ├── apps.rs              # Installed apps definition
│   ├── settings.rs          # Environment-based settings loader
│   ├── settings/
│   │   ├── base.rs          # Common settings for all environments
│   │   ├── local.rs         # Local development settings (including DB config)
│   │   ├── staging.rs       # Staging environment settings
│   │   └── production.rs    # Production environment settings
│   └── urls.rs              # URL routing configuration
├── migrations.rs            # Migration definitions
├── migrations/              # Migration files
│   └── 0001_initial.rs      # Initial migration
├── apps.rs                  # App registry
├── config.rs                # config module declaration
├── main.rs                  # Application entry point
└── bin/
    └── manage.rs            # Management CLI tool
```

## Setup

### Prerequisites

- Rust 2024 edition or later
- PostgreSQL, MySQL, or SQLite
- Cargo

### Database Setup

#### PostgreSQL (Recommended)

```bash
# Start PostgreSQL server
docker run -d \
  --name reinhardt-postgres \
  -e POSTGRES_USER=reinhardt \
  -e POSTGRES_PASSWORD=reinhardt_dev \
  -e POSTGRES_DB=reinhardt_examples \
  -p 5432:5432 \
  postgres:16
```

#### MySQL

```bash
# Start MySQL server
docker run -d \
  --name reinhardt-mysql \
  -e MYSQL_ROOT_PASSWORD=rootpass \
  -e MYSQL_DATABASE=reinhardt_examples \
  -e MYSQL_USER=reinhardt \
  -e MYSQL_PASSWORD=reinhardt_dev \
  -p 3306:3306 \
  mysql:8
```

#### SQLite

No additional setup required for SQLite.

### Build

```bash
# From project root
cargo build --package example-database-integration
```

**Note**: This example will be buildable after reinhardt is published to crates.io (version ^0.1).

## Usage

### Environment Variables

```bash
# PostgreSQL (default)
export DATABASE_URL="postgres://reinhardt:reinhardt_dev@localhost:5432/reinhardt_examples"

# MySQL
export DATABASE_URL="mysql://reinhardt:reinhardt_dev@localhost:3306/reinhardt_examples"

# SQLite
export DATABASE_URL="sqlite://./db.sqlite3"
```

### Migration Management

```bash
# Create new migration
cargo run --bin manage makemigrations

# Apply migrations
cargo run --bin manage migrate

# View migration plan (dry-run)
cargo run --bin manage migrate --plan

# Apply up to specific migration
cargo run --bin manage migrate app_name migration_name
```

### Running Application

```bash
cargo run --package example-database-integration
```

Output example:
```
Database Integration Example
✅ Application initialized
Debug mode: true
Database URL: postgres://reinhardt:reinhardt_dev@localhost:5432/reinhardt_examples
✅ Application started successfully
```

## Database Configuration

### Configuration in local.rs

```rust
use reinhardt_core::DatabaseConfig;

settings.database = Some(DatabaseConfig {
    url: database_url,
    max_connections: 10,
    min_connections: 1,
    connect_timeout: std::time::Duration::from_secs(30),
    idle_timeout: Some(std::time::Duration::from_secs(600)),
});
```

### Environment-Specific Settings

| Environment | File | Database URL | Connection Pool |
|-------------|------|--------------|-----------------|
| local | local.rs | Env var or default | 10 connections |
| staging | staging.rs | Env var required | 20 connections |
| production | production.rs | Env var required | 50 connections |

## Creating Migrations

### 1. Create Migration File

```bash
cargo run --bin manage makemigrations --name create_users_table
```

### 2. Create File in migrations/ Directory

```rust
// migrations/0002_create_users_table.rs
use reinhardt::prelude::*;

pub struct Migration;

impl MigrationTrait for Migration {
    fn name(&self) -> &str {
        "0002_create_users_table"
    }

    async fn up(&self, db: &Database) -> Result<()> {
        db.execute(r#"
            CREATE TABLE users (
                id SERIAL PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                email VARCHAR(255) UNIQUE NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        "#).await?;
        Ok(())
    }

    async fn down(&self, db: &Database) -> Result<()> {
        db.execute("DROP TABLE users").await?;
        Ok(())
    }
}
```

### 3. Register in migrations.rs

```rust
// src/migrations.rs
mod _0001_initial;
mod _0002_create_users_table;

pub fn all_migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![
        Box::new(_0001_initial::Migration),
        Box::new(_0002_create_users_table::Migration),
    ]
}
```

## Troubleshooting

### Connection Errors

```
Error: Database connection failed
```

**Solutions:**
1. Verify database server is running
2. Check DATABASE_URL environment variable is set correctly
3. Verify credentials (username, password) are correct

### Migration Errors

```
Error: Migration failed: table already exists
```

**Solutions:**
1. Mark migration as applied using `--fake` option
2. Or skip initial migration only with `--fake-initial`

```bash
cargo run --bin manage migrate --fake-initial
```

## References

- [Reinhardt ORM Documentation](https://docs.rs/reinhardt-orm)
- [Reinhardt Migrations Guide](https://docs.rs/reinhardt-migrations)
- [Django Migrations](https://docs.djangoproject.com/en/stable/topics/migrations/)
- [SQLAlchemy](https://www.sqlalchemy.org/)

## License

This example is provided as part of the Reinhardt project under MIT/Apache-2.0 license.
