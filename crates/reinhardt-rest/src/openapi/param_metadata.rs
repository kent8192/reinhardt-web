//! Parameter metadata extraction for OpenAPI schema generation
//!
//! This module provides utilities to extract OpenAPI parameter metadata from
//! Reinhardt's parameter types (Path, Query, Header, Cookie).

use super::openapi::ParameterIn as ParameterLocation;
use super::{Parameter, Required};
use crate::ToSchema;
use std::marker::PhantomData;

/// Trait for types that can provide OpenAPI parameter metadata
///
/// This trait is implemented by parameter extractors to provide metadata
/// for OpenAPI schema generation.
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_rest::openapi::param_metadata::ParameterMetadata;
/// use reinhardt_rest::openapi::{Parameter, ParameterLocation, ToSchema, Required};
/// use utoipa::openapi::path::ParameterBuilder;
///
/// struct PathExtractor<T> {
///     _marker: std::marker::PhantomData<T>,
/// }
///
/// impl<T: ToSchema> ParameterMetadata for PathExtractor<T> {
///     fn parameter_metadata(name: &str, include_in_schema: bool) -> Option<Parameter> {
///         if !include_in_schema {
///             return None;
///         }
///         Some(
///             ParameterBuilder::new()
///                 .name(name)
///                 .parameter_in(ParameterLocation::Path)
///                 .required(Required::True)
///                 .schema(Some(T::schema()))
///                 .build()
///         )
///     }
/// }
/// ```
pub trait ParameterMetadata {
	/// Generate OpenAPI parameter metadata
	///
	/// # Arguments
	///
	/// * `name` - The parameter name
	/// * `include_in_schema` - Whether to include this parameter in the schema
	///
	/// # Returns
	///
	/// `Some(Parameter)` if the parameter should be included in the schema,
	/// `None` if `include_in_schema` is false or the parameter is hidden.
	fn parameter_metadata(name: &str, include_in_schema: bool) -> Option<Parameter>;
}

/// Marker type for Path parameter metadata
pub struct PathParam<T>(PhantomData<T>);

/// Marker type for Query parameter metadata
pub struct QueryParam<T>(PhantomData<T>);

/// Marker type for Header parameter metadata
pub struct HeaderParam<T>(PhantomData<T>);

/// Marker type for Cookie parameter metadata
pub struct CookieParam<T>(PhantomData<T>);

impl<T: ToSchema> ParameterMetadata for PathParam<T> {
	fn parameter_metadata(name: &str, include_in_schema: bool) -> Option<Parameter> {
		if !include_in_schema {
			return None;
		}

		use utoipa::openapi::path::ParameterBuilder;

		Some(
			ParameterBuilder::new()
                .name(name)
                .parameter_in(ParameterLocation::Path)
                .required(Required::True) // Path parameters are always required
                .schema(Some(T::schema()))
                .build(),
		)
	}
}

impl<T: ToSchema> ParameterMetadata for QueryParam<T> {
	fn parameter_metadata(name: &str, include_in_schema: bool) -> Option<Parameter> {
		if !include_in_schema {
			return None;
		}

		use utoipa::openapi::path::ParameterBuilder;

		Some(
			ParameterBuilder::new()
                .name(name)
                .parameter_in(ParameterLocation::Query)
                .required(Required::False) // Query parameters are optional by default
                .schema(Some(T::schema()))
                .build(),
		)
	}
}

impl<T: ToSchema> ParameterMetadata for HeaderParam<T> {
	fn parameter_metadata(name: &str, include_in_schema: bool) -> Option<Parameter> {
		if !include_in_schema {
			return None;
		}

		use utoipa::openapi::path::ParameterBuilder;

		Some(
			ParameterBuilder::new()
                .name(name)
                .parameter_in(ParameterLocation::Header)
                .required(Required::False) // Headers are optional by default
                .schema(Some(T::schema()))
                .build(),
		)
	}
}

impl<T: ToSchema> ParameterMetadata for CookieParam<T> {
	fn parameter_metadata(name: &str, include_in_schema: bool) -> Option<Parameter> {
		if !include_in_schema {
			return None;
		}

		use utoipa::openapi::path::ParameterBuilder;

		Some(
			ParameterBuilder::new()
                .name(name)
                .parameter_in(ParameterLocation::Cookie)
                .required(Required::False) // Cookies are optional by default
                .schema(Some(T::schema()))
                .build(),
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_path_parameter_metadata() {
		let param = PathParam::<i64>::parameter_metadata("id", true);
		let param = param.unwrap();
		assert_eq!(param.name, "id");
		assert!(matches!(param.parameter_in, ParameterLocation::Path));
		assert!(matches!(param.required, Required::True));
		assert!(param.schema.is_some());
	}

	#[rstest]
	fn test_path_parameter_hidden() {
		let param = PathParam::<i64>::parameter_metadata("id", false);
		assert!(param.is_none(), "Hidden parameter should return None");
	}

	#[rstest]
	fn test_query_parameter_metadata() {
		let param = QueryParam::<String>::parameter_metadata("search", true);
		let param = param.unwrap();
		assert_eq!(param.name, "search");
		assert!(matches!(param.parameter_in, ParameterLocation::Query));
		assert!(matches!(param.required, Required::False));
	}

	#[rstest]
	fn test_query_parameter_hidden() {
		let param = QueryParam::<String>::parameter_metadata("search", false);
		assert!(param.is_none());
	}

	#[rstest]
	fn test_header_parameter_metadata() {
		let param = HeaderParam::<String>::parameter_metadata("X-API-Key", true);
		let param = param.unwrap();
		assert_eq!(param.name, "X-API-Key");
		assert!(matches!(param.parameter_in, ParameterLocation::Header));
		assert!(matches!(param.required, Required::False));
	}

	#[rstest]
	fn test_header_parameter_hidden() {
		let param = HeaderParam::<String>::parameter_metadata("X-API-Key", false);
		assert!(param.is_none());
	}

	#[rstest]
	fn test_cookie_parameter_metadata() {
		let param = CookieParam::<String>::parameter_metadata("session_id", true);
		let param = param.unwrap();
		assert_eq!(param.name, "session_id");
		assert!(matches!(param.parameter_in, ParameterLocation::Cookie));
		assert!(matches!(param.required, Required::False));
	}

	#[rstest]
	fn test_cookie_parameter_hidden() {
		let param = CookieParam::<String>::parameter_metadata("session_id", false);
		assert!(param.is_none());
	}

	#[rstest]
	fn test_multiple_parameter_types() {
		// Test that we can generate metadata for different types
		let path_int = PathParam::<i64>::parameter_metadata("id", true).unwrap();
		let path_str = PathParam::<String>::parameter_metadata("slug", true).unwrap();
		let query_bool = QueryParam::<bool>::parameter_metadata("active", true).unwrap();

		// Verify schema existence
		assert!(path_int.schema.is_some());
		assert!(path_str.schema.is_some());
		assert!(query_bool.schema.is_some());
	}
}
