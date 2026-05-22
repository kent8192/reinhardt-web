//! Lookup type and value definitions

use crate::orm::Model;
use chrono::Timelike;
use serde::{Deserialize, Serialize};

/// Lookup type - defines how to compare the field value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LookupType {
	// Equality
	/// Exact variant.
	Exact, // =
	/// IExact variant.
	IExact, // ILIKE (case-insensitive)
	/// Ne variant.
	Ne, // !=

	// Pattern matching
	/// Contains variant.
	Contains, // LIKE '%x%'
	/// IContains variant.
	IContains, // ILIKE '%x%'
	/// StartsWith variant.
	StartsWith, // LIKE 'x%'
	/// IStartsWith variant.
	IStartsWith, // ILIKE 'x%'
	/// EndsWith variant.
	EndsWith, // LIKE '%x'
	/// IEndsWith variant.
	IEndsWith, // ILIKE '%x'
	/// Regex variant.
	Regex, // ~ (PostgreSQL)
	/// IRegex variant.
	IRegex, // ~* (PostgreSQL)

	// Comparison
	/// Gt variant.
	Gt, // >
	/// Gte variant.
	Gte, // >=
	/// Lt variant.
	Lt, // <
	/// Lte variant.
	Lte, // <=
	/// Range variant.
	Range, // BETWEEN

	// Set operations
	/// In variant.
	In, // IN
	/// NotIn variant.
	NotIn, // NOT IN

	// NULL checks
	/// IsNull variant.
	IsNull, // IS NULL
	/// IsNotNull variant.
	IsNotNull, // IS NOT NULL
}

/// Lookup value - the value to compare against
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LookupValue {
	/// String variant.
	String(String),
	/// Int variant.
	Int(i64),
	/// Float variant.
	Float(f64),
	/// Bool variant.
	Bool(bool),
	/// Array variant.
	Array(Vec<LookupValue>),
	/// Range variant.
	Range(Box<LookupValue>, Box<LookupValue>),
	/// Null variant.
	Null,
}

impl From<String> for LookupValue {
	fn from(s: String) -> Self {
		LookupValue::String(s)
	}
}

impl From<&str> for LookupValue {
	fn from(s: &str) -> Self {
		LookupValue::String(s.to_string())
	}
}

impl From<i32> for LookupValue {
	fn from(i: i32) -> Self {
		LookupValue::Int(i as i64)
	}
}

impl From<i64> for LookupValue {
	fn from(i: i64) -> Self {
		LookupValue::Int(i)
	}
}

impl From<f32> for LookupValue {
	fn from(f: f32) -> Self {
		LookupValue::Float(f as f64)
	}
}

impl From<f64> for LookupValue {
	fn from(f: f64) -> Self {
		LookupValue::Float(f)
	}
}

impl From<bool> for LookupValue {
	fn from(b: bool) -> Self {
		LookupValue::Bool(b)
	}
}

impl From<()> for LookupValue {
	fn from(_: ()) -> Self {
		LookupValue::Null
	}
}

// DateTime and Date conversions
impl From<super::traits::DateTime> for LookupValue {
	fn from(dt: super::traits::DateTime) -> Self {
		LookupValue::Int(dt.timestamp)
	}
}

impl From<super::traits::Date> for LookupValue {
	fn from(date: super::traits::Date) -> Self {
		// Encode as days since epoch or similar
		let days = date.year * 10000 + (date.month as i32) * 100 + (date.day as i32);
		LookupValue::Int(days as i64)
	}
}

// chrono integration
impl From<chrono::NaiveDateTime> for LookupValue {
	fn from(dt: chrono::NaiveDateTime) -> Self {
		LookupValue::Int(dt.and_utc().timestamp())
	}
}

impl From<chrono::NaiveDate> for LookupValue {
	fn from(date: chrono::NaiveDate) -> Self {
		// Convert to timestamp at midnight UTC
		LookupValue::Int(
			date.and_hms_opt(0, 0, 0)
				.map(|dt| dt.and_utc().timestamp())
				.unwrap_or(0),
		)
	}
}

impl From<chrono::NaiveTime> for LookupValue {
	fn from(time: chrono::NaiveTime) -> Self {
		// Store as seconds since midnight
		LookupValue::Int(time.num_seconds_from_midnight() as i64)
	}
}

impl<Tz: chrono::TimeZone> From<chrono::DateTime<Tz>> for LookupValue {
	fn from(dt: chrono::DateTime<Tz>) -> Self {
		LookupValue::Int(dt.timestamp())
	}
}

impl<T: Into<LookupValue>> From<(T, T)> for LookupValue {
	fn from((start, end): (T, T)) -> Self {
		LookupValue::Range(Box::new(start.into()), Box::new(end.into()))
	}
}

/// A complete lookup specification ready to be compiled to SQL
///
/// # Breaking Change
///
/// The type of `field_path` has been changed from `Vec<&'static str>` to `Vec<String>`.
/// This allows support for dynamic table aliases.
#[derive(Debug, Clone)]
pub struct Lookup<M: Model> {
	pub(crate) field_path: Vec<String>,
	pub(crate) lookup_type: LookupType,
	pub(crate) value: LookupValue,
	pub(crate) _phantom: std::marker::PhantomData<M>,
}

impl<M: Model> Lookup<M> {
	/// Create a new lookup for field filtering in QuerySets
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_db::orm::query_fields::{Lookup, LookupType, LookupValue};
	/// # use reinhardt_db::orm::Model;
	/// # use serde::{Serialize, Deserialize};
	/// # #[derive(Clone, Serialize, Deserialize)]
	/// # struct User { id: Option<i64> }
	/// # #[derive(Clone)]
	/// # struct UserFields;
	/// # impl reinhardt_db::orm::FieldSelector for UserFields {
	/// #     fn with_alias(self, _alias: &str) -> Self { self }
	/// # }
	/// # impl Model for User {
	/// #     type PrimaryKey = i64;
	/// #     type Fields = UserFields;
	/// #     fn app_label() -> &'static str { "app" }
	/// #     fn table_name() -> &'static str { "users" }
	/// #     fn new_fields() -> Self::Fields { UserFields }
	/// #     fn primary_key(&self) -> Option<Self::PrimaryKey> { self.id }
	/// #     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// #     fn primary_key_field() -> &'static str { "id" }
	/// # }
	///
	/// let lookup = Lookup::<User>::new(
	///     vec!["name".to_string()],
	///     LookupType::Exact,
	///     LookupValue::String("Alice".to_string())
	/// );
	/// // Represents: WHERE name = 'Alice'
	/// assert_eq!(lookup.field_path(), &["name".to_string()]);
	/// assert_eq!(*lookup.lookup_type(), LookupType::Exact);
	/// ```
	pub fn new(field_path: Vec<String>, lookup_type: LookupType, value: LookupValue) -> Self {
		Self {
			field_path,
			lookup_type,
			value,
			_phantom: std::marker::PhantomData,
		}
	}
	/// Get the field path
	///
	pub fn field_path(&self) -> &[String] {
		&self.field_path
	}
	/// Get the lookup type
	///
	pub fn lookup_type(&self) -> &LookupType {
		&self.lookup_type
	}
	/// Get the lookup value
	///
	pub fn value(&self) -> &LookupValue {
		&self.value
	}
}

// =============================================================================
// Bridge: Lookup<M> → Filter
//
// Lets the typed `Field::eq()` / `.gt()` / ... builder (which returns
// `Lookup<M>`) flow into `Manager::filter` / `QuerySet::filter`, both of
// which accept `impl Into<Filter>` (Issue #4650).
//
// Only the 1-to-1 mappable subset is implemented today. Case-insensitive
// (`IExact`, `IContains`, `IStartsWith`, `IEndsWith`), regex (`Regex`,
// `IRegex`), and `Range` have no corresponding `FilterOperator` variant
// yet and lower to `unimplemented!()` so adding them later (separate
// follow-up issues) is an additive change instead of silent miscompilation.
// =============================================================================

impl<M: Model> From<Lookup<M>> for crate::orm::query::Filter {
	fn from(lookup: Lookup<M>) -> Self {
		use crate::orm::query::{Filter, FilterOperator};

		let Lookup {
			field_path,
			lookup_type,
			value,
			..
		} = lookup;

		let operator = match lookup_type {
			LookupType::Exact => FilterOperator::Eq,
			LookupType::Ne => FilterOperator::Ne,
			LookupType::Gt => FilterOperator::Gt,
			LookupType::Gte => FilterOperator::Gte,
			LookupType::Lt => FilterOperator::Lt,
			LookupType::Lte => FilterOperator::Lte,
			LookupType::Contains => FilterOperator::Contains,
			LookupType::StartsWith => FilterOperator::StartsWith,
			LookupType::EndsWith => FilterOperator::EndsWith,
			LookupType::In => FilterOperator::In,
			LookupType::NotIn => FilterOperator::NotIn,
			LookupType::IsNull => FilterOperator::IsNull,
			LookupType::IsNotNull => FilterOperator::IsNotNull,
			other @ (LookupType::IExact
			| LookupType::IContains
			| LookupType::IStartsWith
			| LookupType::IEndsWith
			| LookupType::Regex
			| LookupType::IRegex
			| LookupType::Range) => {
				unimplemented!(
					"LookupType::{:?} cannot be lowered to FilterOperator yet — see Issue #4650",
					other
				)
			}
		};

		Filter::new(field_path.join("."), operator, value.into())
	}
}

impl From<LookupValue> for crate::orm::query::FilterValue {
	fn from(value: LookupValue) -> Self {
		use crate::orm::query::FilterValue;

		match value {
			LookupValue::String(s) => FilterValue::String(s),
			LookupValue::Int(i) => FilterValue::Integer(i),
			LookupValue::Float(f) => FilterValue::Float(f),
			LookupValue::Bool(b) => FilterValue::Boolean(b),
			LookupValue::Null => FilterValue::Null,
			LookupValue::Array(items) => {
				// FilterValue::Array is Vec<String>, so stringify each element.
				let stringified = items
					.into_iter()
					.map(|v| match v {
						LookupValue::String(s) => s,
						LookupValue::Int(i) => i.to_string(),
						LookupValue::Float(f) => f.to_string(),
						LookupValue::Bool(b) => b.to_string(),
						LookupValue::Null => "NULL".to_string(),
						LookupValue::Array(_) | LookupValue::Range(_, _) => {
							unimplemented!(
								"nested LookupValue::Array / Range cannot be lowered to FilterValue::Array yet — see Issue #4650"
							)
						}
					})
					.collect();
				FilterValue::Array(stringified)
			}
			LookupValue::Range(_, _) => {
				unimplemented!(
					"LookupValue::Range cannot be lowered to FilterValue yet — see Issue #4650"
				)
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::super::field::Field;
	use super::*;
	use crate::orm::Model;
	use crate::orm::query::{Filter, FilterOperator, FilterValue};

	// Minimal Model fixture for the conversion tests — uses the same shape as
	// the existing `field::tests::TestUser` so the file's existing fixture
	// patterns stay recognizable.
	#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
	struct TestUser {
		id: i64,
	}

	#[derive(Clone)]
	struct TestUserFields;

	impl crate::orm::FieldSelector for TestUserFields {
		fn with_alias(self, _alias: &str) -> Self {
			self
		}
	}

	impl Model for TestUser {
		type PrimaryKey = i64;
		type Fields = TestUserFields;
		fn app_label() -> &'static str {
			"tests"
		}
		fn table_name() -> &'static str {
			"test_user"
		}
		fn new_fields() -> Self::Fields {
			TestUserFields
		}
		fn primary_key(&self) -> Option<Self::PrimaryKey> {
			Some(self.id)
		}
		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = value;
		}
		fn primary_key_field() -> &'static str {
			"id"
		}
	}

	#[test]
	fn lookup_eq_into_filter_roundtrip() {
		// Arrange: typed builder produces a Lookup<TestUser>
		let lookup = Field::<TestUser, i64>::new(vec!["id".to_string()]).eq(42);

		// Act: lower via Into<Filter>
		let filter: Filter = lookup.into();

		// Assert: field path, operator and value all round-trip correctly
		assert_eq!(filter.field, "id");
		assert!(matches!(filter.operator, FilterOperator::Eq));
		assert!(matches!(filter.value, FilterValue::Integer(42)));
	}

	#[test]
	fn lookup_comparison_variants_lower_to_filter_operators() {
		// Each Field comparator should map to the matching FilterOperator
		let cases: Vec<(Lookup<TestUser>, FilterOperator)> = vec![
			(
				Field::<TestUser, i64>::new(vec!["id".to_string()]).ne(1),
				FilterOperator::Ne,
			),
			(
				Field::<TestUser, i64>::new(vec!["id".to_string()]).gt(1),
				FilterOperator::Gt,
			),
			(
				Field::<TestUser, i64>::new(vec!["id".to_string()]).gte(1),
				FilterOperator::Gte,
			),
			(
				Field::<TestUser, i64>::new(vec!["id".to_string()]).lt(1),
				FilterOperator::Lt,
			),
			(
				Field::<TestUser, i64>::new(vec!["id".to_string()]).lte(1),
				FilterOperator::Lte,
			),
		];

		for (lookup, expected_op) in cases {
			let filter: Filter = lookup.into();
			assert_eq!(filter.field, "id");
			// `FilterOperator` does not derive `PartialEq`, so use discriminant
			// equality via formatted debug output (stable across all enum
			// variants and avoids touching that public type's derive set).
			assert_eq!(
				format!("{:?}", filter.operator),
				format!("{:?}", expected_op)
			);
		}
	}

	#[test]
	fn lookup_iexact_panics_with_documented_message() {
		// IExact is one of the out-of-scope variants — confirm the From impl
		// fails fast with the documented message instead of silently
		// miscompiling.
		let lookup = Lookup::<TestUser>::new(
			vec!["email".to_string()],
			LookupType::IExact,
			LookupValue::String("Alice".to_string()),
		);

		let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
			let _: Filter = lookup.into();
		}));

		let panic = result.expect_err("conversion should have panicked");
		let msg = panic
			.downcast_ref::<String>()
			.map(String::as_str)
			.or_else(|| panic.downcast_ref::<&str>().copied())
			.unwrap_or("");
		assert!(
			msg.contains("LookupType::IExact") && msg.contains("Issue #4650"),
			"unexpected panic message: {msg}"
		);
	}
}
