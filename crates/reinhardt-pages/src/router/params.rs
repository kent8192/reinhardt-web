//! Path parameter extraction for typed route handlers.
//!
//! This module provides typed parameter extraction from URL paths,
//! similar to backend's `Path<T>` extractor.
//!
//! # Example
//!
//! ```ignore
//! use reinhardt_pages::router::{Router, PathParams};
//!
//! let router = Router::new()
//!     .route_params("/users/{id}/", |PathParams(id): PathParams<i64>| {
//!         View::text(format!("User ID: {}", id))
//!     });
//! ```

use std::collections::HashMap;
use std::ops::Deref;

use super::core::PathError;

/// Context for parameter extraction.
///
/// Contains both named parameters (for backward compatibility)
/// and ordered parameter values (for tuple extraction).
#[derive(Debug, Clone)]
pub struct ParamContext {
	/// Named parameters extracted from the path.
	#[allow(dead_code)] // Reserved for future named parameter access
	pub(crate) params: HashMap<String, String>,
	/// Parameter values in the order they appear in the pattern.
	///
	/// This guarantees that tuple extraction works correctly by index,
	/// matching the order of parameters in the URL pattern.
	pub(crate) param_values: Vec<String>,
}

impl ParamContext {
	/// Creates a new parameter context.
	pub fn new(params: HashMap<String, String>, param_values: Vec<String>) -> Self {
		Self {
			params,
			param_values,
		}
	}

	/// Returns the number of parameters.
	pub fn len(&self) -> usize {
		self.param_values.len()
	}

	/// Returns whether there are no parameters.
	pub fn is_empty(&self) -> bool {
		self.param_values.is_empty()
	}
}

/// Trait for extracting typed values from path parameters.
///
/// This trait is similar to the backend's `FromRequest` trait,
/// but simplified for client-side routing (no async required).
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::router::{PathParams, FromPath, ParamContext, PathError};
///
/// // Custom type implementing FromPath
/// struct UserId(i64);
///
/// impl FromPath for UserId {
///     fn from_path(ctx: &ParamContext) -> Result<Self, PathError> {
///         if ctx.param_values.len() != 1 {
///             return Err(PathError::CountMismatch {
///                 expected: 1,
///                 actual: ctx.param_values.len(),
///             });
///         }
///         ctx.param_values[0].parse::<i64>()
///             .map(UserId)
///             .map_err(|e| PathError::ParseError {
///                 param_index: Some(0),
///                 param_type: "UserId",
///                 raw_value: ctx.param_values[0].clone(),
///                 source: format!("{}", e),
///             })
///     }
/// }
/// ```
pub trait FromPath: Sized {
	/// Extracts Self from the parameter context.
	///
	/// # Errors
	///
	/// Returns [`PathError::CountMismatch`] if the number of parameters doesn't match.
	/// Returns [`PathError::ParseError`] if parameter parsing fails.
	fn from_path(ctx: &ParamContext) -> Result<Self, PathError>;
}

/// Wrapper type for path parameters.
///
/// This type provides type-safe access to path parameters extracted from URLs.
/// It implements `Deref` for convenient access to the inner value.
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::router::PathParams;
/// use reinhardt_pages::component::View;
///
/// fn user_detail(PathParams(id): PathParams<i64>) -> View {
///     View::text(format!("User ID: {}", id))
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PathParams<T>(pub T);

impl<T> PathParams<T> {
	/// Unwraps the inner value.
	pub fn into_inner(self) -> T {
		self.0
	}
}

impl<T> Deref for PathParams<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T> AsRef<T> for PathParams<T> {
	fn as_ref(&self) -> &T {
		&self.0
	}
}

// Macro for implementing FromPath for primitive types
macro_rules! impl_from_path_for_primitive {
	($($ty:ty => $type_name:expr),* $(,)?) => {
		$(
			impl FromPath for $ty {
				fn from_path(ctx: &ParamContext) -> Result<Self, PathError> {
					if ctx.param_values.len() != 1 {
						return Err(PathError::CountMismatch {
							expected: 1,
							actual: ctx.param_values.len(),
						});
					}

					ctx.param_values[0]
						.parse::<$ty>()
						.map_err(|e| PathError::ParseError {
							param_index: Some(0),
							param_type: $type_name,
							raw_value: ctx.param_values[0].clone(),
							source: format!("{}", e),
						})
				}
			}
		)*
	};
}

// Implement FromPath for common primitive types
impl_from_path_for_primitive! {
	i32 => "i32",
	i64 => "i64",
	u32 => "u32",
	u64 => "u64",
	bool => "bool",
}

// Special implementation for String (no parsing needed)
impl FromPath for String {
	fn from_path(ctx: &ParamContext) -> Result<Self, PathError> {
		if ctx.param_values.len() != 1 {
			return Err(PathError::CountMismatch {
				expected: 1,
				actual: ctx.param_values.len(),
			});
		}

		Ok(ctx.param_values[0].clone())
	}
}

// Implementation for PathParams<T>
impl<T: FromPath> FromPath for PathParams<T> {
	fn from_path(ctx: &ParamContext) -> Result<Self, PathError> {
		T::from_path(ctx).map(PathParams)
	}
}

// Helper macro for parsing tuple elements
macro_rules! parse_tuple_element {
	($ctx:expr, $idx:expr, $ty:ty) => {{
		if $idx >= $ctx.param_values.len() {
			return Err(PathError::CountMismatch {
				expected: $idx + 1,
				actual: $ctx.param_values.len(),
			});
		}

		$ctx.param_values[$idx]
			.parse::<$ty>()
			.map_err(|e| PathError::ParseError {
				param_index: Some($idx),
				param_type: std::any::type_name::<$ty>(),
				raw_value: $ctx.param_values[$idx].clone(),
				source: format!("{}", e),
			})?
	}};
}

// Macro for implementing FromPath for tuples
macro_rules! impl_from_path_for_tuple {
	($($idx:tt => $ty:ident),+ $(,)?) => {
		impl<$($ty),+> FromPath for ($($ty,)+)
		where
			$($ty: std::str::FromStr,)+
			$(<$ty as std::str::FromStr>::Err: std::fmt::Display,)+
		{
			fn from_path(ctx: &ParamContext) -> Result<Self, PathError> {
				let expected_count = [$($idx),+].len();
				if ctx.param_values.len() != expected_count {
					return Err(PathError::CountMismatch {
						expected: expected_count,
						actual: ctx.param_values.len(),
					});
				}

				Ok((
					$(parse_tuple_element!(ctx, $idx, $ty),)+
				))
			}
		}
	};
}

// Implement FromPath for tuples of 2 to 6 elements
impl_from_path_for_tuple!(0 => A, 1 => B);
impl_from_path_for_tuple!(0 => A, 1 => B, 2 => C);
impl_from_path_for_tuple!(0 => A, 1 => B, 2 => C, 3 => D);
impl_from_path_for_tuple!(0 => A, 1 => B, 2 => C, 3 => D, 4 => E);
impl_from_path_for_tuple!(0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F);

// Feature-gated type implementations

#[cfg(feature = "uuid")]
impl FromPath for uuid::Uuid {
	fn from_path(ctx: &ParamContext) -> Result<Self, PathError> {
		if ctx.param_values.len() != 1 {
			return Err(PathError::CountMismatch {
				expected: 1,
				actual: ctx.param_values.len(),
			});
		}

		ctx.param_values[0]
			.parse::<uuid::Uuid>()
			.map_err(|e| PathError::ParseError {
				param_index: Some(0),
				param_type: "uuid::Uuid",
				raw_value: ctx.param_values[0].clone(),
				source: format!("{}", e),
			})
	}
}

#[cfg(feature = "chrono")]
impl FromPath for chrono::NaiveDate {
	fn from_path(ctx: &ParamContext) -> Result<Self, PathError> {
		if ctx.param_values.len() != 1 {
			return Err(PathError::CountMismatch {
				expected: 1,
				actual: ctx.param_values.len(),
			});
		}

		chrono::NaiveDate::parse_from_str(&ctx.param_values[0], "%Y-%m-%d").map_err(|e| {
			PathError::ParseError {
				param_index: Some(0),
				param_type: "chrono::NaiveDate",
				raw_value: ctx.param_values[0].clone(),
				source: format!("{}", e),
			}
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_param_context_new() {
		let mut params = HashMap::new();
		params.insert("id".to_string(), "42".to_string());
		let param_values = vec!["42".to_string()];

		let ctx = ParamContext::new(params.clone(), param_values.clone());

		assert_eq!(ctx.params, params);
		assert_eq!(ctx.param_values, param_values);
		assert_eq!(ctx.len(), 1);
		assert!(!ctx.is_empty());
	}

	#[test]
	fn test_param_context_empty() {
		let ctx = ParamContext::new(HashMap::new(), Vec::new());

		assert_eq!(ctx.len(), 0);
		assert!(ctx.is_empty());
	}

	#[test]
	fn test_path_params_deref() {
		let params = PathParams(42i64);
		assert_eq!(*params, 42);
	}

	#[test]
	fn test_path_params_into_inner() {
		let params = PathParams("hello".to_string());
		assert_eq!(params.into_inner(), "hello");
	}

	#[test]
	fn test_path_params_as_ref() {
		let params = PathParams(42i64);
		let value: &i64 = params.as_ref();
		assert_eq!(*value, 42);
	}

	// FromPath implementation tests
	#[test]
	fn test_from_path_i32() {
		let mut params = HashMap::new();
		params.insert("id".to_string(), "42".to_string());
		let ctx = ParamContext::new(params, vec!["42".to_string()]);

		let result = i32::from_path(&ctx);
		assert!(result.is_ok());
		assert_eq!(result.unwrap(), 42);
	}

	#[test]
	fn test_from_path_i64() {
		let ctx = ParamContext::new(HashMap::new(), vec!["9223372036854775807".to_string()]);

		let result = i64::from_path(&ctx);
		assert!(result.is_ok());
		assert_eq!(result.unwrap(), 9223372036854775807);
	}

	#[test]
	fn test_from_path_u32() {
		let ctx = ParamContext::new(HashMap::new(), vec!["42".to_string()]);

		let result = u32::from_path(&ctx);
		assert!(result.is_ok());
		assert_eq!(result.unwrap(), 42);
	}

	#[test]
	fn test_from_path_u64() {
		let ctx = ParamContext::new(HashMap::new(), vec!["18446744073709551615".to_string()]);

		let result = u64::from_path(&ctx);
		assert!(result.is_ok());
		assert_eq!(result.unwrap(), 18446744073709551615);
	}

	#[test]
	fn test_from_path_bool() {
		let ctx_true = ParamContext::new(HashMap::new(), vec!["true".to_string()]);
		let result = bool::from_path(&ctx_true);
		assert!(result.is_ok());
		assert_eq!(result.unwrap(), true);

		let ctx_false = ParamContext::new(HashMap::new(), vec!["false".to_string()]);
		let result = bool::from_path(&ctx_false);
		assert!(result.is_ok());
		assert_eq!(result.unwrap(), false);
	}

	#[test]
	fn test_from_path_string() {
		let ctx = ParamContext::new(HashMap::new(), vec!["hello".to_string()]);

		let result = String::from_path(&ctx);
		assert!(result.is_ok());
		assert_eq!(result.unwrap(), "hello");
	}

	#[test]
	fn test_from_path_parse_error() {
		let ctx = ParamContext::new(HashMap::new(), vec!["not_a_number".to_string()]);

		let result = i32::from_path(&ctx);
		assert!(result.is_err());

		match result {
			Err(PathError::ParseError {
				param_index,
				param_type,
				raw_value,
				..
			}) => {
				assert_eq!(param_index, Some(0));
				assert_eq!(param_type, "i32");
				assert_eq!(raw_value, "not_a_number");
			}
			_ => panic!("Expected ParseError"),
		}
	}

	#[test]
	fn test_from_path_count_mismatch() {
		let ctx = ParamContext::new(HashMap::new(), vec!["42".to_string(), "43".to_string()]);

		let result = i32::from_path(&ctx);
		assert!(result.is_err());

		match result {
			Err(PathError::CountMismatch { expected, actual }) => {
				assert_eq!(expected, 1);
				assert_eq!(actual, 2);
			}
			_ => panic!("Expected CountMismatch"),
		}
	}

	#[test]
	fn test_path_params_from_path() {
		let ctx = ParamContext::new(HashMap::new(), vec!["42".to_string()]);

		let result = PathParams::<i32>::from_path(&ctx);
		assert!(result.is_ok());
		assert_eq!(result.unwrap().0, 42);
	}

	// Tuple FromPath implementation tests
	#[test]
	fn test_from_path_tuple_2() {
		let ctx = ParamContext::new(HashMap::new(), vec!["42".to_string(), "hello".to_string()]);

		let result = <(i32, String)>::from_path(&ctx);
		assert!(result.is_ok());
		let (a, b) = result.unwrap();
		assert_eq!(a, 42);
		assert_eq!(b, "hello");
	}

	#[test]
	fn test_from_path_tuple_3() {
		let ctx = ParamContext::new(
			HashMap::new(),
			vec!["42".to_string(), "true".to_string(), "100".to_string()],
		);

		let result = <(i32, bool, u32)>::from_path(&ctx);
		assert!(result.is_ok());
		let (a, b, c) = result.unwrap();
		assert_eq!(a, 42);
		assert_eq!(b, true);
		assert_eq!(c, 100);
	}

	#[test]
	fn test_from_path_tuple_mixed_types() {
		let ctx = ParamContext::new(
			HashMap::new(),
			vec![
				"123".to_string(),
				"456".to_string(),
				"test".to_string(),
				"true".to_string(),
			],
		);

		let result = <(i64, u64, String, bool)>::from_path(&ctx);
		assert!(result.is_ok());
		let (a, b, c, d) = result.unwrap();
		assert_eq!(a, 123);
		assert_eq!(b, 456);
		assert_eq!(c, "test");
		assert_eq!(d, true);
	}

	#[test]
	fn test_from_path_tuple_count_mismatch() {
		let ctx = ParamContext::new(HashMap::new(), vec!["42".to_string()]);

		let result = <(i32, String)>::from_path(&ctx);
		assert!(result.is_err());

		match result {
			Err(PathError::CountMismatch { expected, actual }) => {
				assert_eq!(expected, 2);
				assert_eq!(actual, 1);
			}
			_ => panic!("Expected CountMismatch"),
		}
	}

	#[test]
	fn test_from_path_tuple_parse_error() {
		let ctx = ParamContext::new(
			HashMap::new(),
			vec!["not_a_number".to_string(), "hello".to_string()],
		);

		let result = <(i32, String)>::from_path(&ctx);
		assert!(result.is_err());

		match result {
			Err(PathError::ParseError {
				param_index,
				raw_value,
				..
			}) => {
				assert_eq!(param_index, Some(0));
				assert_eq!(raw_value, "not_a_number");
			}
			_ => panic!("Expected ParseError"),
		}
	}

	#[test]
	fn test_from_path_tuple_6_elements() {
		let ctx = ParamContext::new(
			HashMap::new(),
			vec![
				"1".to_string(),
				"2".to_string(),
				"3".to_string(),
				"4".to_string(),
				"5".to_string(),
				"6".to_string(),
			],
		);

		let result = <(i32, i32, i32, i32, i32, i32)>::from_path(&ctx);
		assert!(result.is_ok());
		let (a, b, c, d, e, f) = result.unwrap();
		assert_eq!((a, b, c, d, e, f), (1, 2, 3, 4, 5, 6));
	}

	#[test]
	#[cfg(feature = "uuid")]
	fn test_from_path_uuid() {
		use uuid::Uuid;

		let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
		let ctx = ParamContext::new(HashMap::new(), vec![uuid_str.to_string()]);

		let result = Uuid::from_path(&ctx);
		assert!(result.is_ok());
		assert_eq!(result.unwrap(), Uuid::parse_str(uuid_str).unwrap());
	}

	#[test]
	#[cfg(feature = "uuid")]
	fn test_from_path_uuid_invalid() {
		use uuid::Uuid;

		let ctx = ParamContext::new(HashMap::new(), vec!["not-a-uuid".to_string()]);

		let result = Uuid::from_path(&ctx);
		assert!(result.is_err());

		match result {
			Err(PathError::ParseError {
				param_type,
				raw_value,
				..
			}) => {
				assert_eq!(param_type, "uuid::Uuid");
				assert_eq!(raw_value, "not-a-uuid");
			}
			_ => panic!("Expected ParseError"),
		}
	}

	#[test]
	#[cfg(feature = "chrono")]
	fn test_from_path_naive_date() {
		use chrono::NaiveDate;

		let date_str = "2024-01-15";
		let ctx = ParamContext::new(HashMap::new(), vec![date_str.to_string()]);

		let result = NaiveDate::from_path(&ctx);
		assert!(result.is_ok());
		assert_eq!(
			result.unwrap(),
			NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()
		);
	}

	#[test]
	#[cfg(feature = "chrono")]
	fn test_from_path_naive_date_invalid() {
		use chrono::NaiveDate;

		let ctx = ParamContext::new(HashMap::new(), vec!["not-a-date".to_string()]);

		let result = NaiveDate::from_path(&ctx);
		assert!(result.is_err());

		match result {
			Err(PathError::ParseError {
				param_type,
				raw_value,
				..
			}) => {
				assert_eq!(param_type, "chrono::NaiveDate");
				assert_eq!(raw_value, "not-a-date");
			}
			_ => panic!("Expected ParseError"),
		}
	}
}
