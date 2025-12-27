//! Relationship attribute parsing for `#[rel]` macro.
//!
//! This module provides parsing and validation for relationship attributes
//! used in model definitions.

use proc_macro2::Span;
use syn::{Ident, Lit, Path, spanned::Spanned};

/// Relationship types supported by the `#[rel]` attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelationType {
	/// ForeignKey relationship (many-to-one)
	ForeignKey,
	/// OneToOne relationship
	OneToOne,
	/// OneToMany relationship (reverse of ForeignKey)
	OneToMany,
	/// ManyToMany relationship
	ManyToMany,
	/// Polymorphic association
	Polymorphic,
	/// Polymorphic many-to-many relationship
	PolymorphicManyToMany,
	/// Django-style GenericForeignKey (content_type + object_id)
	/// Allows a model to point to any other model via ContentType
	GenericForeignKey,
	/// Django-style GenericRelation (reverse relation for GenericForeignKey)
	/// Provides reverse access from a model to all objects pointing to it
	GenericRelation,
}

impl RelationType {
	/// Parse relationship type from identifier.
	pub(crate) fn from_ident(ident: &Ident) -> Option<Self> {
		match ident.to_string().as_str() {
			"foreign_key" => Some(Self::ForeignKey),
			"one_to_one" => Some(Self::OneToOne),
			"one_to_many" => Some(Self::OneToMany),
			"many_to_many" => Some(Self::ManyToMany),
			"polymorphic" => Some(Self::Polymorphic),
			"polymorphic_many_to_many" => Some(Self::PolymorphicManyToMany),
			"generic_foreign_key" => Some(Self::GenericForeignKey),
			"generic_relation" => Some(Self::GenericRelation),
			_ => None,
		}
	}

	/// Get the string representation of the relationship type.
	pub(crate) fn as_str(&self) -> &'static str {
		match self {
			Self::ForeignKey => "foreign_key",
			Self::OneToOne => "one_to_one",
			Self::OneToMany => "one_to_many",
			Self::ManyToMany => "many_to_many",
			Self::Polymorphic => "polymorphic",
			Self::PolymorphicManyToMany => "polymorphic_many_to_many",
			Self::GenericForeignKey => "generic_foreign_key",
			Self::GenericRelation => "generic_relation",
		}
	}
}

/// Cascade action for foreign key relationships.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CascadeAction {
	/// CASCADE - Delete/update related rows
	Cascade,
	/// SET NULL - Set foreign key to NULL
	SetNull,
	/// SET DEFAULT - Set foreign key to default value
	SetDefault,
	/// RESTRICT - Prevent deletion/update
	Restrict,
	/// NO ACTION - No action (default)
	#[default]
	NoAction,
}

impl CascadeAction {
	/// Parse cascade action from identifier.
	pub(crate) fn from_ident(ident: &Ident) -> Option<Self> {
		match ident.to_string().as_str() {
			"Cascade" => Some(Self::Cascade),
			"SetNull" => Some(Self::SetNull),
			"SetDefault" => Some(Self::SetDefault),
			"Restrict" => Some(Self::Restrict),
			"NoAction" => Some(Self::NoAction),
			_ => None,
		}
	}
}

/// Parsed `#[rel(...)]` attribute.
#[derive(Debug, Clone)]
pub(crate) struct RelAttribute {
	/// Relationship type (foreign_key, one_to_one, many_to_many, etc.)
	pub rel_type: RelationType,
	/// Target model type (e.g., `User`)
	/// NOTE: For ForeignKey/OneToOne, this is extracted from field type (ForeignKeyField<T>).
	/// The 'to' attribute is NOT allowed for these types.
	pub to: Option<Path>,
	/// Target field for foreign key (default: "id")
	pub to_field: Option<String>,
	/// Related name for reverse accessor
	pub related_name: Option<String>,
	/// Cascade action for DELETE
	pub on_delete: CascadeAction,
	/// Cascade action for UPDATE
	pub on_update: CascadeAction,
	/// Whether the field is nullable
	pub null: Option<bool>,
	/// Whether to create a database index
	pub db_index: Option<bool>,
	/// Custom database column name for foreign key field.
	/// If not specified, defaults to `{field_name}_id`.
	pub db_column: Option<String>,
	/// Custom constraint name
	pub db_constraint: Option<String>,
	/// Parent link (for OneToOne inheritance)
	pub parent_link: Option<bool>,
	/// Polymorphic name (for polymorphic associations)
	pub name: Option<String>,
	/// Composite struct for additional through table fields
	pub composite: Option<Path>,
	/// Foreign key field name (for one_to_many)
	pub foreign_key: Option<String>,
	/// Through table name (for many_to_many)
	pub through: Option<String>,
	/// Source field name in through table (for many_to_many)
	pub source_field: Option<String>,
	/// Target field name in through table (for many_to_many)
	pub target_field: Option<String>,
	/// Content type field name for GenericForeignKey (default: "content_type_id")
	pub ct_field: Option<String>,
	/// Object ID field name for GenericForeignKey (default: "object_id")
	pub fk_field: Option<String>,
	/// Span for error reporting
	pub span: Span,
}

impl Default for RelAttribute {
	fn default() -> Self {
		Self {
			rel_type: RelationType::ForeignKey,
			to: None,
			to_field: None,
			related_name: None,
			on_delete: CascadeAction::default(),
			on_update: CascadeAction::default(),
			null: None,
			db_index: None,
			db_column: None,
			db_constraint: None,
			parent_link: None,
			name: None,
			composite: None,
			foreign_key: None,
			through: None,
			source_field: None,
			target_field: None,
			ct_field: None,
			fk_field: None,
			span: Span::call_site(),
		}
	}
}

impl RelAttribute {
	/// Parse `#[rel(...)]` attribute from a syn Attribute.
	pub(crate) fn from_attribute(attr: &syn::Attribute) -> syn::Result<Self> {
		let span = attr.span();
		let mut result = Self {
			span,
			..Default::default()
		};

		// Parse the attribute contents
		attr.parse_nested_meta(|meta| {
			let path = &meta.path;

			// First argument should be the relationship type
			if result.to.is_none()
				&& result.name.is_none()
				&& let Some(ident) = path.get_ident()
				&& let Some(rel_type) = RelationType::from_ident(ident)
			{
				result.rel_type = rel_type;
				return Ok(());
			}

			// Parse named arguments
			if path.is_ident("to") {
				let value: Path = meta.value()?.parse()?;
				result.to = Some(value);
			} else if path.is_ident("to_field") {
				let value = parse_string_value(&meta)?;
				result.to_field = Some(value);
			} else if path.is_ident("related_name") {
				let value = parse_string_value(&meta)?;
				result.related_name = Some(value);
			} else if path.is_ident("on_delete") {
				let ident: Ident = meta.value()?.parse()?;
				result.on_delete = CascadeAction::from_ident(&ident)
					.ok_or_else(|| syn::Error::new(ident.span(), "Invalid on_delete value"))?;
			} else if path.is_ident("on_update") {
				let ident: Ident = meta.value()?.parse()?;
				result.on_update = CascadeAction::from_ident(&ident)
					.ok_or_else(|| syn::Error::new(ident.span(), "Invalid on_update value"))?;
			} else if path.is_ident("null") {
				let value = parse_bool_value(&meta)?;
				result.null = Some(value);
			} else if path.is_ident("db_index") {
				let value = parse_bool_value(&meta)?;
				result.db_index = Some(value);
			} else if path.is_ident("db_column") {
				let value = parse_string_value(&meta)?;
				result.db_column = Some(value);
			} else if path.is_ident("db_constraint") {
				let value = parse_string_value(&meta)?;
				result.db_constraint = Some(value);
			} else if path.is_ident("parent_link") {
				let value = parse_bool_value(&meta)?;
				result.parent_link = Some(value);
			} else if path.is_ident("name") {
				let value = parse_string_value(&meta)?;
				result.name = Some(value);
			} else if path.is_ident("composite") {
				let value: Path = meta.value()?.parse()?;
				result.composite = Some(value);
			} else if path.is_ident("foreign_key") {
				let value = parse_string_value(&meta)?;
				result.foreign_key = Some(value);
			} else if path.is_ident("through") {
				let value = parse_string_value(&meta)?;
				result.through = Some(value);
			} else if path.is_ident("source_field") {
				let value = parse_string_value(&meta)?;
				result.source_field = Some(value);
			} else if path.is_ident("target_field") {
				let value = parse_string_value(&meta)?;
				result.target_field = Some(value);
			} else if path.is_ident("ct_field") {
				let value = parse_string_value(&meta)?;
				result.ct_field = Some(value);
			} else if path.is_ident("fk_field") {
				let value = parse_string_value(&meta)?;
				result.fk_field = Some(value);
			} else {
				return Err(meta.error(format!("Unknown rel attribute: {:?}", path)));
			}

			Ok(())
		})?;

		// Validate required fields based on relationship type
		result.validate()?;

		Ok(result)
	}

	/// Validate the parsed attribute based on relationship type.
	fn validate(&self) -> syn::Result<()> {
		match self.rel_type {
			RelationType::ForeignKey | RelationType::OneToOne => {
				// ForeignKey/OneToOne uses ForeignKeyField<T>/OneToOneField<T> type parameters.
				// The 'to' parameter is NOT allowed - target model is inferred from field type.
				if self.to.is_some() {
					return Err(syn::Error::new(
						self.span,
						format!(
							"#[rel({}, ...)] does not accept 'to' parameter. \
							 Use {}Field<Target> type instead.",
							self.rel_type.as_str(),
							if self.rel_type == RelationType::ForeignKey {
								"ForeignKey"
							} else {
								"OneToOne"
							}
						),
					));
				}
			}
			RelationType::OneToMany => {
				if self.to.is_none() {
					return Err(syn::Error::new(
						self.span,
						"#[rel(one_to_many, ...)] requires 'to' parameter",
					));
				}
			}
			RelationType::ManyToMany => {
				// ManyToMany uses ManyToManyField<Source, Target> type parameters.
				// The 'to' parameter is no longer supported.
				if self.to.is_some() {
					return Err(syn::Error::new(
						self.span,
						"#[rel(many_to_many, ...)] does not accept 'to' parameter. \
						 Use ManyToManyField<Source, Target> type parameters instead.",
					));
				}
			}
			RelationType::Polymorphic | RelationType::PolymorphicManyToMany => {
				if self.name.is_none() {
					return Err(syn::Error::new(
						self.span,
						format!(
							"#[rel({}, ...)] requires 'name' parameter",
							self.rel_type.as_str()
						),
					));
				}
			}
			RelationType::GenericForeignKey => {
				// GenericForeignKey uses GenericForeignKeyField type.
				// ct_field and fk_field are optional (have defaults)
				// No additional required fields - ct_field defaults to "content_type_id"
				// and fk_field defaults to "object_id"
			}
			RelationType::GenericRelation => {
				// GenericRelation requires 'to' to specify which model has the GenericForeignKey
				if self.to.is_none() {
					return Err(syn::Error::new(
						self.span,
						"#[rel(generic_relation, ...)] requires 'to' parameter \
						 to specify the related model with GenericForeignKey",
					));
				}
			}
		}
		Ok(())
	}
}

/// Parse a string value from meta.
fn parse_string_value(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<String> {
	let lit: Lit = meta.value()?.parse()?;
	match lit {
		Lit::Str(s) => Ok(s.value()),
		_ => Err(syn::Error::new(lit.span(), "Expected string literal")),
	}
}

/// Parse a boolean value from meta.
fn parse_bool_value(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<bool> {
	let lit: Lit = meta.value()?.parse()?;
	match lit {
		Lit::Bool(b) => Ok(b.value()),
		_ => Err(syn::Error::new(lit.span(), "Expected boolean literal")),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_relationship_type_from_ident() {
		let ident = Ident::new("foreign_key", Span::call_site());
		assert_eq!(
			RelationType::from_ident(&ident),
			Some(RelationType::ForeignKey)
		);

		let ident = Ident::new("many_to_many", Span::call_site());
		assert_eq!(
			RelationType::from_ident(&ident),
			Some(RelationType::ManyToMany)
		);

		let ident = Ident::new("unknown", Span::call_site());
		assert_eq!(RelationType::from_ident(&ident), None);
	}

	#[test]
	fn test_cascade_action_from_ident() {
		let ident = Ident::new("Cascade", Span::call_site());
		assert_eq!(
			CascadeAction::from_ident(&ident),
			Some(CascadeAction::Cascade)
		);

		let ident = Ident::new("SetNull", Span::call_site());
		assert_eq!(
			CascadeAction::from_ident(&ident),
			Some(CascadeAction::SetNull)
		);
	}
}
