//! # Many-to-Many Relationship Support
//!
//! SQLAlchemy-inspired many-to-many relationship implementation.
//!
//! This module is inspired by SQLAlchemy's relationship patterns
//! Copyright 2005-2025 SQLAlchemy authors and contributors
//! Licensed under MIT License. See THIRD-PARTY-NOTICES for details.

use crate::Model;
use sea_query::{
	Alias, ColumnDef, ColumnType, DeleteStatement, Expr, ExprTrait, InsertStatement, IntoIden,
	Query, SelectStatement, Table,
};
use std::marker::PhantomData;

/// Association table definition for many-to-many relationships
#[derive(Debug, Clone)]
pub struct AssociationTable {
	/// Table name
	pub table_name: String,

	/// Left side foreign key column
	pub left_column: String,

	/// Right side foreign key column
	pub right_column: String,

	/// Additional columns in the association table
	pub extra_columns: Vec<(String, String)>, // (column_name, column_type)
}

impl AssociationTable {
	/// Create a new association table definition
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::many_to_many::AssociationTable;
	/// use sea_query::PostgresQueryBuilder;
	///
	/// let table = AssociationTable::new("student_courses", "student_id", "course_id");
	/// let sql = table.to_create_sql(PostgresQueryBuilder);
	///
	/// assert!(sql.contains("CREATE TABLE"));
	/// assert!(sql.contains("student_courses"));
	/// assert!(sql.contains("student_id"));
	/// assert!(sql.contains("course_id"));
	/// ```
	pub fn new(
		table_name: impl Into<String>,
		left_column: impl Into<String>,
		right_column: impl Into<String>,
	) -> Self {
		Self {
			table_name: table_name.into(),
			left_column: left_column.into(),
			right_column: right_column.into(),
			extra_columns: Vec::new(),
		}
	}

	/// Parse type string to ColumnType enum
	///
	/// Converts common SQL type strings to sea-query ColumnType enum values.
	/// Falls back to Custom type for unrecognized types.
	fn parse_column_type(type_str: &str) -> ColumnType {
		use sea_query::StringLen;

		match type_str.to_lowercase().as_str() {
			"integer" | "int" => ColumnType::Integer,
			"bigint" | "biginteger" => ColumnType::BigInteger,
			"smallint" | "smallinteger" => ColumnType::SmallInteger,
			"tinyint" | "tinyinteger" => ColumnType::TinyInteger,
			"string" | "varchar" => ColumnType::String(StringLen::N(255)), // Default varchar length
			"text" => ColumnType::Text,
			"boolean" | "bool" => ColumnType::Boolean,
			"float" | "real" => ColumnType::Float,
			"double" => ColumnType::Double,
			"decimal" => ColumnType::Decimal(None),
			"date" => ColumnType::Date,
			"time" => ColumnType::Time,
			"datetime" | "timestamp" => ColumnType::DateTime,
			"timestamptz" | "timestamp with time zone" => ColumnType::TimestampWithTimeZone,
			"json" => ColumnType::Json,
			"jsonb" => ColumnType::JsonBinary,
			"uuid" => ColumnType::Uuid,
			"binary" | "blob" => ColumnType::Binary(255), // Default binary length
			"varbinary" => ColumnType::VarBinary(StringLen::N(255)), // Default varbinary length
			"char" => ColumnType::Char(Some(1)),          // Default char length
			_ => {
				// For VARCHAR(N), CHAR(N), DECIMAL(P,S) patterns, use custom
				ColumnType::Custom(Alias::new(type_str).into_iden())
			}
		}
	}
	/// Add extra column to the association table
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::many_to_many::AssociationTable;
	/// use sea_query::PostgresQueryBuilder;
	///
	/// let table = AssociationTable::new("student_courses", "student_id", "course_id")
	///     .with_column("enrolled_at", "TIMESTAMP")
	///     .with_column("grade", "VARCHAR(2)");
	///
	/// let sql = table.to_create_sql(PostgresQueryBuilder);
	/// assert!(sql.contains("enrolled_at"));
	/// assert!(sql.contains("grade"));
	/// ```
	pub fn with_column(mut self, name: impl Into<String>, type_: impl Into<String>) -> Self {
		self.extra_columns.push((name.into(), type_.into()));
		self
	}
	/// Apply ColumnType to ColumnDef
	///
	/// Helper function to apply the parsed column type to a ColumnDef.
	fn apply_column_type(mut col_def: ColumnDef, column_type: ColumnType) -> ColumnDef {
		use sea_query::StringLen;

		match column_type {
			ColumnType::Integer => {
				col_def.integer();
			}
			ColumnType::BigInteger => {
				col_def.big_integer();
			}
			ColumnType::SmallInteger => {
				col_def.small_integer();
			}
			ColumnType::TinyInteger => {
				col_def.tiny_integer();
			}
			ColumnType::String(StringLen::N(len)) => {
				col_def.string_len(len);
			}
			ColumnType::String(_) => {
				col_def.string();
			}
			ColumnType::Text => {
				col_def.text();
			}
			ColumnType::Boolean => {
				col_def.boolean();
			}
			ColumnType::Float => {
				col_def.float();
			}
			ColumnType::Double => {
				col_def.double();
			}
			ColumnType::Decimal(_) => {
				col_def.decimal();
			}
			ColumnType::Date => {
				col_def.date();
			}
			ColumnType::Time => {
				col_def.time();
			}
			ColumnType::DateTime => {
				col_def.date_time();
			}
			ColumnType::TimestampWithTimeZone => {
				col_def.timestamp_with_time_zone();
			}
			ColumnType::Json => {
				col_def.json();
			}
			ColumnType::JsonBinary => {
				col_def.json_binary();
			}
			ColumnType::Uuid => {
				col_def.uuid();
			}
			ColumnType::Binary(len) => {
				col_def.binary_len(len);
			}
			ColumnType::VarBinary(StringLen::N(len)) => {
				col_def.var_binary(len);
			}
			ColumnType::VarBinary(_) => {
				col_def.var_binary(255);
			} // Fallback
			ColumnType::Char(Some(len)) => {
				col_def.char_len(len);
			}
			ColumnType::Char(None) => {
				col_def.char_len(1);
			} // Fallback
			ColumnType::Custom(iden) => {
				col_def.custom(iden);
			}
			_ => {
				col_def.text();
			} // Fallback to TEXT for unknown types
		}
		col_def
	}

	/// Generate SeaQuery CREATE TABLE statement for the association table
	///
	/// Returns a TableCreateStatement that can be converted to SQL.
	pub fn to_create_statement(&self) -> sea_query::TableCreateStatement {
		let mut stmt = Table::create();
		stmt.table(Alias::new(&self.table_name))
			.if_not_exists()
			.col(
				ColumnDef::new(Alias::new(&self.left_column))
					.integer()
					.not_null(),
			)
			.col(
				ColumnDef::new(Alias::new(&self.right_column))
					.integer()
					.not_null(),
			);

		// Add extra columns with parsed types
		for (name, type_str) in &self.extra_columns {
			let column_type = Self::parse_column_type(type_str);
			let col_def = ColumnDef::new(Alias::new(name));
			let col_def = Self::apply_column_type(col_def, column_type);
			stmt.col(col_def);
		}

		// Add composite primary key
		stmt.primary_key(
			sea_query::Index::create()
				.col(Alias::new(&self.left_column))
				.col(Alias::new(&self.right_column)),
		);

		stmt.to_owned()
	}

	/// Generate CREATE TABLE SQL for the association table (convenience method)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::many_to_many::AssociationTable;
	/// use sea_query::SqliteQueryBuilder;
	///
	/// let table = AssociationTable::new("user_roles", "user_id", "role_id");
	/// let sql = table.to_create_sql(SqliteQueryBuilder);
	///
	/// assert!(sql.contains("CREATE TABLE"));
	/// assert!(sql.contains("user_roles"));
	/// ```
	pub fn to_create_sql<T: sea_query::SchemaBuilder>(&self, builder: T) -> String {
		self.to_create_statement().to_string(builder)
	}
}

/// Many-to-many relationship between two models
pub struct ManyToMany<L: Model, R: Model> {
	/// Association table definition
	association_table: AssociationTable,

	/// Loading strategy (lazy, eager, etc.)
	lazy: bool,

	/// Back reference name
	back_populates: Option<String>,

	/// Cascade options
	cascade: Vec<String>,

	_phantom_left: PhantomData<L>,
	_phantom_right: PhantomData<R>,
}

impl<L: Model, R: Model> ManyToMany<L, R> {
	/// Create a new many-to-many relationship
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_orm::many_to_many::{AssociationTable, ManyToMany};
	/// use reinhardt_orm::Model;
	/// use sea_query::PostgresQueryBuilder;
	/// use serde::{Serialize, Deserialize};
	///
	/// #[derive(Debug, Clone, Serialize, Deserialize)]
	/// struct Student { id: Option<i64>, name: String }
	///
	/// #[derive(Debug, Clone, Serialize, Deserialize)]
	/// struct Course { id: Option<i64>, title: String }
	///
	/// # #[derive(Clone)]
	/// # struct StudentFields;
	/// # impl reinhardt_orm::FieldSelector for StudentFields {
	/// #     fn with_alias(self, _alias: &str) -> Self { self }
	/// # }
	/// #
	/// # #[derive(Clone)]
	/// # struct CourseFields;
	/// # impl reinhardt_orm::FieldSelector for CourseFields {
	/// #     fn with_alias(self, _alias: &str) -> Self { self }
	/// # }
	/// #
	/// impl Model for Student {
	///     type PrimaryKey = i64;
	/// #     type Fields = StudentFields;
	///     fn table_name() -> &'static str { "students" }
	/// #     fn new_fields() -> Self::Fields { StudentFields }
	///     fn primary_key(&self) -> Option<Self::PrimaryKey> { self.id }
	///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// }
	///
	/// impl Model for Course {
	///     type PrimaryKey = i64;
	/// #     type Fields = CourseFields;
	///     fn table_name() -> &'static str { "courses" }
	/// #     fn new_fields() -> Self::Fields { CourseFields }
	///     fn primary_key(&self) -> Option<Self::PrimaryKey> { self.id }
	///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// }
	///
	/// let assoc = AssociationTable::new("student_courses", "student_id", "course_id");
	/// let m2m = ManyToMany::<Student, Course>::new(assoc);
	/// let join_sql = m2m.join_sql(PostgresQueryBuilder);
	///
	/// assert!(join_sql.contains("student_courses"));
	/// assert!(join_sql.contains("students"));
	/// assert!(join_sql.contains("courses"));
	/// ```
	pub fn new(association_table: AssociationTable) -> Self {
		Self {
			association_table,
			lazy: true,
			back_populates: None,
			cascade: Vec::new(),
			_phantom_left: PhantomData,
			_phantom_right: PhantomData,
		}
	}
	/// Set eager loading
	///
	pub fn eager(mut self) -> Self {
		self.lazy = false;
		self
	}
	/// Set back reference
	///
	pub fn back_populates(mut self, name: impl Into<String>) -> Self {
		self.back_populates = Some(name.into());
		self
	}
	/// Add cascade option
	///
	pub fn cascade(mut self, option: impl Into<String>) -> Self {
		self.cascade.push(option.into());
		self
	}
	/// Generate SeaQuery SELECT statement for joining through the association table
	///
	/// Returns a SelectStatement with JOINs through the association table.
	pub fn join_query(&self) -> SelectStatement {
		let left_table = L::table_name();
		let right_table = R::table_name();
		let assoc_table = &self.association_table.table_name;

		Query::select()
			.from(Alias::new(left_table))
			.column((Alias::new(left_table), sea_query::Asterisk))
			.column((Alias::new(right_table), sea_query::Asterisk))
			.inner_join(
				Alias::new(assoc_table),
				Expr::col((Alias::new(left_table), Alias::new("id"))).equals((
					Alias::new(assoc_table),
					Alias::new(&self.association_table.left_column),
				)),
			)
			.inner_join(
				Alias::new(right_table),
				Expr::col((
					Alias::new(assoc_table),
					Alias::new(&self.association_table.right_column),
				))
				.equals((Alias::new(right_table), Alias::new("id"))),
			)
			.to_owned()
	}

	/// Generate SQL for joining (convenience method)
	///
	pub fn join_sql<T: sea_query::QueryBuilder>(&self, builder: T) -> String {
		self.join_query().to_string(builder)
	}

	/// Generate SeaQuery INSERT statement for adding a relationship
	///
	pub fn add_query(&self, left_id: i64, right_id: i64) -> InsertStatement {
		Query::insert()
			.into_table(Alias::new(&self.association_table.table_name))
			.columns([
				Alias::new(&self.association_table.left_column),
				Alias::new(&self.association_table.right_column),
			])
			.values_panic([Expr::val(left_id), Expr::val(right_id)])
			.to_owned()
	}

	/// Generate SQL for adding a relationship (convenience method)
	///
	pub fn add_sql<T: sea_query::QueryBuilder>(
		&self,
		left_id: i64,
		right_id: i64,
		builder: T,
	) -> String {
		self.add_query(left_id, right_id).to_string(builder)
	}

	/// Generate SeaQuery DELETE statement for removing a relationship
	///
	pub fn remove_query(&self, left_id: i64, right_id: i64) -> DeleteStatement {
		Query::delete()
			.from_table(Alias::new(&self.association_table.table_name))
			.and_where(Expr::col(Alias::new(&self.association_table.left_column)).eq(left_id))
			.and_where(Expr::col(Alias::new(&self.association_table.right_column)).eq(right_id))
			.to_owned()
	}

	/// Generate SQL for removing a relationship (convenience method)
	///
	pub fn remove_sql<T: sea_query::QueryBuilder>(
		&self,
		left_id: i64,
		right_id: i64,
		builder: T,
	) -> String {
		self.remove_query(left_id, right_id).to_string(builder)
	}
	/// Get association table reference
	///
	pub fn table(&self) -> &AssociationTable {
		&self.association_table
	}
}
/// Helper function to create an association table
///
pub fn association_table(
	table_name: impl Into<String>,
	left_column: impl Into<String>,
	right_column: impl Into<String>,
) -> AssociationTable {
	AssociationTable::new(table_name, left_column, right_column)
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_core::validators::TableName;
	use serde::{Deserialize, Serialize};

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct Student {
		id: Option<i64>,
		name: String,
	}

	#[derive(Clone)]
	struct StudentFields;
	impl crate::model::FieldSelector for StudentFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	const STUDENT_TABLE: TableName = TableName::new_const("students");

	impl Model for Student {
		type PrimaryKey = i64;
		type Fields = StudentFields;

		fn table_name() -> &'static str {
			STUDENT_TABLE.as_str()
		}

		fn new_fields() -> Self::Fields {
			StudentFields
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	struct Course {
		id: Option<i64>,
		title: String,
	}

	#[derive(Clone)]
	struct CourseFields;
	impl crate::model::FieldSelector for CourseFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	const COURSE_TABLE: TableName = TableName::new_const("courses");

	impl Model for Course {
		type PrimaryKey = i64;
		type Fields = CourseFields;

		fn table_name() -> &'static str {
			COURSE_TABLE.as_str()
		}

		fn new_fields() -> Self::Fields {
			CourseFields
		}

		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			self.id
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}
	}

	#[test]
	fn test_association_table() {
		use sea_query::SqliteQueryBuilder;

		let table = AssociationTable::new("student_courses", "student_id", "course_id");
		let sql = table.to_create_sql(SqliteQueryBuilder);

		assert_eq!(
			sql,
			"CREATE TABLE IF NOT EXISTS \"student_courses\" ( \"student_id\" integer NOT NULL, \"course_id\" integer NOT NULL, PRIMARY KEY (\"student_id\", \"course_id\") )",
			"Expected exact CREATE TABLE SQL, got: {}",
			sql
		);
	}

	#[test]
	fn test_association_table_with_extra_columns() {
		use sea_query::SqliteQueryBuilder;

		let table = AssociationTable::new("student_courses", "student_id", "course_id")
			.with_column("enrolled_at", "TIMESTAMP")
			.with_column("grade", "VARCHAR(2)");

		let sql = table.to_create_sql(SqliteQueryBuilder);
		// SQLite uses datetime_text for TIMESTAMP/DATETIME columns
		assert_eq!(
			sql,
			"CREATE TABLE IF NOT EXISTS \"student_courses\" ( \"student_id\" integer NOT NULL, \"course_id\" integer NOT NULL, \"enrolled_at\" datetime_text, \"grade\" VARCHAR(2), PRIMARY KEY (\"student_id\", \"course_id\") )",
			"Expected exact CREATE TABLE with extra columns SQL, got: {}",
			sql
		);
	}

	#[test]
	fn test_many_to_many_join() {
		use sea_query::SqliteQueryBuilder;

		let assoc = AssociationTable::new("student_courses", "student_id", "course_id");
		let m2m = ManyToMany::<Student, Course>::new(assoc);

		let join_sql = m2m.join_sql(SqliteQueryBuilder);
		assert_eq!(
			join_sql,
			"SELECT \"students\".*, \"courses\".* FROM \"students\" INNER JOIN \"student_courses\" ON \"students\".\"id\" = \"student_courses\".\"student_id\" INNER JOIN \"courses\" ON \"student_courses\".\"course_id\" = \"courses\".\"id\"",
			"Expected exact JOIN SQL, got: {}",
			join_sql
		);
	}

	#[test]
	fn test_many_to_many_add() {
		use sea_query::SqliteQueryBuilder;

		let assoc = AssociationTable::new("student_courses", "student_id", "course_id");
		let m2m = ManyToMany::<Student, Course>::new(assoc);

		let sql = m2m.add_sql(1, 10, SqliteQueryBuilder);
		assert_eq!(
			sql, "INSERT INTO \"student_courses\" (\"student_id\", \"course_id\") VALUES (1, 10)",
			"Expected exact INSERT SQL, got: {}",
			sql
		);
	}

	#[test]
	fn test_many_to_many_remove() {
		use sea_query::SqliteQueryBuilder;

		let assoc = AssociationTable::new("student_courses", "student_id", "course_id");
		let m2m = ManyToMany::<Student, Course>::new(assoc);

		let sql = m2m.remove_sql(1, 10, SqliteQueryBuilder);
		assert_eq!(
			sql, "DELETE FROM \"student_courses\" WHERE \"student_id\" = 1 AND \"course_id\" = 10",
			"Expected exact DELETE SQL, got: {}",
			sql
		);
	}

	#[test]
	fn test_parse_column_type_integer() {
		let col_type = AssociationTable::parse_column_type("INTEGER");
		assert!(matches!(col_type, ColumnType::Integer));

		let col_type = AssociationTable::parse_column_type("int");
		assert!(matches!(col_type, ColumnType::Integer));
	}

	#[test]
	fn test_parse_column_type_bigint() {
		let col_type = AssociationTable::parse_column_type("BIGINT");
		assert!(matches!(col_type, ColumnType::BigInteger));
	}

	#[test]
	fn test_parse_column_type_string() {
		let col_type = AssociationTable::parse_column_type("VARCHAR");
		assert!(matches!(col_type, ColumnType::String(_)));

		let col_type = AssociationTable::parse_column_type("string");
		assert!(matches!(col_type, ColumnType::String(_)));
	}

	#[test]
	fn test_parse_column_type_text() {
		let col_type = AssociationTable::parse_column_type("TEXT");
		assert!(matches!(col_type, ColumnType::Text));
	}

	#[test]
	fn test_parse_column_type_boolean() {
		let col_type = AssociationTable::parse_column_type("BOOLEAN");
		assert!(matches!(col_type, ColumnType::Boolean));

		let col_type = AssociationTable::parse_column_type("bool");
		assert!(matches!(col_type, ColumnType::Boolean));
	}

	#[test]
	fn test_parse_column_type_datetime() {
		let col_type = AssociationTable::parse_column_type("DATETIME");
		assert!(matches!(col_type, ColumnType::DateTime));

		let col_type = AssociationTable::parse_column_type("timestamp");
		assert!(matches!(col_type, ColumnType::DateTime));
	}

	#[test]
	fn test_parse_column_type_json() {
		let col_type = AssociationTable::parse_column_type("JSON");
		assert!(matches!(col_type, ColumnType::Json));

		let col_type = AssociationTable::parse_column_type("JSONB");
		assert!(matches!(col_type, ColumnType::JsonBinary));
	}

	#[test]
	fn test_parse_column_type_uuid() {
		let col_type = AssociationTable::parse_column_type("UUID");
		assert!(matches!(col_type, ColumnType::Uuid));
	}

	#[test]
	fn test_parse_column_type_custom() {
		let col_type = AssociationTable::parse_column_type("VARCHAR(255)");
		assert!(matches!(col_type, ColumnType::Custom(_)));

		let col_type = AssociationTable::parse_column_type("UNKNOWN_TYPE");
		assert!(matches!(col_type, ColumnType::Custom(_)));
	}

	#[test]
	fn test_parse_column_type_case_insensitive() {
		let col_type = AssociationTable::parse_column_type("integer");
		assert!(matches!(col_type, ColumnType::Integer));

		let col_type = AssociationTable::parse_column_type("INTEGER");
		assert!(matches!(col_type, ColumnType::Integer));

		let col_type = AssociationTable::parse_column_type("InTeGeR");
		assert!(matches!(col_type, ColumnType::Integer));
	}

	#[test]
	fn test_association_table_with_typed_columns() {
		use sea_query::SqliteQueryBuilder;

		let table = AssociationTable::new("enrollments", "student_id", "course_id")
			.with_column("enrolled_at", "DATETIME")
			.with_column("grade", "INTEGER")
			.with_column("notes", "TEXT");

		let sql = table.to_create_sql(SqliteQueryBuilder);

		// SQLite uses datetime_text for TIMESTAMP/DATETIME columns
		assert_eq!(
			sql,
			"CREATE TABLE IF NOT EXISTS \"enrollments\" ( \"student_id\" integer NOT NULL, \"course_id\" integer NOT NULL, \"enrolled_at\" datetime_text, \"grade\" integer, \"notes\" text, PRIMARY KEY (\"student_id\", \"course_id\") )",
			"Expected exact CREATE TABLE with typed columns SQL, got: {}",
			sql
		);
	}
}
