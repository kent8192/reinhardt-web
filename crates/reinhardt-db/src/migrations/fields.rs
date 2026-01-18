//! Field type definitions for migrations

use serde::{Deserialize, Serialize};

/// Represents database field types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FieldType {
	// Integer types
	BigInteger,
	Integer,
	SmallInteger,
	TinyInt,   // MySQL-specific
	MediumInt, // MySQL-specific

	// String types (with parameters)
	Char(u32),
	VarChar(u32),
	Text,
	TinyText,   // MySQL-specific
	MediumText, // MySQL-specific
	LongText,   // MySQL-specific

	// Date/time types
	Date,
	Time,
	DateTime,
	TimestampTz, // PostgreSQL TIMESTAMPTZ

	// Numeric types
	Decimal {
		precision: u32,
		scale: u32,
	},
	Float,
	Double,
	Real,

	// Boolean type
	Boolean,

	// Binary types
	Binary,
	Blob,       // MySQL-specific
	TinyBlob,   // MySQL-specific
	MediumBlob, // MySQL-specific
	LongBlob,   // MySQL-specific
	Bytea,      // PostgreSQL-specific

	// JSON types
	Json,
	JsonBinary, // PostgreSQL JSONB

	// PostgreSQL-specific types
	/// PostgreSQL Array type with inner element type
	Array(Box<FieldType>),
	/// PostgreSQL HStore key-value store
	HStore,
	/// PostgreSQL case-insensitive text
	CIText,
	/// PostgreSQL int4range (integer range)
	Int4Range,
	/// PostgreSQL int8range (bigint range)
	Int8Range,
	/// PostgreSQL numrange (numeric range)
	NumRange,
	/// PostgreSQL daterange
	DateRange,
	/// PostgreSQL tsrange (timestamp range without timezone)
	TsRange,
	/// PostgreSQL tstzrange (timestamp range with timezone)
	TsTzRange,
	/// PostgreSQL tsvector for full-text search
	TsVector,
	/// PostgreSQL tsquery for full-text search queries
	TsQuery,

	// Other types
	Uuid,
	Year, // MySQL-specific

	// MySQL-specific collection types
	Enum {
		values: Vec<String>,
	},
	Set {
		values: Vec<String>,
	},

	// Relationship field types
	/// ForeignKey relationship field
	ForeignKey {
		to_table: String,
		to_field: String,
		on_delete: super::ForeignKeyAction,
	},

	/// OneToOne relationship field
	OneToOne {
		to: String, // "app.Model" format
		on_delete: super::ForeignKeyAction,
		on_update: super::ForeignKeyAction,
	},

	/// ManyToMany relationship field
	ManyToMany {
		to: String,              // "app.Model" format
		through: Option<String>, // Intermediate table name (None for auto-generation)
	},

	// Custom types
	Custom(String),
}

impl FieldType {
	/// Convert FieldType to SQL string for a specific dialect
	///
	/// This method returns database-specific SQL types.
	/// Use this method when generating SQL for a specific database.
	pub fn to_sql_for_dialect(&self, dialect: &super::operations::SqlDialect) -> String {
		use super::operations::SqlDialect;

		match self {
			FieldType::DateTime => match dialect {
				SqlDialect::Postgres | SqlDialect::Cockroachdb => "TIMESTAMP".to_string(),
				SqlDialect::Mysql | SqlDialect::Sqlite => "DATETIME".to_string(),
			},
			FieldType::TimestampTz => match dialect {
				SqlDialect::Postgres | SqlDialect::Cockroachdb => "TIMESTAMPTZ".to_string(),
				SqlDialect::Mysql => "DATETIME".to_string(), // MySQL doesn't have TIMESTAMPTZ
				SqlDialect::Sqlite => "DATETIME".to_string(), // SQLite doesn't have TIMESTAMPTZ
			},
			// Use "BOOLEAN" for SQLite instead of "INTEGER" to ensure sqlx's
			// type_info().name() returns "BOOLEAN". This allows our convert_row
			// function to properly detect boolean columns and convert integer 0/1
			// values to bool. SQLite will still store values as integers due to
			// type affinity, but the declared type name will be "BOOLEAN".
			FieldType::Boolean => match dialect {
				SqlDialect::Postgres | SqlDialect::Cockroachdb => "BOOLEAN".to_string(),
				SqlDialect::Mysql => "TINYINT(1)".to_string(), // MySQL uses TINYINT for boolean
				SqlDialect::Sqlite => "BOOLEAN".to_string(),   // SQLite - use BOOLEAN for type detection
			},
			FieldType::Uuid => match dialect {
				SqlDialect::Postgres | SqlDialect::Cockroachdb => "UUID".to_string(),
				SqlDialect::Mysql => "CHAR(36)".to_string(), // MySQL doesn't have native UUID
				SqlDialect::Sqlite => "TEXT".to_string(),    // SQLite doesn't have native UUID
			},
			FieldType::JsonBinary => match dialect {
				SqlDialect::Postgres | SqlDialect::Cockroachdb => "JSONB".to_string(),
				SqlDialect::Mysql | SqlDialect::Sqlite => "JSON".to_string(), // Fallback to JSON
			},
			// PostgreSQL-specific types with dialect handling
			FieldType::Array(inner) => match dialect {
				SqlDialect::Postgres | SqlDialect::Cockroachdb => {
					format!("{}[]", inner.to_sql_for_dialect(dialect))
				}
				// MySQL and SQLite don't support native arrays, fallback to JSON
				SqlDialect::Mysql | SqlDialect::Sqlite => "JSON".to_string(),
			},
			FieldType::HStore => match dialect {
				SqlDialect::Postgres | SqlDialect::Cockroachdb => "HSTORE".to_string(),
				// Fallback to JSON for other databases
				SqlDialect::Mysql | SqlDialect::Sqlite => "JSON".to_string(),
			},
			FieldType::CIText => match dialect {
				SqlDialect::Postgres | SqlDialect::Cockroachdb => "CITEXT".to_string(),
				// Fallback to TEXT for other databases
				SqlDialect::Mysql | SqlDialect::Sqlite => "TEXT".to_string(),
			},
			FieldType::Int4Range => match dialect {
				SqlDialect::Postgres | SqlDialect::Cockroachdb => "INT4RANGE".to_string(),
				// No native range support in other databases
				SqlDialect::Mysql | SqlDialect::Sqlite => "VARCHAR(50)".to_string(),
			},
			FieldType::Int8Range => match dialect {
				SqlDialect::Postgres | SqlDialect::Cockroachdb => "INT8RANGE".to_string(),
				SqlDialect::Mysql | SqlDialect::Sqlite => "VARCHAR(50)".to_string(),
			},
			FieldType::NumRange => match dialect {
				SqlDialect::Postgres | SqlDialect::Cockroachdb => "NUMRANGE".to_string(),
				SqlDialect::Mysql | SqlDialect::Sqlite => "VARCHAR(100)".to_string(),
			},
			FieldType::DateRange => match dialect {
				SqlDialect::Postgres | SqlDialect::Cockroachdb => "DATERANGE".to_string(),
				SqlDialect::Mysql | SqlDialect::Sqlite => "VARCHAR(50)".to_string(),
			},
			FieldType::TsRange => match dialect {
				SqlDialect::Postgres | SqlDialect::Cockroachdb => "TSRANGE".to_string(),
				SqlDialect::Mysql | SqlDialect::Sqlite => "VARCHAR(100)".to_string(),
			},
			FieldType::TsTzRange => match dialect {
				SqlDialect::Postgres | SqlDialect::Cockroachdb => "TSTZRANGE".to_string(),
				SqlDialect::Mysql | SqlDialect::Sqlite => "VARCHAR(100)".to_string(),
			},
			FieldType::TsVector => match dialect {
				SqlDialect::Postgres | SqlDialect::Cockroachdb => "TSVECTOR".to_string(),
				// No native full-text search vector in other databases
				SqlDialect::Mysql | SqlDialect::Sqlite => "TEXT".to_string(),
			},
			FieldType::TsQuery => match dialect {
				SqlDialect::Postgres | SqlDialect::Cockroachdb => "TSQUERY".to_string(),
				SqlDialect::Mysql | SqlDialect::Sqlite => "TEXT".to_string(),
			},
			// For all other types, use the generic SQL type
			_ => self.to_sql_string(),
		}
	}

	/// Convert FieldType to SQL string
	///
	/// This method returns generic SQL types that may not be compatible with all databases.
	/// For database-specific SQL generation, use `to_sql_for_dialect()` instead.
	pub fn to_sql_string(&self) -> String {
		match self {
			FieldType::BigInteger => "BIGINT".to_string(),
			FieldType::Integer => "INTEGER".to_string(),
			FieldType::SmallInteger => "SMALLINT".to_string(),
			FieldType::TinyInt => "TINYINT".to_string(),
			FieldType::MediumInt => "MEDIUMINT".to_string(),
			FieldType::Char(max_length) => format!("CHAR({})", max_length),
			FieldType::VarChar(max_length) => format!("VARCHAR({})", max_length),
			FieldType::Text => "TEXT".to_string(),
			FieldType::TinyText => "TINYTEXT".to_string(),
			FieldType::MediumText => "MEDIUMTEXT".to_string(),
			FieldType::LongText => "LONGTEXT".to_string(),
			FieldType::Date => "DATE".to_string(),
			FieldType::Time => "TIME".to_string(),
			FieldType::DateTime => "DATETIME".to_string(),
			FieldType::TimestampTz => "TIMESTAMPTZ".to_string(),
			FieldType::Decimal { precision, scale } => format!("DECIMAL({}, {})", precision, scale),
			FieldType::Float => "FLOAT".to_string(),
			FieldType::Double => "DOUBLE".to_string(),
			FieldType::Real => "REAL".to_string(),
			FieldType::Boolean => "BOOLEAN".to_string(),
			FieldType::Binary => "BINARY".to_string(),
			FieldType::Blob => "BLOB".to_string(),
			FieldType::TinyBlob => "TINYBLOB".to_string(),
			FieldType::MediumBlob => "MEDIUMBLOB".to_string(),
			FieldType::LongBlob => "LONGBLOB".to_string(),
			FieldType::Bytea => "BYTEA".to_string(),
			FieldType::Json => "JSON".to_string(),
			FieldType::JsonBinary => "JSONB".to_string(),
			// PostgreSQL-specific types
			FieldType::Array(inner) => format!("{}[]", inner.to_sql_string()),
			FieldType::HStore => "HSTORE".to_string(),
			FieldType::CIText => "CITEXT".to_string(),
			FieldType::Int4Range => "INT4RANGE".to_string(),
			FieldType::Int8Range => "INT8RANGE".to_string(),
			FieldType::NumRange => "NUMRANGE".to_string(),
			FieldType::DateRange => "DATERANGE".to_string(),
			FieldType::TsRange => "TSRANGE".to_string(),
			FieldType::TsTzRange => "TSTZRANGE".to_string(),
			FieldType::TsVector => "TSVECTOR".to_string(),
			FieldType::TsQuery => "TSQUERY".to_string(),
			FieldType::Uuid => "UUID".to_string(),
			FieldType::Year => "YEAR".to_string(),
			FieldType::Enum { values } => {
				let values_str = values
					.iter()
					.map(|v| format!("'{}'", v))
					.collect::<Vec<_>>()
					.join(",");
				format!("ENUM({})", values_str)
			}
			FieldType::Set { values } => {
				let values_str = values
					.iter()
					.map(|v| format!("'{}'", v))
					.collect::<Vec<_>>()
					.join(",");
				format!("SET({})", values_str)
			}
			FieldType::ForeignKey { to_table, .. } => {
				format!("-- ForeignKey to {}", to_table)
			}
			FieldType::OneToOne { to, .. } => {
				format!("-- OneToOne relationship to {}", to)
			}
			FieldType::ManyToMany { to, through } => match through {
				Some(table) => format!("-- ManyToMany through {}", table),
				None => format!("-- ManyToMany to {} (auto-generated)", to),
			},
			FieldType::Custom(custom_type) => custom_type.clone(),
		}
	}

	/// Get max_length if this type has one
	pub fn max_length(&self) -> Option<u32> {
		match self {
			FieldType::Char(max_length) | FieldType::VarChar(max_length) => Some(*max_length),
			_ => None,
		}
	}
}

/// Trait for field types that provide their type name as a compile-time constant
pub trait FieldTypeName {
	const NAME: &'static str;
}

// Type-safe field type markers
pub struct BigIntegerField;
impl FieldTypeName for BigIntegerField {
	const NAME: &'static str = "BigIntegerField";
}

pub struct IntegerField;
impl FieldTypeName for IntegerField {
	const NAME: &'static str = "IntegerField";
}

pub struct SmallIntegerField;
impl FieldTypeName for SmallIntegerField {
	const NAME: &'static str = "SmallIntegerField";
}

pub struct CharField;
impl FieldTypeName for CharField {
	const NAME: &'static str = "CharField";
}

pub struct TextField;
impl FieldTypeName for TextField {
	const NAME: &'static str = "TextField";
}

pub struct DateTimeField;
impl FieldTypeName for DateTimeField {
	const NAME: &'static str = "DateTimeField";
}

pub struct DateField;
impl FieldTypeName for DateField {
	const NAME: &'static str = "DateField";
}

pub struct TimeField;
impl FieldTypeName for TimeField {
	const NAME: &'static str = "TimeField";
}

pub struct BooleanField;
impl FieldTypeName for BooleanField {
	const NAME: &'static str = "BooleanField";
}

pub struct DecimalField;
impl FieldTypeName for DecimalField {
	const NAME: &'static str = "DecimalField";
}

pub struct BinaryField;
impl FieldTypeName for BinaryField {
	const NAME: &'static str = "BinaryField";
}

pub struct JSONField;
impl FieldTypeName for JSONField {
	const NAME: &'static str = "JSONField";
}

pub struct UUIDField;
impl FieldTypeName for UUIDField {
	const NAME: &'static str = "UUIDField";
}

// PostgreSQL-specific field type markers
pub struct ArrayField;
impl FieldTypeName for ArrayField {
	const NAME: &'static str = "ArrayField";
}

pub struct HStoreField;
impl FieldTypeName for HStoreField {
	const NAME: &'static str = "HStoreField";
}

pub struct CITextField;
impl FieldTypeName for CITextField {
	const NAME: &'static str = "CITextField";
}

pub struct Int4RangeField;
impl FieldTypeName for Int4RangeField {
	const NAME: &'static str = "Int4RangeField";
}

pub struct Int8RangeField;
impl FieldTypeName for Int8RangeField {
	const NAME: &'static str = "Int8RangeField";
}

pub struct NumRangeField;
impl FieldTypeName for NumRangeField {
	const NAME: &'static str = "NumRangeField";
}

pub struct DateRangeField;
impl FieldTypeName for DateRangeField {
	const NAME: &'static str = "DateRangeField";
}

pub struct TsRangeField;
impl FieldTypeName for TsRangeField {
	const NAME: &'static str = "TsRangeField";
}

pub struct TsTzRangeField;
impl FieldTypeName for TsTzRangeField {
	const NAME: &'static str = "TsTzRangeField";
}

pub struct TsVectorField;
impl FieldTypeName for TsVectorField {
	const NAME: &'static str = "TsVectorField";
}

pub struct TsQueryField;
impl FieldTypeName for TsQueryField {
	const NAME: &'static str = "TsQueryField";
}

pub mod prelude {
	pub use super::{
		// PostgreSQL-specific field types
		ArrayField,
		// Standard field types
		BigIntegerField,
		BinaryField,
		BooleanField,
		CITextField,
		CharField,
		DateField,
		DateRangeField,
		DateTimeField,
		DecimalField,
		FieldTypeName,
		HStoreField,
		Int4RangeField,
		Int8RangeField,
		IntegerField,
		JSONField,
		NumRangeField,
		SmallIntegerField,
		TextField,
		TimeField,
		TsQueryField,
		TsRangeField,
		TsTzRangeField,
		TsVectorField,
		UUIDField,
	};
}

// Into implementations for database-specific types

/// PostgreSQL Type → FieldType conversion
impl From<&sea_schema::postgres::def::Type> for FieldType {
	fn from(col_type: &sea_schema::postgres::def::Type) -> Self {
		use sea_schema::postgres::def::Type;
		match col_type {
			// Integer types
			Type::Serial | Type::Integer => FieldType::Integer,
			Type::BigSerial | Type::BigInt => FieldType::BigInteger,
			Type::SmallSerial | Type::SmallInt => FieldType::SmallInteger,

			// String types
			Type::Varchar(attr) => {
				let max_length = attr.length.unwrap_or(255).into();
				FieldType::VarChar(max_length)
			}
			Type::Char(attr) => {
				let max_length = attr.length.unwrap_or(1).into();
				FieldType::Char(max_length)
			}
			Type::Text => FieldType::Text,

			// Date/Time types
			Type::Date => FieldType::Date,
			Type::Time(_) => FieldType::Time,
			Type::Timestamp(_) => FieldType::DateTime,
			Type::TimestampWithTimeZone(_) => FieldType::TimestampTz,

			// Numeric types
			Type::Decimal(attr) | Type::Numeric(attr) => {
				let precision = attr.precision.unwrap_or(10).into();
				let scale = attr.scale.unwrap_or(2).into();
				FieldType::Decimal { precision, scale }
			}
			Type::Real => FieldType::Real,
			Type::DoublePrecision => FieldType::Double,

			// Boolean
			Type::Boolean => FieldType::Boolean,

			// Binary
			Type::Bytea => FieldType::Bytea,

			// JSON
			Type::Json => FieldType::Json,
			Type::JsonBinary => FieldType::JsonBinary,

			// UUID
			Type::Uuid => FieldType::Uuid,

			// Unknown/Custom types
			Type::Unknown(type_name) => FieldType::Custom(type_name.clone()),
			Type::PgLsn => FieldType::Custom("PG_LSN".to_string()),
			Type::Money => FieldType::Custom("MONEY".to_string()),

			// Array types - use native Array variant
			Type::Array(inner_array) => {
				if let Some(ref inner_type) = inner_array.col_type {
					let inner_field_type: FieldType = inner_type.as_ref().into();
					FieldType::Array(Box::new(inner_field_type))
				} else {
					// Default to Text array if inner type is unknown
					FieldType::Array(Box::new(FieldType::Text))
				}
			}

			// Time with time zone
			Type::TimeWithTimeZone(_) => FieldType::Time,

			// Interval types
			Type::Interval(_) => FieldType::Custom("INTERVAL".to_string()),

			// Geometric types
			Type::Point => FieldType::Custom("POINT".to_string()),
			Type::Line => FieldType::Custom("LINE".to_string()),
			Type::Lseg => FieldType::Custom("LSEG".to_string()),
			Type::Box => FieldType::Custom("BOX".to_string()),
			Type::Path => FieldType::Custom("PATH".to_string()),
			Type::Polygon => FieldType::Custom("POLYGON".to_string()),
			Type::Circle => FieldType::Custom("CIRCLE".to_string()),

			// Network address types
			Type::Cidr => FieldType::Custom("CIDR".to_string()),
			Type::Inet => FieldType::Custom("INET".to_string()),
			Type::MacAddr => FieldType::Custom("MACADDR".to_string()),
			Type::MacAddr8 => FieldType::Custom("MACADDR8".to_string()),

			// Bit string types
			Type::Bit(_) => FieldType::Custom("BIT".to_string()),
			Type::VarBit(_) => FieldType::Custom("VARBIT".to_string()),

			// Text search types - use native variants
			Type::TsVector => FieldType::TsVector,
			Type::TsQuery => FieldType::TsQuery,

			// XML type
			Type::Xml => FieldType::Custom("XML".to_string()),

			// Range types - use native variants
			Type::Int4Range => FieldType::Int4Range,
			Type::Int8Range => FieldType::Int8Range,
			Type::NumRange => FieldType::NumRange,
			Type::TsRange => FieldType::TsRange,
			Type::TsTzRange => FieldType::TsTzRange,
			Type::DateRange => FieldType::DateRange,

			// Enum types
			Type::Enum(enum_def) => FieldType::Enum {
				values: enum_def.values.clone(),
			},
		}
	}
}

/// MySQL Type → FieldType conversion
impl From<&sea_schema::mysql::def::Type> for FieldType {
	fn from(col_type: &sea_schema::mysql::def::Type) -> Self {
		use sea_schema::mysql::def::Type;
		match col_type {
			// Integer types
			Type::TinyInt(_) => FieldType::TinyInt,
			Type::SmallInt(_) => FieldType::SmallInteger,
			Type::MediumInt(_) => FieldType::MediumInt,
			Type::Int(_) => FieldType::Integer,
			Type::BigInt(_) => FieldType::BigInteger,

			// String types
			Type::Varchar(attr) | Type::NVarchar(attr) => {
				let max_length = attr.length.unwrap_or(255);
				FieldType::VarChar(max_length)
			}
			Type::Char(attr) | Type::NChar(attr) => {
				let max_length = attr.length.unwrap_or(1);
				FieldType::Char(max_length)
			}
			Type::Text(_) => FieldType::Text,
			Type::TinyText(_) => FieldType::TinyText,
			Type::MediumText(_) => FieldType::MediumText,
			Type::LongText(_) => FieldType::LongText,

			// Date/Time types
			Type::Date => FieldType::Date,
			Type::Time(_) => FieldType::Time,
			Type::DateTime(_) => FieldType::DateTime,
			Type::Timestamp(_) => FieldType::DateTime,
			Type::Year => FieldType::Year,

			// Numeric types
			Type::Decimal(attr) => {
				let precision = attr.maximum.unwrap_or(10);
				let scale = attr.decimal.unwrap_or(2);
				FieldType::Decimal { precision, scale }
			}
			Type::Float(_) => FieldType::Float,
			Type::Double(_) => FieldType::Double,

			// Binary types
			Type::Binary(_) | Type::Varbinary(_) => FieldType::Binary,
			Type::Blob(_) => FieldType::Blob,
			Type::TinyBlob => FieldType::TinyBlob,
			Type::MediumBlob => FieldType::MediumBlob,
			Type::LongBlob => FieldType::LongBlob,

			// Boolean (MySQL doesn't have native BOOLEAN, uses TINYINT(1))
			Type::Bool => FieldType::Boolean,
			Type::Bit(_) => FieldType::Boolean,

			// JSON
			Type::Json => FieldType::Json,

			// MySQL-specific collection types
			Type::Enum(enum_def) => FieldType::Enum {
				values: enum_def.values.clone(),
			},
			Type::Set(set_def) => FieldType::Set {
				values: set_def.members.clone(),
			},

			// Spatial types (store as Custom)
			Type::Geometry(_)
			| Type::Point(_)
			| Type::LineString(_)
			| Type::Polygon(_)
			| Type::MultiPoint(_)
			| Type::MultiLineString(_)
			| Type::MultiPolygon(_)
			| Type::GeometryCollection(_) => FieldType::Custom(format!("{:?}", col_type)),

			// Unknown types
			Type::Unknown(type_name) => FieldType::Custom(type_name.clone()),

			// Serial is auto_increment BIGINT
			Type::Serial => FieldType::BigInteger,
		}
	}
}

/// SeaQuery ColumnType → FieldType conversion (for SQLite)
impl From<&sea_schema::sea_query::ColumnType> for FieldType {
	fn from(col_type: &sea_schema::sea_query::ColumnType) -> Self {
		use sea_schema::sea_query::ColumnType;
		match col_type {
			// Integer types
			ColumnType::TinyInteger => FieldType::TinyInt,
			ColumnType::SmallInteger => FieldType::SmallInteger,
			ColumnType::Integer => FieldType::Integer,
			ColumnType::BigInteger => FieldType::BigInteger,

			// Floating point types
			ColumnType::Float => FieldType::Float,
			ColumnType::Double => FieldType::Double,
			ColumnType::Decimal(Some((precision, scale))) => FieldType::Decimal {
				precision: *precision,
				scale: *scale,
			},
			ColumnType::Decimal(None) => FieldType::Decimal {
				precision: 10,
				scale: 2,
			},

			// String types
			ColumnType::String(str_len) => match str_len {
				sea_schema::sea_query::StringLen::N(length) => FieldType::VarChar(*length),
				sea_schema::sea_query::StringLen::None => FieldType::VarChar(255),
				sea_schema::sea_query::StringLen::Max => FieldType::Text,
			},
			ColumnType::Text => FieldType::Text,
			ColumnType::Char(length) => {
				if let Some(len) = length {
					FieldType::Char(*len)
				} else {
					FieldType::Char(1)
				}
			}

			// Binary
			ColumnType::Binary(_) => FieldType::Binary,

			// Boolean
			ColumnType::Boolean => FieldType::Boolean,

			// Date/Time types
			ColumnType::Date => FieldType::Date,
			ColumnType::Time => FieldType::Time,
			ColumnType::DateTime => FieldType::DateTime,
			ColumnType::TimestampWithTimeZone => FieldType::TimestampTz,
			ColumnType::Timestamp => FieldType::DateTime,

			// UUID
			ColumnType::Uuid => FieldType::Uuid,

			// JSON
			ColumnType::Json => FieldType::Json,
			ColumnType::JsonBinary => FieldType::JsonBinary,

			// Arrays (store as Custom)
			ColumnType::Array(_) => FieldType::Custom(format!("{:?}", col_type)),

			// Custom types
			ColumnType::Custom(_) => FieldType::Custom(format!("{:?}", col_type)),

			// Enum
			ColumnType::Enum { .. } => FieldType::Custom(format!("{:?}", col_type)),

			// Cidr/Inet (PostgreSQL network types)
			ColumnType::Cidr | ColumnType::Inet => FieldType::Custom(format!("{:?}", col_type)),

			// Money
			ColumnType::Money(_) => FieldType::Custom(format!("{:?}", col_type)),

			// Unknown types
			_ => FieldType::Custom(format!("{:?}", col_type)),
		}
	}
}

/// Display implementation for FieldType
impl std::fmt::Display for FieldType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.to_sql_string())
	}
}
