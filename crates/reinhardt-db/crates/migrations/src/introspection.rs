//! Database schema introspection
//!
//! This module provides functionality to read the current database schema
//! and extract table definitions, column metadata, indexes, and constraints.

use async_trait::async_trait;
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

    fn convert_column_type(col_type: &sea_schema::postgres::def::Type) -> String {
        use sea_schema::postgres::def::Type;
        match col_type {
            Type::Serial => "INTEGER".to_string(),
            Type::BigSerial => "BIGINT".to_string(),
            Type::SmallSerial => "SMALLINT".to_string(),
            Type::Integer => "INTEGER".to_string(),
            Type::BigInt => "BIGINT".to_string(),
            Type::SmallInt => "SMALLINT".to_string(),
            Type::Text => "TEXT".to_string(),
            Type::Varchar(attr) => {
                if let Some(len) = attr.length {
                    format!("VARCHAR({})", len)
                } else {
                    "VARCHAR".to_string()
                }
            }
            Type::Char(attr) => {
                if let Some(len) = attr.length {
                    format!("CHAR({})", len)
                } else {
                    "CHAR".to_string()
                }
            }
            Type::Boolean => "BOOLEAN".to_string(),
            Type::Date => "DATE".to_string(),
            Type::Time(_) => "TIME".to_string(),
            Type::TimeWithTimeZone(_) => "TIME WITH TIME ZONE".to_string(),
            Type::Timestamp(_) => "TIMESTAMP".to_string(),
            Type::TimestampWithTimeZone(_) => "TIMESTAMPTZ".to_string(),
            Type::Numeric(attr) | Type::Decimal(attr) => {
                if let (Some(precision), Some(scale)) = (attr.precision, attr.scale) {
                    format!("DECIMAL({}, {})", precision, scale)
                } else {
                    "DECIMAL".to_string()
                }
            }
            Type::Real => "REAL".to_string(),
            Type::DoublePrecision => "DOUBLE PRECISION".to_string(),
            Type::Uuid => "UUID".to_string(),
            Type::Json => "JSON".to_string(),
            Type::JsonBinary => "JSONB".to_string(),
            Type::Bytea => "BYTEA".to_string(),
            _ => "TEXT".to_string(), // fallback
        }
    }

    fn convert_table_def(table_def: &sea_schema::postgres::def::TableDef) -> Result<TableInfo> {
        use sea_schema::postgres::def::Type;
        let mut columns = HashMap::new();

        // Extract primary key from constraints
        let mut primary_key = Vec::new();
        for pk_constraint in &table_def.primary_key_constraints {
            primary_key.extend(pk_constraint.columns.clone());
        }

        for column_def in &table_def.columns {
            let is_auto = matches!(
                column_def.col_type,
                Type::Serial | Type::BigSerial | Type::SmallSerial
            ) || column_def.is_identity;

            let default_str = column_def.default.as_ref().map(|expr| expr.0.clone());

            columns.insert(
                column_def.name.clone(),
                ColumnInfo {
                    name: column_def.name.clone(),
                    column_type: Self::convert_column_type(&column_def.col_type),
                    nullable: column_def.not_null.is_none(),
                    default: default_str,
                    auto_increment: is_auto,
                },
            );
        }

        // PostgreSQL doesn't have separate index information in TableDef
        // Indexes would need to be queried separately
        let indexes = HashMap::new();

        // Extract foreign keys from reference_constraints
        let foreign_keys = table_def
            .reference_constraints
            .iter()
            .map(|ref_constraint| {
                use sea_schema::postgres::def::ForeignKeyAction;

                let convert_action = |action: &Option<ForeignKeyAction>| -> Option<String> {
                    action.as_ref().map(|a| match a {
                        ForeignKeyAction::Cascade => "CASCADE".to_string(),
                        ForeignKeyAction::SetNull => "SET NULL".to_string(),
                        ForeignKeyAction::SetDefault => "SET DEFAULT".to_string(),
                        ForeignKeyAction::Restrict => "RESTRICT".to_string(),
                        ForeignKeyAction::NoAction => "NO ACTION".to_string(),
                    })
                };

                ForeignKeyInfo {
                    name: ref_constraint.name.clone(),
                    columns: ref_constraint.columns.clone(),
                    referenced_table: ref_constraint.table.clone(),
                    referenced_columns: ref_constraint.foreign_columns.clone(),
                    on_delete: convert_action(&ref_constraint.on_delete),
                    on_update: convert_action(&ref_constraint.on_update),
                }
            })
            .collect();

        // Extract unique constraints
        let unique_constraints = table_def
            .unique_constraints
            .iter()
            .map(|unique_constraint| UniqueConstraintInfo {
                name: unique_constraint.name.clone(),
                columns: unique_constraint.columns.clone(),
            })
            .collect();

        Ok(TableInfo {
            name: table_def.info.name.clone(),
            columns,
            indexes,
            primary_key,
            foreign_keys,
            unique_constraints,
        })
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl DatabaseIntrospector for PostgresIntrospector {
    async fn read_schema(&self) -> Result<DatabaseSchema> {
        use sea_schema::postgres::discovery::SchemaDiscovery;

        let discovery = SchemaDiscovery::new(self.pool.clone(), "public");
        let schema = discovery
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

    fn convert_column_type(col_type: &sea_schema::mysql::def::Type) -> String {
        use sea_schema::mysql::def::Type;
        match col_type {
            Type::Serial => "BIGINT UNSIGNED AUTO_INCREMENT".to_string(),
            Type::TinyInt(_) => "TINYINT".to_string(),
            Type::Bool => "BOOLEAN".to_string(),
            Type::SmallInt(_) => "SMALLINT".to_string(),
            Type::MediumInt(_) => "MEDIUMINT".to_string(),
            Type::Int(_) => "INT".to_string(),
            Type::BigInt(_) => "BIGINT".to_string(),
            Type::Decimal(attr) => {
                if let (Some(precision), Some(scale)) = (attr.maximum, attr.decimal) {
                    format!("DECIMAL({}, {})", precision, scale)
                } else {
                    "DECIMAL".to_string()
                }
            }
            Type::Float(_) => "FLOAT".to_string(),
            Type::Double(_) => "DOUBLE".to_string(),
            Type::Char(attr) | Type::NChar(attr) => {
                if let Some(len) = attr.length {
                    format!("CHAR({})", len)
                } else {
                    "CHAR".to_string()
                }
            }
            Type::Varchar(attr) | Type::NVarchar(attr) => {
                if let Some(len) = attr.length {
                    format!("VARCHAR({})", len)
                } else {
                    "VARCHAR".to_string()
                }
            }
            Type::Binary(attr) => {
                if let Some(len) = attr.length {
                    format!("BINARY({})", len)
                } else {
                    "BINARY".to_string()
                }
            }
            Type::Varbinary(attr) => {
                if let Some(len) = attr.length {
                    format!("VARBINARY({})", len)
                } else {
                    "VARBINARY".to_string()
                }
            }
            Type::Text(_) => "TEXT".to_string(),
            Type::TinyText(_) => "TINYTEXT".to_string(),
            Type::MediumText(_) => "MEDIUMTEXT".to_string(),
            Type::LongText(_) => "LONGTEXT".to_string(),
            Type::Blob(_) => "BLOB".to_string(),
            Type::TinyBlob => "TINYBLOB".to_string(),
            Type::MediumBlob => "MEDIUMBLOB".to_string(),
            Type::LongBlob => "LONGBLOB".to_string(),
            Type::Date => "DATE".to_string(),
            Type::Time(_) => "TIME".to_string(),
            Type::DateTime(_) => "DATETIME".to_string(),
            Type::Timestamp(_) => "TIMESTAMP".to_string(),
            Type::Year => "YEAR".to_string(),
            Type::Json => "JSON".to_string(),
            Type::Enum(enum_def) => {
                let values = enum_def.values.join("','");
                format!("ENUM('{}')", values)
            }
            Type::Set(set_def) => {
                let members = set_def.members.join("','");
                format!("SET('{}')", members)
            }
            Type::Bit(_) | Type::Geometry(_) | Type::Point(_) | Type::LineString(_)
            | Type::Polygon(_) | Type::MultiPoint(_) | Type::MultiLineString(_)
            | Type::MultiPolygon(_) | Type::GeometryCollection(_) => {
                format!("{:?}", col_type).to_uppercase()
            }
            Type::Unknown(s) => s.clone(),
        }
    }

    fn convert_foreign_key_action(action: &sea_schema::mysql::def::ForeignKeyAction) -> String {
        use sea_schema::mysql::def::ForeignKeyAction;
        match action {
            ForeignKeyAction::Cascade => "CASCADE".to_string(),
            ForeignKeyAction::SetNull => "SET NULL".to_string(),
            ForeignKeyAction::Restrict => "RESTRICT".to_string(),
            ForeignKeyAction::NoAction => "NO ACTION".to_string(),
            ForeignKeyAction::SetDefault => "SET DEFAULT".to_string(),
        }
    }

    fn convert_index_type(idx_type: &sea_schema::mysql::def::IndexType) -> String {
        use sea_schema::mysql::def::IndexType;
        match idx_type {
            IndexType::BTree => "BTREE".to_string(),
            IndexType::Hash => "HASH".to_string(),
            IndexType::FullText => "FULLTEXT".to_string(),
            IndexType::Spatial => "SPATIAL".to_string(),
            IndexType::RTree => "RTREE".to_string(),
        }
    }

    fn convert_table_def(table_def: &sea_schema::mysql::def::TableDef) -> Result<TableInfo> {
        let mut columns = HashMap::new();
        let mut primary_key = Vec::new();

        for column_def in &table_def.columns {
            let is_auto = column_def.extra.auto_increment;

            if column_def.key == sea_schema::mysql::def::ColumnKey::Primary {
                primary_key.push(column_def.name.clone());
            }

            let default_str = column_def.default.as_ref().map(|def| {
                use sea_schema::mysql::def::ColumnDefault;
                match def {
                    ColumnDefault::Null => "NULL".to_string(),
                    ColumnDefault::Int(i) => i.to_string(),
                    ColumnDefault::Real(f) => f.to_string(),
                    ColumnDefault::String(s) => s.clone(),
                    ColumnDefault::CustomExpr(s) => s.clone(),
                    ColumnDefault::CurrentTimestamp => "CURRENT_TIMESTAMP".to_string(),
                }
            });

            columns.insert(
                column_def.name.clone(),
                ColumnInfo {
                    name: column_def.name.clone(),
                    column_type: Self::convert_column_type(&column_def.col_type),
                    nullable: column_def.null,
                    default: default_str,
                    auto_increment: is_auto,
                },
            );
        }

        // Extract unique constraints from indexes
        let mut unique_constraints = Vec::new();
        let mut indexes = HashMap::new();
        for index_def in &table_def.indexes {
            let columns: Vec<String> = index_def
                .parts
                .iter()
                .map(|p| p.column.clone())
                .collect();

            if index_def.unique {
                unique_constraints.push(UniqueConstraintInfo {
                    name: index_def.name.clone(),
                    columns: columns.clone(),
                });
            }

            indexes.insert(
                index_def.name.clone(),
                IndexInfo {
                    name: index_def.name.clone(),
                    columns,
                    unique: index_def.unique,
                    index_type: Some(Self::convert_index_type(&index_def.idx_type)),
                },
            );
        }

        // Extract foreign keys
        let foreign_keys: Vec<ForeignKeyInfo> = table_def
            .foreign_keys
            .iter()
            .map(|fk| ForeignKeyInfo {
                name: fk.name.clone(),
                columns: fk.columns.clone(),
                referenced_table: fk.referenced_table.clone(),
                referenced_columns: fk.referenced_columns.clone(),
                on_delete: Some(Self::convert_foreign_key_action(&fk.on_delete)),
                on_update: Some(Self::convert_foreign_key_action(&fk.on_update)),
            })
            .collect();

        Ok(TableInfo {
            name: table_def.info.name.clone(),
            columns,
            indexes,
            primary_key,
            foreign_keys,
            unique_constraints,
        })
    }
}

#[cfg(feature = "mysql")]
#[async_trait]
impl DatabaseIntrospector for MySQLIntrospector {
    async fn read_schema(&self) -> Result<DatabaseSchema> {
        use sea_schema::mysql::discovery::SchemaDiscovery;

        // MySQL SchemaDiscovery::new requires a schema name (database name)
        // We use empty string to discover the current database
        let discovery = SchemaDiscovery::new(self.pool.clone(), "");
        let schema = discovery
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

    fn convert_column_type(col_type: &sea_schema::sea_query::ColumnType) -> String {
        use sea_schema::sea_query::ColumnType;
        match col_type {
            ColumnType::TinyInteger | ColumnType::SmallInteger | ColumnType::Integer | ColumnType::BigInteger => {
                "INTEGER".to_string()
            }
            ColumnType::Float | ColumnType::Double | ColumnType::Decimal(_) => {
                "REAL".to_string()
            }
            ColumnType::String(_) | ColumnType::Text | ColumnType::Char(_) => {
                "TEXT".to_string()
            }
            ColumnType::Binary(_) => "BLOB".to_string(),
            ColumnType::Boolean => "INTEGER".to_string(), // SQLite uses INTEGER for boolean
            _ => "TEXT".to_string(), // fallback
        }
    }

    #[allow(dead_code)]
    fn convert_foreign_key_action(action: &sea_schema::sqlite::def::ForeignKeyAction) -> String {
        use sea_schema::sqlite::def::ForeignKeyAction;
        match action {
            ForeignKeyAction::Cascade => "CASCADE".to_string(),
            ForeignKeyAction::SetNull => "SET NULL".to_string(),
            ForeignKeyAction::SetDefault => "SET DEFAULT".to_string(),
            ForeignKeyAction::Restrict => "RESTRICT".to_string(),
            ForeignKeyAction::NoAction => "NO ACTION".to_string(),
        }
    }

    /// Extracts foreign key information from SQLite using PRAGMA foreign_key_list.
    ///
    /// SQLite's PRAGMA foreign_key_list returns:
    /// - id: FK constraint ID (for multi-column FKs, same ID for all columns)
    /// - seq: Column sequence in the FK
    /// - table: Referenced table name
    /// - from: Source column name
    /// - to: Referenced column name
    /// - on_update: ON UPDATE action
    /// - on_delete: ON DELETE action
    /// - match: MATCH clause (usually 'NONE')
    async fn extract_foreign_keys(
        pool: &sqlx::SqlitePool,
        table_name: &str,
    ) -> Result<Vec<ForeignKeyInfo>> {
        #[derive(sqlx::FromRow)]
        struct ForeignKeyRow {
            id: i64,
            seq: i64,
            table: String,
            from: String,
            to: String,
            on_update: String,
            on_delete: String,
            #[allow(dead_code)]
            r#match: String,
        }

        let query = format!("PRAGMA foreign_key_list({})", table_name);
        let rows: Vec<ForeignKeyRow> = sqlx::query_as(&query)
            .fetch_all(pool)
            .await
            .map_err(|e| MigrationError::IntrospectionError(e.to_string()))?;

        // Group by FK ID to handle multi-column foreign keys
        let mut fk_map: HashMap<i64, Vec<ForeignKeyRow>> = HashMap::new();
        for row in rows {
            fk_map.entry(row.id).or_insert_with(Vec::new).push(row);
        }

        // Convert to ForeignKeyInfo
        let mut foreign_keys = Vec::new();
        for (fk_id, mut fk_rows) in fk_map {
            // Sort by sequence to maintain column order
            fk_rows.sort_by_key(|r| r.seq);

            let referenced_table = fk_rows[0].table.clone();
            let on_update = fk_rows[0].on_update.clone();
            let on_delete = fk_rows[0].on_delete.clone();

            let columns: Vec<String> = fk_rows.iter().map(|r| r.from.clone()).collect();
            let referenced_columns: Vec<String> = fk_rows.iter().map(|r| r.to.clone()).collect();

            // Generate FK constraint name
            let name = format!("fk_{}_{}", table_name, fk_id);

            foreign_keys.push(ForeignKeyInfo {
                name,
                columns,
                referenced_table,
                referenced_columns,
                on_delete: if on_delete == "NO ACTION" {
                    None
                } else {
                    Some(on_delete)
                },
                on_update: if on_update == "NO ACTION" {
                    None
                } else {
                    Some(on_update)
                },
            });
        }

        Ok(foreign_keys)
    }

    /// Extracts index information from SQLite using PRAGMA index_list and PRAGMA index_info.
    ///
    /// Note: sea_schema doesn't detect regular (non-unique) indexes created with CREATE INDEX,
    /// so we need to use PRAGMA commands to get complete index information.
    async fn extract_indexes(
        pool: &sqlx::SqlitePool,
        table_name: &str,
    ) -> Result<HashMap<String, IndexInfo>> {
        #[derive(sqlx::FromRow)]
        struct IndexListRow {
            #[allow(dead_code)]
            seq: i64,
            name: String,
            unique: i64,
            #[allow(dead_code)]
            origin: String,
            #[allow(dead_code)]
            partial: i64,
        }

        #[derive(sqlx::FromRow)]
        struct IndexInfoRow {
            #[allow(dead_code)]
            seqno: i64,
            #[allow(dead_code)]
            cid: i64,
            name: Option<String>,
        }

        let mut indexes = HashMap::new();

        // Get list of indexes for the table
        let query = format!("PRAGMA index_list({})", table_name);
        let index_list: Vec<IndexListRow> = sqlx::query_as(&query)
            .fetch_all(pool)
            .await
            .map_err(|e| MigrationError::IntrospectionError(e.to_string()))?;

        for index_row in index_list {
            // Get columns for this index
            let info_query = format!("PRAGMA index_info({})", index_row.name);
            let index_info: Vec<IndexInfoRow> = sqlx::query_as(&info_query)
                .fetch_all(pool)
                .await
                .map_err(|e| MigrationError::IntrospectionError(e.to_string()))?;

            let columns: Vec<String> = index_info
                .into_iter()
                .filter_map(|info| info.name)
                .collect();

            indexes.insert(
                index_row.name.clone(),
                IndexInfo {
                    name: index_row.name,
                    columns,
                    unique: index_row.unique != 0,
                    index_type: None,
                },
            );
        }

        Ok(indexes)
    }

    fn convert_table_def(table_def: &sea_schema::sqlite::def::TableDef) -> Result<TableInfo> {
        let mut columns = HashMap::new();
        let mut primary_key = Vec::new();

        for column_def in &table_def.columns {
            let is_auto = table_def.auto_increment && column_def.primary_key;

            if column_def.primary_key {
                primary_key.push(column_def.name.clone());
            }

            // In SQLite, PRIMARY KEY columns are implicitly NOT NULL
            let is_nullable = if column_def.primary_key {
                false
            } else {
                !column_def.not_null
            };

            columns.insert(
                column_def.name.clone(),
                ColumnInfo {
                    name: column_def.name.clone(),
                    column_type: Self::convert_column_type(&column_def.r#type),
                    nullable: is_nullable,
                    default: match &column_def.default_value {
                        sea_schema::sqlite::def::DefaultType::String(s) => Some(s.clone()),
                        sea_schema::sqlite::def::DefaultType::Integer(i) => Some(i.to_string()),
                        sea_schema::sqlite::def::DefaultType::Float(f) => Some(f.to_string()),
                        sea_schema::sqlite::def::DefaultType::CurrentTimestamp => Some("CURRENT_TIMESTAMP".to_string()),
                        sea_schema::sqlite::def::DefaultType::Null | sea_schema::sqlite::def::DefaultType::Unspecified => None,
                    },
                    auto_increment: is_auto,
                },
            );
        }

        // Extract indexes from table_def (UNIQUE constraints detected by sea_schema)
        // Note: Regular indexes will be extracted separately using PRAGMA index_list
        let mut indexes = HashMap::new();
        for index_def in &table_def.indexes {
            indexes.insert(
                index_def.index_name.clone(),
                IndexInfo {
                    name: index_def.index_name.clone(),
                    columns: index_def.columns.clone(),
                    unique: index_def.unique,
                    index_type: None,
                },
            );
        }

        // Extract unique constraints from table_def.constraints
        let mut unique_constraints = Vec::new();
        for constraint_def in &table_def.constraints {
            if constraint_def.unique {
                unique_constraints.push(UniqueConstraintInfo {
                    name: constraint_def.index_name.clone(),
                    columns: constraint_def.columns.clone(),
                });
            }
        }

        // Foreign keys will be extracted separately using PRAGMA foreign_key_list
        // in the read_schema method
        let foreign_keys: Vec<ForeignKeyInfo> = Vec::new();

        Ok(TableInfo {
            name: table_def.name.clone(),
            columns,
            indexes,
            primary_key,
            foreign_keys,
            unique_constraints,
        })
    }
}

#[cfg(feature = "sqlite")]
#[async_trait]
impl DatabaseIntrospector for SQLiteIntrospector {
    async fn read_schema(&self) -> Result<DatabaseSchema> {
        use sea_schema::sqlite::discovery::SchemaDiscovery;

        let discovery = SchemaDiscovery::new(self.pool.clone());
        let schema = discovery
            .discover()
            .await
            .map_err(|e| MigrationError::IntrospectionError(e.to_string()))?;

        let mut tables = HashMap::new();
        for table_def in schema.tables {
            let mut table_info = Self::convert_table_def(&table_def)?;

            // Extract foreign keys using PRAGMA foreign_key_list
            let foreign_keys = Self::extract_foreign_keys(&self.pool, &table_info.name).await?;
            table_info.foreign_keys = foreign_keys;

            // Extract indexes using PRAGMA index_list (sea_schema doesn't detect regular indexes)
            let indexes = Self::extract_indexes(&self.pool, &table_info.name).await?;
            table_info.indexes = indexes;

            tables.insert(table_info.name.clone(), table_info);
        }

        Ok(DatabaseSchema { tables })
    }

    async fn read_table(&self, table_name: &str) -> Result<Option<TableInfo>> {
        use sea_schema::sqlite::discovery::SchemaDiscovery;

        let discovery = SchemaDiscovery::new(self.pool.clone());
        let schema = discovery
            .discover()
            .await
            .map_err(|e| MigrationError::IntrospectionError(e.to_string()))?;

        for table_def in schema.tables {
            if table_def.name == table_name {
                let mut table_info = Self::convert_table_def(&table_def)?;

                // Extract foreign keys using PRAGMA foreign_key_list
                let foreign_keys = Self::extract_foreign_keys(&self.pool, &table_info.name).await?;
                table_info.foreign_keys = foreign_keys;

                // Extract indexes using PRAGMA index_list
                let indexes = Self::extract_indexes(&self.pool, &table_info.name).await?;
                table_info.indexes = indexes;

                return Ok(Some(table_info));
            }
        }

        Ok(None)
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

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_sqlite_introspector_foreign_keys_and_unique_constraints() {
        use sqlx::SqlitePool;

        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create pool");

        // Create tables with foreign keys and unique constraints
        sqlx::query(
            r#"
            CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                email TEXT UNIQUE NOT NULL,
                username TEXT UNIQUE NOT NULL,
                name TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create users table");

        sqlx::query(
            r#"
            CREATE TABLE posts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                title TEXT NOT NULL,
                FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE ON UPDATE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create posts table");

        let introspector = SQLiteIntrospector::new(pool);
        let schema = introspector
            .read_schema()
            .await
            .expect("Failed to read schema");

        // Check users table
        assert!(schema.tables.contains_key("users"));
        let users_table = &schema.tables["users"];

        // Check unique constraints (email and username)
        assert!(
            !users_table.unique_constraints.is_empty(),
            "Users table should have unique constraints"
        );

        // Check posts table
        assert!(schema.tables.contains_key("posts"));
        let posts_table = &schema.tables["posts"];

        // Verify foreign key on posts table
        assert!(
            !posts_table.foreign_keys.is_empty(),
            "Posts table should have foreign key constraint"
        );

        let fk = &posts_table.foreign_keys[0];
        assert_eq!(
            fk.referenced_table, "users",
            "Foreign key should reference users table"
        );
        assert_eq!(
            fk.columns,
            vec!["user_id"],
            "Foreign key should be on user_id column"
        );
        assert_eq!(
            fk.referenced_columns,
            vec!["id"],
            "Foreign key should reference id column"
        );
        assert_eq!(
            fk.on_delete,
            Some("CASCADE".to_string()),
            "Foreign key should have CASCADE on delete"
        );
        assert_eq!(
            fk.on_update,
            Some("CASCADE".to_string()),
            "Foreign key should have CASCADE on update"
        );
    }
}
