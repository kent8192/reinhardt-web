//! Database schema introspection
//!
//! This module provides functionality to read the current database schema
//! and extract table definitions, column metadata, indexes, and constraints.

use async_trait::async_trait;
use std::collections::HashMap;

use super::{MigrationError, Result};

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
	/// CHECK constraints
	pub check_constraints: Vec<CheckConstraintInfo>,
}

/// Column metadata
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnInfo {
	/// Column name
	pub name: String,
	/// Column type
	pub column_type: super::FieldType,
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

/// CHECK constraint
#[derive(Debug, Clone, PartialEq)]
pub struct CheckConstraintInfo {
	/// Constraint name (None for anonymous CHECK constraints)
	pub name: Option<String>,
	/// CHECK expression (without the CHECK keyword and outer parentheses)
	pub expression: String,
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

	fn convert_column_type(col_type: &sea_schema::postgres::def::Type) -> super::FieldType {
		col_type.into()
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

		// PostgreSQL CHECK constraints extraction is not yet implemented.
		// This is a non-critical feature that can be added when needed.
		// Most common constraints (NOT NULL, UNIQUE, FK, PK) are already supported.
		let check_constraints: Vec<CheckConstraintInfo> = Vec::new();

		Ok(TableInfo {
			name: table_def.info.name.clone(),
			columns,
			indexes,
			primary_key,
			foreign_keys,
			unique_constraints,
			check_constraints,
		})
	}

	/// Fetch index information for a specific table from PostgreSQL system catalogs
	async fn fetch_table_indexes(&self, table_name: &str) -> Result<HashMap<String, IndexInfo>> {
		use sqlx::Row;

		// Query PostgreSQL system catalogs to get index information
		// Excludes primary key indexes as they are handled separately
		let query = r#"
			SELECT
				i.relname AS index_name,
				array_agg(a.attname ORDER BY array_position(ix.indkey, a.attnum)) AS column_names,
				ix.indisunique AS is_unique,
				am.amname AS index_type
			FROM
				pg_class t,
				pg_class i,
				pg_index ix,
				pg_attribute a,
				pg_am am,
				pg_namespace n
			WHERE
				t.oid = ix.indrelid
				AND i.oid = ix.indexrelid
				AND a.attrelid = t.oid
				AND a.attnum = ANY(ix.indkey)
				AND t.relkind = 'r'
				AND t.relname = $1
				AND i.relam = am.oid
				AND NOT ix.indisprimary
				AND n.oid = t.relnamespace
				AND n.nspname = 'public'
			GROUP BY i.relname, ix.indisunique, am.amname
			ORDER BY i.relname
		"#;

		let rows = sqlx::query(query)
			.bind(table_name)
			.fetch_all(&self.pool)
			.await
			.map_err(|e| {
				MigrationError::IntrospectionError(format!(
					"Failed to fetch indexes for table {}: {}",
					table_name, e
				))
			})?;

		let mut indexes = HashMap::new();
		for row in rows {
			let index_name: String = row.try_get("index_name").map_err(|e| {
				MigrationError::IntrospectionError(format!("Failed to get index_name: {}", e))
			})?;
			let column_names: Vec<String> = row.try_get("column_names").map_err(|e| {
				MigrationError::IntrospectionError(format!("Failed to get column_names: {}", e))
			})?;
			let is_unique: bool = row.try_get("is_unique").map_err(|e| {
				MigrationError::IntrospectionError(format!("Failed to get is_unique: {}", e))
			})?;
			let index_type: String = row.try_get("index_type").map_err(|e| {
				MigrationError::IntrospectionError(format!("Failed to get index_type: {}", e))
			})?;

			indexes.insert(
				index_name.clone(),
				IndexInfo {
					name: index_name,
					columns: column_names,
					unique: is_unique,
					index_type: Some(index_type),
				},
			);
		}

		Ok(indexes)
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
			let mut table_info = Self::convert_table_def(&table_def)?;

			// Fetch index information separately for each table
			table_info.indexes = self.fetch_table_indexes(&table_info.name).await?;

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

	fn convert_column_type(col_type: &sea_schema::mysql::def::Type) -> super::FieldType {
		col_type.into()
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
			let columns: Vec<String> = index_def.parts.iter().map(|p| p.column.clone()).collect();

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

		// MySQL CHECK constraints extraction is not yet implemented.
		// This is a non-critical feature that can be added when needed.
		// Most common constraints (NOT NULL, UNIQUE, FK, PK) are already supported.
		let check_constraints: Vec<CheckConstraintInfo> = Vec::new();

		Ok(TableInfo {
			name: table_def.info.name.clone(),
			columns,
			indexes,
			primary_key,
			foreign_keys,
			unique_constraints,
			check_constraints,
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

	fn convert_column_type(col_type: &sea_schema::sea_query::ColumnType) -> super::FieldType {
		col_type.into()
	}

	// Future implementation: Will be used for FK constraint validation in migrations
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
	///
	/// This method also extracts actual constraint names from CREATE TABLE SQL
	/// when available (for named FK constraints like `CONSTRAINT fk_name FOREIGN KEY...`).
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
			// SQLite PRAGMA field: MATCH clause parsed but not currently used in FK info
			#[allow(dead_code)]
			r#match: String,
		}

		// Get CREATE TABLE SQL to extract actual constraint names
		let create_sql = Self::get_create_table_sql(pool, table_name).await?;
		let named_fks = create_sql
			.as_ref()
			.map(|sql| Self::parse_fk_constraint_names(sql))
			.unwrap_or_default();

		let query = format!("PRAGMA foreign_key_list({})", table_name);
		let rows: Vec<ForeignKeyRow> = sqlx::query_as(&query)
			.fetch_all(pool)
			.await
			.map_err(|e| MigrationError::IntrospectionError(e.to_string()))?;

		// Group by FK ID to handle multi-column foreign keys
		let mut fk_map: HashMap<i64, Vec<ForeignKeyRow>> = HashMap::new();
		for row in rows {
			fk_map.entry(row.id).or_default().push(row);
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

			// Try to find actual constraint name from CREATE TABLE SQL
			// If not found, fall back to generated name
			let signature = (columns.clone(), referenced_table.clone());
			let name = named_fks
				.get(&signature)
				.cloned()
				.unwrap_or_else(|| format!("fk_{}_{}", table_name, fk_id));

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
			// SQLite PRAGMA field: Sequence number in index list, not used in IndexInfo
			#[allow(dead_code)]
			seq: i64,
			name: String,
			unique: i64,
			// SQLite PRAGMA field: Index creation origin (c=CREATE INDEX, u=UNIQUE, pk=PRIMARY KEY)
			#[allow(dead_code)]
			origin: String,
			// SQLite PRAGMA field: Whether index is partial (WHERE clause), not currently used
			#[allow(dead_code)]
			partial: i64,
		}

		#[derive(sqlx::FromRow)]
		struct IndexInfoRow {
			// SQLite PRAGMA field: Column sequence in index, not used in IndexInfo
			#[allow(dead_code)]
			seqno: i64,
			// SQLite PRAGMA field: Column ID in table, not used in IndexInfo
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
			let index_info: Vec<IndexInfoRow> =
				sqlx::query_as(&info_query)
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

	/// Gets the CREATE TABLE SQL statement from sqlite_master.
	async fn get_create_table_sql(
		pool: &sqlx::SqlitePool,
		table_name: &str,
	) -> Result<Option<String>> {
		#[derive(sqlx::FromRow)]
		struct SqlRow {
			sql: Option<String>,
		}

		let query = "SELECT sql FROM sqlite_master WHERE type='table' AND name=?";
		let result: Option<SqlRow> = sqlx::query_as(query)
			.bind(table_name)
			.fetch_optional(pool)
			.await
			.map_err(|e| MigrationError::IntrospectionError(e.to_string()))?;

		Ok(result.and_then(|r| r.sql))
	}

	/// Extracts CHECK constraints from the CREATE TABLE SQL statement.
	///
	/// SQLite doesn't have a PRAGMA for CHECK constraints, so we parse them
	/// from the CREATE TABLE statement stored in sqlite_master.
	///
	/// Handles both:
	/// - Named CHECK: `CONSTRAINT check_name CHECK(expression)`
	/// - Anonymous CHECK: `CHECK(expression)`
	async fn extract_check_constraints(
		pool: &sqlx::SqlitePool,
		table_name: &str,
	) -> Result<Vec<CheckConstraintInfo>> {
		let create_sql = match Self::get_create_table_sql(pool, table_name).await? {
			Some(sql) => sql,
			None => return Ok(Vec::new()),
		};

		Self::parse_check_constraints(&create_sql)
	}

	/// Parses CHECK constraints from a CREATE TABLE SQL statement.
	fn parse_check_constraints(create_sql: &str) -> Result<Vec<CheckConstraintInfo>> {
		let mut constraints = Vec::new();

		// Named CHECK constraint pattern: CONSTRAINT name CHECK(...)
		// We need to handle nested parentheses in the expression
		let named_pattern = regex::Regex::new(r#"(?i)CONSTRAINT\s+["'`]?(\w+)["'`]?\s+CHECK\s*\("#)
			.map_err(|e| MigrationError::IntrospectionError(e.to_string()))?;

		// Anonymous CHECK pattern: CHECK(...) not preceded by CONSTRAINT
		let anon_pattern = regex::Regex::new(r#"(?i)CHECK\s*\("#)
			.map_err(|e| MigrationError::IntrospectionError(e.to_string()))?;

		// Pattern to check for CONSTRAINT name before CHECK
		let constraint_pattern =
			regex::Regex::new(r#"(?i)CONSTRAINT\s+["'`]?\w+["'`]?\s*$"#).unwrap();

		// Find named CHECK constraints
		for cap in named_pattern.captures_iter(create_sql) {
			let name = cap.get(1).map(|m| m.as_str().to_string());
			let match_end = cap.get(0).unwrap().end();

			// Extract expression by counting parentheses
			if let Some(expr) = Self::extract_parenthesized_expression(create_sql, match_end - 1) {
				constraints.push(CheckConstraintInfo {
					name,
					expression: expr,
				});
			}
		}

		// Find anonymous CHECK constraints
		// We need to exclude the named ones we already found
		for m in anon_pattern.find_iter(create_sql) {
			let start = m.start();

			// Check if this is preceded by CONSTRAINT (skip if so)
			let before = &create_sql[..start];
			if before.to_uppercase().trim_end().ends_with("CONSTRAINT") {
				continue;
			}

			// Also check for CONSTRAINT name pattern before CHECK
			if constraint_pattern.is_match(before.trim_end()) {
				continue;
			}

			let match_end = m.end();
			if let Some(expr) = Self::extract_parenthesized_expression(create_sql, match_end - 1) {
				constraints.push(CheckConstraintInfo {
					name: None,
					expression: expr,
				});
			}
		}

		Ok(constraints)
	}

	/// Extracts the content inside parentheses, handling nested parentheses.
	/// `start_pos` should be the position of the opening parenthesis.
	fn extract_parenthesized_expression(sql: &str, start_pos: usize) -> Option<String> {
		let chars: Vec<char> = sql.chars().collect();
		if start_pos >= chars.len() || chars[start_pos] != '(' {
			return None;
		}

		let mut depth = 0;
		let expr_start = start_pos + 1;
		let mut expr_end = start_pos + 1;

		for (i, &c) in chars.iter().enumerate().skip(start_pos) {
			match c {
				'(' => depth += 1,
				')' => {
					depth -= 1;
					if depth == 0 {
						expr_end = i;
						break;
					}
				}
				_ => {}
			}
		}

		if depth == 0 && expr_end > expr_start {
			let expr: String = chars[expr_start..expr_end].iter().collect();
			Some(expr.trim().to_string())
		} else {
			None
		}
	}

	/// Parses FK constraint names from a CREATE TABLE SQL statement.
	///
	/// Returns a HashMap where:
	/// - Key: (source_columns, referenced_table) as a signature
	/// - Value: constraint name
	///
	/// This is used to match PRAGMA foreign_key_list results with actual constraint names.
	fn parse_fk_constraint_names(create_sql: &str) -> HashMap<(Vec<String>, String), String> {
		let mut result = HashMap::new();

		// Pattern: CONSTRAINT name FOREIGN KEY (cols) REFERENCES table(cols)
		let fk_pattern = regex::Regex::new(
			r#"(?i)CONSTRAINT\s+["'`]?(\w+)["'`]?\s+FOREIGN\s+KEY\s*\(([^)]+)\)\s*REFERENCES\s+["'`]?(\w+)["'`]?"#,
		);

		if let Ok(re) = fk_pattern {
			for cap in re.captures_iter(create_sql) {
				if let (Some(name), Some(cols), Some(ref_table)) =
					(cap.get(1), cap.get(2), cap.get(3))
				{
					let constraint_name = name.as_str().to_string();
					let columns: Vec<String> = cols
						.as_str()
						.split(',')
						.map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
						.collect();
					let referenced_table = ref_table.as_str().to_string();

					result.insert((columns, referenced_table), constraint_name);
				}
			}
		}

		result
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
						sea_schema::sqlite::def::DefaultType::CurrentTimestamp => {
							Some("CURRENT_TIMESTAMP".to_string())
						}
						sea_schema::sqlite::def::DefaultType::Null
						| sea_schema::sqlite::def::DefaultType::Unspecified => None,
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

		// CHECK constraints will be extracted separately using sqlite_master
		// in the read_schema method
		let check_constraints: Vec<CheckConstraintInfo> = Vec::new();

		Ok(TableInfo {
			name: table_def.name.clone(),
			columns,
			indexes,
			primary_key,
			foreign_keys,
			unique_constraints,
			check_constraints,
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

			// Extract CHECK constraints from sqlite_master
			let check_constraints =
				Self::extract_check_constraints(&self.pool, &table_info.name).await?;
			table_info.check_constraints = check_constraints;

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

				// Extract CHECK constraints from sqlite_master
				let check_constraints =
					Self::extract_check_constraints(&self.pool, &table_info.name).await?;
				table_info.check_constraints = check_constraints;

				return Ok(Some(table_info));
			}
		}

		Ok(None)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::migrations::FieldType;

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
		assert_eq!(id_col.column_type, FieldType::Integer);
		assert!(id_col.auto_increment);
		assert!(!id_col.nullable);

		// Check name column
		let name_col = &users_table.columns["name"];
		assert_eq!(name_col.name, "name");
		assert_eq!(name_col.column_type, FieldType::Text);
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
