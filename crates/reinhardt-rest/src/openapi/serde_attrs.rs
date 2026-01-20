//! Serde attributes integration for OpenAPI schema generation
//!
//! This module provides utilities for handling common serde attributes
//! when generating OpenAPI schemas.

use super::openapi::SchemaType;
use super::{ObjectBuilder, Schema};
use utoipa::openapi::Type;

/// Field metadata extracted from serde attributes
///
/// This struct represents metadata that can be derived from serde attributes
/// like `#[serde(rename = "...")]`, `#[serde(skip)]`, etc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldMetadata {
	/// Original field name
	pub name: String,

	/// Renamed field name (from `#[serde(rename = "...")]`)
	pub renamed: Option<String>,

	/// Whether field is skipped (from `#[serde(skip)]`)
	pub skip: bool,

	/// Whether field is skipped during serialization (from `#[serde(skip_serializing)]`)
	pub skip_serializing: bool,

	/// Whether field is skipped during deserialization (from `#[serde(skip_deserializing)]`)
	pub skip_deserializing: bool,

	/// Skip serialization condition (from `#[serde(skip_serializing_if = "...")]`)
	pub skip_serializing_if: Option<String>,

	/// Whether field is flattened (from `#[serde(flatten)]`)
	pub flatten: bool,

	/// Default value function (from `#[serde(default)]` or `#[serde(default = "...")]`)
	pub default: Option<String>,
}

impl FieldMetadata {
	/// Create new field metadata with default values
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_rest::openapi::serde_attrs::FieldMetadata;
	///
	/// let metadata = FieldMetadata::new("user_id");
	/// assert_eq!(metadata.name, "user_id");
	/// assert!(!metadata.skip);
	/// ```
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			renamed: None,
			skip: false,
			skip_serializing: false,
			skip_deserializing: false,
			skip_serializing_if: None,
			flatten: false,
			default: None,
		}
	}

	/// Set renamed field name
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_rest::openapi::serde_attrs::FieldMetadata;
	///
	/// let metadata = FieldMetadata::new("user_id")
	///     .with_rename("userId");
	///
	/// assert_eq!(metadata.renamed, Some("userId".to_string()));
	/// ```
	pub fn with_rename(mut self, rename: impl Into<String>) -> Self {
		self.renamed = Some(rename.into());
		self
	}

	/// Mark field as skipped
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_rest::openapi::serde_attrs::FieldMetadata;
	///
	/// let metadata = FieldMetadata::new("internal_field")
	///     .with_skip(true);
	///
	/// assert!(metadata.skip);
	/// ```
	pub fn with_skip(mut self, skip: bool) -> Self {
		self.skip = skip;
		self
	}

	/// Mark field as skipped during serialization
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_rest::openapi::serde_attrs::FieldMetadata;
	///
	/// let metadata = FieldMetadata::new("password")
	///     .with_skip_serializing(true);
	///
	/// assert!(metadata.skip_serializing);
	/// ```
	pub fn with_skip_serializing(mut self, skip: bool) -> Self {
		self.skip_serializing = skip;
		self
	}

	/// Mark field as skipped during deserialization
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_rest::openapi::serde_attrs::FieldMetadata;
	///
	/// let metadata = FieldMetadata::new("computed_field")
	///     .with_skip_deserializing(true);
	///
	/// assert!(metadata.skip_deserializing);
	/// ```
	pub fn with_skip_deserializing(mut self, skip: bool) -> Self {
		self.skip_deserializing = skip;
		self
	}

	/// Set skip serialization condition
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_rest::openapi::serde_attrs::FieldMetadata;
	///
	/// let metadata = FieldMetadata::new("optional_field")
	///     .with_skip_serializing_if("Option::is_none");
	///
	/// assert_eq!(metadata.skip_serializing_if, Some("Option::is_none".to_string()));
	/// ```
	pub fn with_skip_serializing_if(mut self, condition: impl Into<String>) -> Self {
		self.skip_serializing_if = Some(condition.into());
		self
	}

	/// Mark field as flattened
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_rest::openapi::serde_attrs::FieldMetadata;
	///
	/// let metadata = FieldMetadata::new("metadata")
	///     .with_flatten(true);
	///
	/// assert!(metadata.flatten);
	/// ```
	pub fn with_flatten(mut self, flatten: bool) -> Self {
		self.flatten = flatten;
		self
	}

	/// Set default value function
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_rest::openapi::serde_attrs::FieldMetadata;
	///
	/// let metadata = FieldMetadata::new("count")
	///     .with_default("default_count");
	///
	/// assert_eq!(metadata.default, Some("default_count".to_string()));
	/// ```
	pub fn with_default(mut self, default: impl Into<String>) -> Self {
		self.default = Some(default.into());
		self
	}

	/// Get the effective field name (renamed or original)
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_rest::openapi::serde_attrs::FieldMetadata;
	///
	/// let metadata = FieldMetadata::new("user_id")
	///     .with_rename("userId");
	///
	/// assert_eq!(metadata.effective_name(), "userId");
	/// ```
	pub fn effective_name(&self) -> &str {
		self.renamed.as_deref().unwrap_or(&self.name)
	}

	/// Check if field should be included in schema
	///
	/// A field should be excluded if it's marked with `#[serde(skip)]`.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_rest::openapi::serde_attrs::FieldMetadata;
	///
	/// let normal = FieldMetadata::new("id");
	/// assert!(normal.should_include());
	///
	/// let skipped = FieldMetadata::new("internal")
	///     .with_skip(true);
	/// assert!(!skipped.should_include());
	/// ```
	pub fn should_include(&self) -> bool {
		!self.skip
	}

	/// Check if field is required
	///
	/// A field is not required if it has a default value.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_rest::openapi::serde_attrs::FieldMetadata;
	///
	/// let required = FieldMetadata::new("id");
	/// assert!(required.is_required());
	///
	/// let optional = FieldMetadata::new("count")
	///     .with_default("0");
	/// assert!(!optional.is_required());
	/// ```
	pub fn is_required(&self) -> bool {
		self.default.is_none()
	}
}

/// Schema builder with serde attribute support
///
/// Provides utilities for building OpenAPI schemas that respect serde attributes.
///
/// # Example
///
/// ```rust
/// use reinhardt_rest::openapi::serde_attrs::{FieldMetadata, SchemaBuilderExt};
/// use reinhardt_rest::openapi::{Schema, SchemaExt};
///
/// let fields = vec![
///     (
///         FieldMetadata::new("user_id").with_rename("userId"),
///         Schema::integer(),
///     ),
///     (
///         FieldMetadata::new("name"),
///         Schema::string(),
///     ),
/// ];
///
/// let schema = SchemaBuilderExt::build_object_from_fields(fields);
/// ```
pub struct SchemaBuilderExt;

impl SchemaBuilderExt {
	/// Build an object schema from fields with metadata
	///
	/// This function creates an OpenAPI object schema from a list of fields
	/// with their metadata, respecting serde attributes like rename and skip.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_rest::openapi::serde_attrs::{FieldMetadata, SchemaBuilderExt};
	/// use reinhardt_rest::openapi::{Schema, SchemaExt};
	///
	/// let fields = vec![
	///     (FieldMetadata::new("id"), Schema::integer()),
	///     (FieldMetadata::new("name"), Schema::string()),
	///     (FieldMetadata::new("secret").with_skip(true), Schema::string()),
	/// ];
	///
	/// let schema = SchemaBuilderExt::build_object_from_fields(fields);
	///
	/// match schema {
	///     Schema::Object(obj) => {
	///         // "secret" should be excluded
	///         assert_eq!(obj.properties.len(), 2);
	///         assert!(obj.properties.contains_key("id"));
	///         assert!(obj.properties.contains_key("name"));
	///         assert!(!obj.properties.contains_key("secret"));
	///     }
	///     _ => panic!("Expected Object schema"),
	/// }
	/// ```
	pub fn build_object_from_fields(fields: Vec<(FieldMetadata, Schema)>) -> Schema {
		let mut builder = ObjectBuilder::new().schema_type(SchemaType::Type(Type::Object));

		let mut flattened_schemas = Vec::new();

		for (metadata, schema) in fields {
			if !metadata.should_include() {
				continue;
			}

			if metadata.flatten {
				// Collect flattened schemas for allOf
				flattened_schemas.push(schema);
			} else {
				let field_name = metadata.effective_name();
				builder = builder.property(field_name, schema);

				if metadata.is_required() {
					builder = builder.required(field_name);
				}
			}
		}

		let base_schema = Schema::Object(builder.build());

		// If there are flattened schemas, wrap in AllOf
		if !flattened_schemas.is_empty() {
			let mut all_of_schemas = vec![utoipa::openapi::RefOr::T(base_schema)];
			all_of_schemas.extend(flattened_schemas.into_iter().map(utoipa::openapi::RefOr::T));

			let mut all_of = utoipa::openapi::schema::AllOf::new();
			all_of.items = all_of_schemas;

			Schema::AllOf(all_of)
		} else {
			base_schema
		}
	}

	/// Apply rename transformation to field names
	///
	/// This function applies a rename transformation (like `rename_all = "camelCase"`)
	/// to all field names.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_rest::openapi::serde_attrs::{FieldMetadata, SchemaBuilderExt, RenameAll};
	///
	/// let metadata = vec![
	///     FieldMetadata::new("user_id"),
	///     FieldMetadata::new("first_name"),
	/// ];
	///
	/// let renamed = SchemaBuilderExt::apply_rename_all(metadata, RenameAll::CamelCase);
	///
	/// assert_eq!(renamed[0].effective_name(), "userId");
	/// assert_eq!(renamed[1].effective_name(), "firstName");
	/// ```
	pub fn apply_rename_all(
		mut metadata: Vec<FieldMetadata>,
		rename_all: RenameAll,
	) -> Vec<FieldMetadata> {
		for meta in &mut metadata {
			if meta.renamed.is_none() {
				meta.renamed = Some(rename_all.transform(&meta.name));
			}
		}
		metadata
	}
}

/// Rename transformation strategy
///
/// Corresponds to serde's `#[serde(rename_all = "...")]` attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameAll {
	/// lowercase
	LowerCase,
	/// UPPERCASE
	UpperCase,
	/// PascalCase
	PascalCase,
	/// camelCase
	CamelCase,
	/// snake_case
	SnakeCase,
	/// SCREAMING_SNAKE_CASE
	ScreamingSnakeCase,
	/// kebab-case
	KebabCase,
	/// SCREAMING-KEBAB-CASE
	ScreamingKebabCase,
}

impl RenameAll {
	/// Transform a field name according to the rename strategy
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_rest::openapi::serde_attrs::RenameAll;
	///
	/// assert_eq!(RenameAll::CamelCase.transform("user_id"), "userId");
	/// assert_eq!(RenameAll::PascalCase.transform("user_id"), "UserId");
	/// assert_eq!(RenameAll::KebabCase.transform("user_id"), "user-id");
	/// ```
	pub fn transform(&self, input: &str) -> String {
		match self {
			RenameAll::LowerCase => input.to_lowercase(),
			RenameAll::UpperCase => input.to_uppercase(),
			RenameAll::PascalCase => Self::to_pascal_case(input),
			RenameAll::CamelCase => Self::to_camel_case(input),
			RenameAll::SnakeCase => input.to_string(), // Already snake_case in Rust
			RenameAll::ScreamingSnakeCase => input.to_uppercase(),
			RenameAll::KebabCase => input.replace('_', "-"),
			RenameAll::ScreamingKebabCase => input.to_uppercase().replace('_', "-"),
		}
	}

	fn to_pascal_case(input: &str) -> String {
		input
			.split('_')
			.map(|word| {
				let mut chars = word.chars();
				match chars.next() {
					None => String::new(),
					Some(first) => first.to_uppercase().chain(chars).collect(),
				}
			})
			.collect()
	}

	fn to_camel_case(input: &str) -> String {
		let pascal = Self::to_pascal_case(input);
		let mut chars = pascal.chars();
		match chars.next() {
			None => String::new(),
			Some(first) => first.to_lowercase().chain(chars).collect(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::openapi::SchemaExt;

	#[test]
	fn test_field_metadata_new() {
		let metadata = FieldMetadata::new("user_id");
		assert_eq!(metadata.name, "user_id");
		assert!(!metadata.skip);
		assert!(metadata.should_include());
	}

	#[test]
	fn test_field_metadata_rename() {
		let metadata = FieldMetadata::new("user_id").with_rename("userId");
		assert_eq!(metadata.effective_name(), "userId");
	}

	#[test]
	fn test_field_metadata_skip() {
		let metadata = FieldMetadata::new("secret").with_skip(true);
		assert!(!metadata.should_include());
	}

	#[test]
	fn test_field_metadata_default() {
		let metadata = FieldMetadata::new("count").with_default("0");
		assert!(!metadata.is_required());
	}

	#[test]
	fn test_build_object_from_fields() {
		let fields = vec![
			(FieldMetadata::new("id"), Schema::integer()),
			(FieldMetadata::new("name"), Schema::string()),
			(
				FieldMetadata::new("secret").with_skip(true),
				Schema::string(),
			),
		];

		let schema = SchemaBuilderExt::build_object_from_fields(fields);

		match schema {
			Schema::Object(obj) => {
				assert_eq!(obj.properties.len(), 2);
				assert!(obj.properties.contains_key("id"));
				assert!(obj.properties.contains_key("name"));
				assert!(!obj.properties.contains_key("secret"));
			}
			_ => panic!("Expected Object schema"),
		}
	}

	#[test]
	fn test_build_object_with_renamed_fields() {
		let fields = vec![
			(
				FieldMetadata::new("user_id").with_rename("userId"),
				Schema::integer(),
			),
			(
				FieldMetadata::new("first_name").with_rename("firstName"),
				Schema::string(),
			),
		];

		let schema = SchemaBuilderExt::build_object_from_fields(fields);

		match schema {
			Schema::Object(obj) => {
				assert!(obj.properties.contains_key("userId"));
				assert!(obj.properties.contains_key("firstName"));
				assert!(!obj.properties.contains_key("user_id"));
				assert!(!obj.properties.contains_key("first_name"));
			}
			_ => panic!("Expected Object schema"),
		}
	}

	#[test]
	fn test_build_object_with_optional_fields() {
		let fields = vec![
			(FieldMetadata::new("id"), Schema::integer()),
			(
				FieldMetadata::new("count").with_default("0"),
				Schema::integer(),
			),
		];

		let schema = SchemaBuilderExt::build_object_from_fields(fields);

		match schema {
			Schema::Object(obj) => {
				assert!(obj.required.contains(&"id".to_string()));
				assert!(!obj.required.contains(&"count".to_string()));
			}
			_ => panic!("Expected Object schema"),
		}
	}

	#[test]
	fn test_build_object_with_flatten() {
		let fields = vec![
			(FieldMetadata::new("id"), Schema::integer()),
			(
				FieldMetadata::new("metadata").with_flatten(true),
				Schema::object_with_properties(vec![("key", Schema::string())], vec!["key"]),
			),
		];

		let schema = SchemaBuilderExt::build_object_from_fields(fields);

		match schema {
			Schema::AllOf(all_of) => {
				// Should use AllOf for flattened fields
				assert_eq!(all_of.items.len(), 2);
			}
			_ => panic!("Expected AllOf schema"),
		}
	}

	#[test]
	fn test_rename_all_camel_case() {
		assert_eq!(RenameAll::CamelCase.transform("user_id"), "userId");
		assert_eq!(RenameAll::CamelCase.transform("first_name"), "firstName");
	}

	#[test]
	fn test_rename_all_pascal_case() {
		assert_eq!(RenameAll::PascalCase.transform("user_id"), "UserId");
		assert_eq!(RenameAll::PascalCase.transform("first_name"), "FirstName");
	}

	#[test]
	fn test_rename_all_kebab_case() {
		assert_eq!(RenameAll::KebabCase.transform("user_id"), "user-id");
		assert_eq!(RenameAll::KebabCase.transform("first_name"), "first-name");
	}

	#[test]
	fn test_apply_rename_all() {
		let metadata = vec![
			FieldMetadata::new("user_id"),
			FieldMetadata::new("first_name"),
		];

		let renamed = SchemaBuilderExt::apply_rename_all(metadata, RenameAll::CamelCase);

		assert_eq!(renamed[0].effective_name(), "userId");
		assert_eq!(renamed[1].effective_name(), "firstName");
	}

	#[test]
	fn test_apply_rename_all_preserves_explicit_rename() {
		let metadata = vec![
			FieldMetadata::new("user_id").with_rename("customName"),
			FieldMetadata::new("first_name"),
		];

		let renamed = SchemaBuilderExt::apply_rename_all(metadata, RenameAll::CamelCase);

		// Explicit rename should be preserved
		assert_eq!(renamed[0].effective_name(), "customName");
		// Auto rename should be applied
		assert_eq!(renamed[1].effective_name(), "firstName");
	}
}
