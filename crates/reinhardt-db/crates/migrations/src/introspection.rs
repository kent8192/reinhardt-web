//! Database schema introspection
//!
//! This module provides functionality to read the current database schema
//! and extract table definitions, column metadata, indexes, and constraints.

use async_trait::async_trait;
use sea_schema::def::{ColumnDef, ColumnType, IndexDef, Schema, TableDef};
use std::collections::HashMap;

use crate::{MigrationError, Result};

/// Schema information extracted from a database
#[derive(Debug, Clone, PartialEq)]
pub struct DatabaseSchema {
    /// All tables in the schema
    pub tables: HashMap<String, TableInfo>,
}

/// Table metadata
#[derive(Debug, Clone, PartialEq)]
pub struct TableInfo {
    /// Table name
    pub name: String,
    /// Columns in the table
    pub columns: HashMap<String, ColumnInfo>,
    /// Indexes on the table
    pub indexes: HashMap<String, IndexInfo>,
    /// Primary key columns
    pub primary_key: Vec<String>,
    /// Foreign key constraints
    pub foreign_keys: Vec<ForeignKeyInfo>,
    /// Unique constraints (excluding unique indexes)
    pub unique_constraints: Vec<UniqueConstraintInfo>,
}

/// Column metadata
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnInfo {
    /// Column name
    pub name: String,
    /// Column type
    pub column_type: String,
    /// Whether the column is nullable
    pub nullable: bool,
    /// Default value expression
    pub default: Option<String>,
    /// Whether this is an auto-increment column
    pub auto_increment: bool,
}

/// Index metadata
#[derive(Debug, Clone, PartialEq)]
pub struct IndexInfo {
    /// Index name
    pub name: String,
    /// Columns in the index (in order)
    pub columns: Vec<String>,
    /// Whether the index is unique
    pub unique: bool,
    /// Index type (e.g., BTREE, HASH)
    pub index_type: Option<String>,
}

/// Foreign key constraint
#[derive(Debug, Clone, PartialEq)]
pub struct ForeignKeyInfo {
    /// Constraint name
    pub name: String,
    /// Columns in this table
    pub columns: Vec<String>,
    /// Referenced table
    pub referenced_table: String,
    /// Referenced columns
    pub referenced_columns: Vec<String>,
    /// ON DELETE action
    pub on_delete: Option<String>,
    /// ON UPDATE action
    pub on_update: Option<String>,
}

/// Unique constraint
#[derive(Debug, Clone, PartialEq)]
pub struct UniqueConstraintInfo {
    /// Constraint name
    pub name: String,
    /// Columns in the constraint
    pub columns: Vec<String>,
}

/// Trait for database-specific schema introspection
#[async_trait]
pub trait DatabaseIntrospector: Send + Sync {
    /// Read the complete database schema
    async fn read_schema(&self) -> Result<DatabaseSchema>;

    /// Read a specific table schema
    async fn read_table(&self, table_name: &str) -> Result<Option<TableInfo>>;
}

/// PostgreSQL schema introspector
#[cfg(feature = "postgres")]
pub struct PostgresIntrospector {
    pool: sqlx::PgPool,
}

#[cfg(feature = "postgres")]
impl PostgresIntrospector {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    fn convert_column_type(col_type: &ColumnType) -> String {
        match col_type {
            ColumnType::Serial => "INTEGER".to_string(),
            ColumnType::BigSerial => "BIGINT".to_string(),
            ColumnType::SmallSerial => "SMALLINT".to_string(),
            ColumnType::Integer => "INTEGER".to_string(),
            ColumnType::BigInteger => "BIGINT".to_string(),
            ColumnType::SmallInteger => "SMALLINT".to_string(),
            ColumnType::Text => "TEXT".to_string(),
            ColumnType::Varchar(len) => format!("VARCHAR({})", len.unwrap_or(255)),
            ColumnType::Char(len) => format!("CHAR({})", len.unwrap_or(1)),
            ColumnType::Boolean => "BOOLEAN".to_string(),
            ColumnType::Date => "DATE".to_string(),
            ColumnType::Time => "TIME".to_string(),
            ColumnType::Timestamp => "TIMESTAMP".to_string(),
            ColumnType::TimestampWithTimeZone => "TIMESTAMPTZ".to_string(),
            ColumnType::Decimal(precision, scale) => {
                format!(
                    "DECIMAL({}, {})",
                    precision.unwrap_or(10),
                    scale.unwrap_or(0)
                )
            }
            ColumnType::Float => "REAL".to_string(),
            ColumnType::Double => "DOUBLE PRECISION".to_string(),
            ColumnType::Uuid => "UUID".to_string(),
            ColumnType::Json => "JSON".to_string(),
            ColumnType::JsonBinary => "JSONB".to_string(),
            ColumnType::Binary(len) => format!("BYTEA({})", len.unwrap_or(255)),
            _ => "TEXT".to_string(), // fallback
        }
    }

    fn convert_table_def(table_def: &TableDef) -> Result<TableInfo> {
        let mut columns = HashMap::new();
        let mut primary_key = Vec::new();

        for column_def in &table_def.columns {
            let is_auto = matches!(
                column_def.col_type,
                ColumnType::Serial | ColumnType::BigSerial | ColumnType::SmallSerial
            );

            if column_def.key.is_primary() {
                primary_key.push(column_def.name.clone());
            }

            columns.insert(
                column_def.name.clone(),
                ColumnInfo {
                    name: column_def.name.clone(),
                    column_type: Self::convert_column_type(&column_def.col_type),
                    nullable: column_def.null,
                    default: column_def.default.clone(),
                    auto_increment: is_auto,
                },
            );
        }

        let mut indexes = HashMap::new();
        for index_def in &table_def.indexes {
            indexes.insert(
                index_def.name.clone(),
                IndexInfo {
                    name: index_def.name.clone(),
                    columns: index_def.columns.clone(),
                    unique: index_def.unique,
                    index_type: index_def.r#type.clone(),
                },
            );
        }

        Ok(TableInfo {
            name: table_def.info.name.clone(),
            columns,
            indexes,
            primary_key,
            foreign_keys: Vec::new(), // TODO: Extract from table_def
            unique_constraints: Vec::new(), // TODO: Extract from table_def
        })
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl DatabaseIntrospector for PostgresIntrospector {
    async fn read_schema(&self) -> Result<DatabaseSchema> {
        use sea_schema::postgres::discovery::SchemaDiscovery;

        let discovery = SchemaDiscovery::new(self.pool.clone(), "public");
        let schema: Schema = discovery
            .discover()
            .await
            .map_err(|e| MigrationError::IntrospectionError(e.to_string()))?;

        let mut tables = HashMap::new();
        for table_def in schema.tables {
            let table_info = Self::convert_table_def(&table_def)?;
            tables.insert(table_info.name.clone(), table_info);
        }

        Ok(DatabaseSchema { tables })
    }

    async fn read_table(&self, table_name: &str) -> Result<Option<TableInfo>> {
        let schema = self.read_schema().await?;
        Ok(schema.tables.get(table_name).cloned())
    }
}

/// MySQL schema introspector
#[cfg(feature = "mysql")]
pub struct MySQLIntrospector {
    pool: sqlx::MySqlPool,
}

#[cfg(feature = "mysql")]
impl MySQLIntrospector {
    pub fn new(pool: sqlx::MySqlPool) -> Self {
        Self { pool }
    }

    fn convert_column_type(col_type: &ColumnType) -> String {
        match col_type {
            ColumnType::Serial => "INT AUTO_INCREMENT".to_string(),
            ColumnType::BigSerial => "BIGINT AUTO_INCREMENT".to_string(),
            ColumnType::SmallSerial => "SMALLINT AUTO_INCREMENT".to_string(),
            ColumnType::Integer => "INT".to_string(),
            ColumnType::BigInteger => "BIGINT".to_string(),
            ColumnType::SmallInteger => "SMALLINT".to_string(),
            ColumnType::Text => "TEXT".to_string(),
            ColumnType::Varchar(len) => format!("VARCHAR({})", len.unwrap_or(255)),
            ColumnType::Char(len) => format!("CHAR({})", len.unwrap_or(1)),
            ColumnType::Boolean => "BOOLEAN".to_string(),
            ColumnType::Date => "DATE".to_string(),
            ColumnType::Time => "TIME".to_string(),
            ColumnType::Timestamp => "TIMESTAMP".to_string(),
            ColumnType::TimestampWithTimeZone => "TIMESTAMP".to_string(), // MySQL doesn't have explicit TZ support
            ColumnType::Decimal(precision, scale) => {
                format!(
                    "DECIMAL({}, {})",
                    precision.unwrap_or(10),
                    scale.unwrap_or(0)
                )
            }
            ColumnType::Float => "FLOAT".to_string(),
            ColumnType::Double => "DOUBLE".to_string(),
            ColumnType::Json => "JSON".to_string(),
            ColumnType::JsonBinary => "JSON".to_string(), // MySQL JSON is binary by default
            ColumnType::Binary(len) => format!("VARBINARY({})", len.unwrap_or(255)),
            _ => "TEXT".to_string(), // fallback
        }
    }

    fn convert_table_def(table_def: &TableDef) -> Result<TableInfo> {
        let mut columns = HashMap::new();
        let mut primary_key = Vec::new();

        for column_def in &table_def.columns {
            let is_auto = matches!(
                column_def.col_type,
                ColumnType::Serial | ColumnType::BigSerial | ColumnType::SmallSerial
            );

            if column_def.key.is_primary() {
                primary_key.push(column_def.name.clone());
            }

            columns.insert(
                column_def.name.clone(),
                ColumnInfo {
                    name: column_def.name.clone(),
                    column_type: Self::convert_column_type(&column_def.col_type),
                    nullable: column_def.null,
                    default: column_def.default.clone(),
                    auto_increment: is_auto,
                },
            );
        }

        let mut indexes = HashMap::new();
        for index_def in &table_def.indexes {
            indexes.insert(
                index_def.name.clone(),
                IndexInfo {
                    name: index_def.name.clone(),
                    columns: index_def.columns.clone(),
                    unique: index_def.unique,
                    index_type: index_def.r#type.clone(),
                },
            );
        }

        Ok(TableInfo {
            name: table_def.info.name.clone(),
            columns,
            indexes,
            primary_key,
            foreign_keys: Vec::new(), // TODO: Extract from table_def
            unique_constraints: Vec::new(), // TODO: Extract from table_def
        })
    }
}

#[cfg(feature = "mysql")]
#[async_trait]
impl DatabaseIntrospector for MySQLIntrospector {
    async fn read_schema(&self) -> Result<DatabaseSchema> {
        use sea_schema::mysql::discovery::SchemaDiscovery;

        let discovery = SchemaDiscovery::new(self.pool.clone());
        let schema: Schema = discovery
            .discover()
            .await
            .map_err(|e| MigrationError::IntrospectionError(e.to_string()))?;

        let mut tables = HashMap::new();
        for table_def in schema.tables {
            let table_info = Self::convert_table_def(&table_def)?;
            tables.insert(table_info.name.clone(), table_info);
        }

        Ok(DatabaseSchema { tables })
    }

    async fn read_table(&self, table_name: &str) -> Result<Option<TableInfo>> {
        let schema = self.read_schema().await?;
        Ok(schema.tables.get(table_name).cloned())
    }
}

/// SQLite schema introspector
#[cfg(feature = "sqlite")]
pub struct SQLiteIntrospector {
    pool: sqlx::SqlitePool,
}

#[cfg(feature = "sqlite")]
impl SQLiteIntrospector {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }

    fn convert_column_type(col_type: &ColumnType) -> String {
        match col_type {
            ColumnType::Serial | ColumnType::BigSerial | ColumnType::SmallSerial => {
                "INTEGER".to_string()
            }
            ColumnType::Integer | ColumnType::BigInteger | ColumnType::SmallInteger => {
                "INTEGER".to_string()
            }
            ColumnType::Text
            | ColumnType::Varchar(_)
            | ColumnType::Char(_)
            | ColumnType::Uuid
            | ColumnType::Json
            | ColumnType::JsonBinary => "TEXT".to_string(),
            ColumnType::Boolean => "INTEGER".to_string(), // SQLite stores booleans as integers
            ColumnType::Date | ColumnType::Time | ColumnType::Timestamp => "TEXT".to_string(),
            ColumnType::TimestampWithTimeZone => "TEXT".to_string(),
            ColumnType::Decimal(_, _) | ColumnType::Float | ColumnType::Double => {
                "REAL".to_string()
            }
            ColumnType::Binary(_) => "BLOB".to_string(),
            _ => "TEXT".to_string(), // fallback
        }
    }

    fn convert_table_def(table_def: &TableDef) -> Result<TableInfo> {
        let mut columns = HashMap::new();
        let mut primary_key = Vec::new();

        for column_def in &table_def.columns {
            let is_auto = matches!(
                column_def.col_type,
                ColumnType::Serial | ColumnType::BigSerial | ColumnType::SmallSerial
            );

            if column_def.key.is_primary() {
                primary_key.push(column_def.name.clone());
            }

            columns.insert(
                column_def.name.clone(),
                ColumnInfo {
                    name: column_def.name.clone(),
                    column_type: Self::convert_column_type(&column_def.col_type),
                    nullable: column_def.null,
                    default: column_def.default.clone(),
                    auto_increment: is_auto,
                },
            );
        }

        let mut indexes = HashMap::new();
        for index_def in &table_def.indexes {
            indexes.insert(
                index_def.name.clone(),
                IndexInfo {
                    name: index_def.name.clone(),
                    columns: index_def.columns.clone(),
                    unique: index_def.unique,
                    index_type: index_def.r#type.clone(),
                },
            );
        }

        Ok(TableInfo {
            name: table_def.info.name.clone(),
            columns,
            indexes,
            primary_key,
            foreign_keys: Vec::new(), // TODO: Extract from table_def
            unique_constraints: Vec::new(), // TODO: Extract from table_def
        })
    }
}

#[cfg(feature = "sqlite")]
#[async_trait]
impl DatabaseIntrospector for SQLiteIntrospector {
    async fn read_schema(&self) -> Result<DatabaseSchema> {
        use sea_schema::sqlite::discovery::SchemaDiscovery;

        let discovery = SchemaDiscovery::new(self.pool.clone());
        let schema: Schema = discovery
            .discover()
            .await
            .map_err(|e| MigrationError::IntrospectionError(e.to_string()))?;

        let mut tables = HashMap::new();
        for table_def in schema.tables {
            let table_info = Self::convert_table_def(&table_def)?;
            tables.insert(table_info.name.clone(), table_info);
        }

        Ok(DatabaseSchema { tables })
    }

    async fn read_table(&self, table_name: &str) -> Result<Option<TableInfo>> {
        let schema = self.read_schema().await?;
        Ok(schema.tables.get(table_name).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_sqlite_introspector_read_schema() {
        use sqlx::SqlitePool;

        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create pool");

        // Create a test table
        sqlx::query(
            r#"
            CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                email TEXT UNIQUE NOT NULL,
                age INTEGER
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create table");

        let introspector = SQLiteIntrospector::new(pool);
        let schema = introspector
            .read_schema()
            .await
            .expect("Failed to read schema");

        assert!(schema.tables.contains_key("users"));
        let users_table = &schema.tables["users"];
        assert_eq!(users_table.name, "users");
        assert_eq!(users_table.columns.len(), 4);

        // Check id column
        let id_col = &users_table.columns["id"];
        assert_eq!(id_col.name, "id");
        assert_eq!(id_col.column_type, "INTEGER");
        assert!(id_col.auto_increment);
        assert!(!id_col.nullable);

        // Check name column
        let name_col = &users_table.columns["name"];
        assert_eq!(name_col.name, "name");
        assert_eq!(name_col.column_type, "TEXT");
        assert!(!name_col.nullable);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_sqlite_introspector_read_table() {
        use sqlx::SqlitePool;

        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create pool");

        // Create a test table
        sqlx::query(
            r#"
            CREATE TABLE posts (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create table");

        let introspector = SQLiteIntrospector::new(pool);
        let table = introspector
            .read_table("posts")
            .await
            .expect("Failed to read table");

        assert!(table.is_some());
        let posts_table = table.unwrap();
        assert_eq!(posts_table.name, "posts");
        assert_eq!(posts_table.columns.len(), 3);

        // Check non-existent table
        let missing = introspector
            .read_table("non_existent")
            .await
            .expect("Failed to read table");
        assert!(missing.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_sqlite_introspector_with_indexes() {
        use sqlx::SqlitePool;

        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create pool");

        // Create table with indexes
        sqlx::query(
            r#"
            CREATE TABLE products (
                id INTEGER PRIMARY KEY,
                sku TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create table");

        sqlx::query("CREATE INDEX idx_products_name ON products(name)")
            .execute(&pool)
            .await
            .expect("Failed to create index");

        let introspector = SQLiteIntrospector::new(pool);
        let schema = introspector
            .read_schema()
            .await
            .expect("Failed to read schema");

        let products_table = &schema.tables["products"];
        assert!(!products_table.indexes.is_empty());
    }
}
