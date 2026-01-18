//! Advanced enum schema generation
//!
//! This module provides support for generating OpenAPI schemas for Rust enums
//! with various serde tagging strategies.

use crate::openapi::{ObjectBuilder, RefOr, Schema, SchemaExt, SchemaType};
use utoipa::openapi::Type;

/// Enum tagging strategy
///
/// Corresponds to serde's enum representation attributes:
/// - `#[serde(tag = "type")]` - Internally tagged
/// - `#[serde(tag = "type", content = "value")]` - Adjacently tagged
/// - `#[serde(untagged)]` - Untagged
/// - No attribute - Externally tagged (default)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnumTagging {
	/// Externally tagged (default): `{"Variant": {...}}`
	External,

	/// Internally tagged: `{"type": "Variant", ...fields}`
	///
	/// # Example
	///
	/// ```rust,no_run
	/// # use serde::Serialize;
	/// #[derive(Serialize)]
	/// #[serde(tag = "type")]
	/// enum Message {
	///     Text { content: String },
	///     Image { url: String },
	/// }
	/// ```
	Internal {
		/// The tag field name (e.g., "type")
		tag: String,
	},

	/// Adjacently tagged: `{"tag": "Variant", "content": {...}}`
	///
	/// # Example
	///
	/// ```rust,no_run
	/// # use serde::Serialize;
	/// #[derive(Serialize)]
	/// #[serde(tag = "tag", content = "content")]
	/// enum Message {
	///     Text { content: String },
	///     Image { url: String },
	/// }
	/// ```
	Adjacent {
		/// The tag field name (e.g., "tag")
		tag: String,
		/// The content field name (e.g., "content")
		content: String,
	},

	/// Untagged: no discriminator, tries each variant in order
	///
	/// # Example
	///
	/// ```rust,no_run
	/// # use serde::Serialize;
	/// #[derive(Serialize)]
	/// #[serde(untagged)]
	/// enum Value {
	///     String(String),
	///     Number(i32),
	/// }
	/// ```
	Untagged,
}

/// Builder for enum schemas
///
/// Provides a fluent API for constructing OpenAPI schemas for Rust enums
/// with different tagging strategies.
///
/// # Example
///
/// ```rust
/// use reinhardt_rest::openapi::enum_schema::{EnumSchemaBuilder, EnumTagging};
/// use reinhardt_rest::openapi::{Schema, SchemaExt};
///
/// // Internally tagged enum
/// let schema = EnumSchemaBuilder::new("Message")
///     .tagging(EnumTagging::Internal {
///         tag: "type".to_string(),
///     })
///     .variant("Text", Schema::object_with_properties(
///         vec![("content", Schema::string())],
///         vec!["content"],
///     ))
///     .variant("Image", Schema::object_with_properties(
///         vec![("url", Schema::string())],
///         vec!["url"],
///     ))
///     .build();
/// ```
pub struct EnumSchemaBuilder {
	name: String,
	tagging: EnumTagging,
	variants: Vec<(String, Schema)>,
	description: Option<String>,
}

impl EnumSchemaBuilder {
	/// Create a new enum schema builder
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_rest::openapi::enum_schema::EnumSchemaBuilder;
	///
	/// let builder = EnumSchemaBuilder::new("Status");
	/// ```
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			tagging: EnumTagging::External,
			variants: Vec::new(),
			description: None,
		}
	}

	/// Set the tagging strategy
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_rest::openapi::enum_schema::{EnumSchemaBuilder, EnumTagging};
	///
	/// let builder = EnumSchemaBuilder::new("Status")
	///     .tagging(EnumTagging::Internal {
	///         tag: "type".to_string(),
	///     });
	/// ```
	pub fn tagging(mut self, tagging: EnumTagging) -> Self {
		self.tagging = tagging;
		self
	}

	/// Add a variant to the enum
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_rest::openapi::enum_schema::EnumSchemaBuilder;
	/// use reinhardt_rest::openapi::{Schema, SchemaExt};
	///
	/// let builder = EnumSchemaBuilder::new("Status")
	///     .variant("Active", Schema::object())
	///     .variant("Inactive", Schema::object());
	/// ```
	pub fn variant(mut self, name: impl Into<String>, schema: Schema) -> Self {
		self.variants.push((name.into(), schema));
		self
	}

	/// Set the enum description
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_rest::openapi::enum_schema::EnumSchemaBuilder;
	///
	/// let builder = EnumSchemaBuilder::new("Status")
	///     .description("User status");
	/// ```
	pub fn description(mut self, description: impl Into<String>) -> Self {
		self.description = Some(description.into());
		self
	}

	/// Build the OpenAPI schema
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_rest::openapi::enum_schema::EnumSchemaBuilder;
	/// use reinhardt_rest::openapi::{Schema, SchemaExt};
	///
	/// let schema = EnumSchemaBuilder::new("Status")
	///     .variant("Active", Schema::object())
	///     .variant("Inactive", Schema::object())
	///     .build();
	/// ```
	pub fn build(self) -> Schema {
		match self.tagging.clone() {
			EnumTagging::External => self.build_external(),
			EnumTagging::Internal { tag } => self.build_internal(&tag),
			EnumTagging::Adjacent { tag, content } => self.build_adjacent(&tag, &content),
			EnumTagging::Untagged => self.build_untagged(),
		}
	}

	fn build_external(self) -> Schema {
		// External tagging: oneOf with object for each variant
		// Each variant is an object with single property (variant name)
		let variant_schemas: Vec<RefOr<Schema>> = self
			.variants
			.into_iter()
			.map(|(name, schema)| {
				RefOr::T(Schema::Object(
					ObjectBuilder::new()
						.schema_type(SchemaType::Type(Type::Object))
						.property(name, schema)
						.build(),
				))
			})
			.collect();

		let mut one_of = utoipa::openapi::schema::OneOf::new();
		one_of.items = variant_schemas;
		one_of.title = Some(self.name);

		if let Some(desc) = self.description {
			one_of.description = Some(desc);
		}

		Schema::OneOf(one_of)
	}

	fn build_internal(self, tag: &str) -> Schema {
		// Internal tagging: oneOf with discriminator
		// Each variant includes the tag field
		let variant_schemas: Vec<RefOr<Schema>> = self
			.variants
			.into_iter()
			.map(|(name, schema)| {
				let mut properties = vec![(tag.to_string(), Schema::string())];
				let mut required = vec![tag.to_string()];

				// Merge variant schema properties
				if let Schema::Object(obj) = schema {
					for (prop_name, prop_schema) in obj.properties {
						properties.push((prop_name.clone(), prop_schema.into()));
						if obj.required.contains(&prop_name) {
							required.push(prop_name);
						}
					}

					let mut builder = ObjectBuilder::new()
						.schema_type(SchemaType::Type(Type::Object))
						.property(tag, Schema::string());

					for (prop_name, prop_schema) in properties.into_iter().skip(1) {
						builder = builder.property(prop_name, prop_schema);
					}

					for req in required {
						builder = builder.required(req);
					}

					// Add const constraint for tag
					builder = builder.property(
						tag,
						Schema::Object(
							ObjectBuilder::new()
								.schema_type(SchemaType::Type(Type::String))
								.enum_values(Some(vec![serde_json::Value::String(name)]))
								.build(),
						),
					);

					RefOr::T(Schema::Object(builder.build()))
				} else {
					RefOr::T(Schema::Object(
						ObjectBuilder::new()
							.schema_type(SchemaType::Type(Type::Object))
							.property(
								tag,
								Schema::Object(
									ObjectBuilder::new()
										.schema_type(SchemaType::Type(Type::String))
										.enum_values(Some(vec![serde_json::Value::String(name)]))
										.build(),
								),
							)
							.required(tag)
							.build(),
					))
				}
			})
			.collect();

		let mut one_of = utoipa::openapi::schema::OneOf::new();
		one_of.items = variant_schemas;
		one_of.title = Some(self.name.clone());
		one_of.discriminator = Some(utoipa::openapi::schema::Discriminator::new(tag));

		if let Some(desc) = self.description {
			one_of.description = Some(desc);
		}

		Schema::OneOf(one_of)
	}

	fn build_adjacent(self, tag: &str, content: &str) -> Schema {
		// Adjacent tagging: oneOf with tag and content fields
		let variant_schemas: Vec<RefOr<Schema>> = self
			.variants
			.into_iter()
			.map(|(name, schema)| {
				RefOr::T(Schema::Object(
					ObjectBuilder::new()
						.schema_type(SchemaType::Type(Type::Object))
						.property(
							tag,
							Schema::Object(
								ObjectBuilder::new()
									.schema_type(SchemaType::Type(Type::String))
									.enum_values(Some(vec![serde_json::Value::String(name)]))
									.build(),
							),
						)
						.property(content, schema)
						.required(tag)
						.required(content)
						.build(),
				))
			})
			.collect();

		let mut one_of = utoipa::openapi::schema::OneOf::new();
		one_of.items = variant_schemas;
		one_of.title = Some(self.name);
		one_of.discriminator = Some(utoipa::openapi::schema::Discriminator::new(tag));

		if let Some(desc) = self.description {
			one_of.description = Some(desc);
		}

		Schema::OneOf(one_of)
	}

	fn build_untagged(self) -> Schema {
		// Untagged: oneOf without discriminator
		let variant_schemas: Vec<RefOr<Schema>> = self
			.variants
			.into_iter()
			.map(|(_, schema)| RefOr::T(schema))
			.collect();

		let mut one_of = utoipa::openapi::schema::OneOf::new();
		one_of.items = variant_schemas;
		one_of.title = Some(self.name);

		if let Some(desc) = self.description {
			one_of.description = Some(desc);
		}

		Schema::OneOf(one_of)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_external_tagging() {
		let schema = EnumSchemaBuilder::new("Message")
			.variant("Text", Schema::string())
			.variant("Image", Schema::string())
			.build();

		match schema {
			Schema::OneOf(one_of) => {
				assert_eq!(one_of.items.len(), 2);
			}
			_ => panic!("Expected OneOf schema"),
		}
	}

	#[test]
	fn test_internal_tagging() {
		let schema = EnumSchemaBuilder::new("Message")
			.tagging(EnumTagging::Internal {
				tag: "type".to_string(),
			})
			.variant(
				"Text",
				Schema::object_with_properties(
					vec![("content", Schema::string())],
					vec!["content"],
				),
			)
			.build();

		match schema {
			Schema::OneOf(one_of) => {
				assert!(one_of.discriminator.is_some());

				let discriminator = one_of.discriminator.unwrap();
				assert_eq!(discriminator.property_name, "type");
			}
			_ => panic!("Expected OneOf schema"),
		}
	}

	#[test]
	fn test_adjacent_tagging() {
		let schema = EnumSchemaBuilder::new("Message")
			.tagging(EnumTagging::Adjacent {
				tag: "tag".to_string(),
				content: "content".to_string(),
			})
			.variant("Text", Schema::string())
			.build();

		match schema {
			Schema::OneOf(one_of) => {
				assert!(one_of.discriminator.is_some());

				let discriminator = one_of.discriminator.unwrap();
				assert_eq!(discriminator.property_name, "tag");
			}
			_ => panic!("Expected OneOf schema"),
		}
	}

	#[test]
	fn test_untagged() {
		let schema = EnumSchemaBuilder::new("Value")
			.tagging(EnumTagging::Untagged)
			.variant("String", Schema::string())
			.variant("Number", Schema::integer())
			.build();

		match schema {
			Schema::OneOf(one_of) => {
				assert!(one_of.discriminator.is_none());
			}
			_ => panic!("Expected OneOf schema"),
		}
	}

	#[test]
	fn test_with_description() {
		let schema = EnumSchemaBuilder::new("Status")
			.description("User status")
			.variant("Active", Schema::object())
			.build();

		match schema {
			Schema::OneOf(one_of) => {
				assert_eq!(one_of.description, Some("User status".to_string()));
			}
			_ => panic!("Expected OneOf schema"),
		}
	}

	#[test]
	fn test_multiple_variants() {
		let schema = EnumSchemaBuilder::new("Color")
			.variant("Red", Schema::object())
			.variant("Green", Schema::object())
			.variant("Blue", Schema::object())
			.build();

		match schema {
			Schema::OneOf(one_of) => {
				assert_eq!(one_of.items.len(), 3);
			}
			_ => panic!("Expected OneOf schema"),
		}
	}

	#[test]
	fn test_internal_tagging_preserves_variant_properties() {
		let schema = EnumSchemaBuilder::new("Message")
			.tagging(EnumTagging::Internal {
				tag: "type".to_string(),
			})
			.variant(
				"Text",
				Schema::object_with_properties(
					vec![("content", Schema::string()), ("author", Schema::string())],
					vec!["content", "author"],
				),
			)
			.build();

		match schema {
			Schema::OneOf(one_of) => {
				assert_eq!(one_of.items.len(), 1);

				match &one_of.items[0] {
					RefOr::T(Schema::Object(variant_obj)) => {
						// Should have type, content, and author properties
						assert!(variant_obj.properties.contains_key("type"));
						assert!(variant_obj.properties.contains_key("content"));
						assert!(variant_obj.properties.contains_key("author"));

						// All should be required
						assert!(variant_obj.required.contains(&"type".to_string()));
						assert!(variant_obj.required.contains(&"content".to_string()));
						assert!(variant_obj.required.contains(&"author".to_string()));
					}
					_ => panic!("Expected T(Object) variant"),
				}
			}
			_ => panic!("Expected OneOf schema"),
		}
	}
}
