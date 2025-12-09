//! Model derive macro for automatic ORM model registration
//!
//! Provides automatic `Model` trait implementation and registration to the global ModelRegistry.

use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, GenericArgument, PathArguments, Result, Type, parse_quote};
use syn::{Ident, LitStr, bracketed, parenthesized};

use crate::rel::RelAttribute;

/// Constraint specification from #[model(constraints = [...])]
#[derive(Debug, Clone)]
enum ConstraintSpec {
	/// unique(fields = [...], name = "...", condition = "...")
	Unique {
		fields: Vec<String>,
		name: Option<String>,
		condition: Option<String>,
	},
}

/// Parsed model attributes (intermediate representation)
struct ModelAttributesParsed {
	app_label: Option<String>,
	table_name: Option<String>,
	constraints: Option<Vec<ConstraintSpec>>,
	unique_together: Vec<Vec<String>>, // Multiple Django-style unique_together constraints
	#[cfg(feature = "db-sqlite")]
	strict: Option<bool>,
	#[cfg(feature = "db-sqlite")]
	without_rowid: Option<bool>,
}

/// Model configuration from #[model(...)] attribute
#[derive(Debug, Clone)]
#[allow(dead_code)] // Phase 3.2 fields (strict, without_rowid) parsed but not yet used
struct ModelConfig {
	app_label: String,
	table_name: String,
	constraints: Vec<ConstraintSpec>,

	// Phase 3.2: Table-level attributes (SQLite)
	#[cfg(feature = "db-sqlite")]
	strict: Option<bool>,
	#[cfg(feature = "db-sqlite")]
	without_rowid: Option<bool>,
}

impl ModelConfig {
	/// Parse #[model(...)] attribute
	fn from_attrs(attrs: &[syn::Attribute], struct_name: &syn::Ident) -> Result<Self> {
		let mut app_label = None;
		let mut table_name = None;
		let mut constraints = Vec::new();

		#[cfg(feature = "db-sqlite")]
		let mut strict = None;
		#[cfg(feature = "db-sqlite")]
		let mut without_rowid = None;

		for attr in attrs {
			// Accept both #[model(...)] and #[model_config(...)] helper attributes
			if !attr.path().is_ident("model") && !attr.path().is_ident("model_config") {
				continue;
			}

			// Use custom parser for all model attributes
			let model_attr = attr
				.parse_args_with(|input: syn::parse::ParseStream| {
					Self::parse_model_attributes(input)
				})
				.map_err(|e| {
					syn::Error::new_spanned(attr, format!("parse_args_with failed: {}", e))
				})?;

			if let Some(c) = model_attr.constraints {
				constraints = c;
			}
			// Convert each unique_together to ConstraintSpec::Unique
			for fields in model_attr.unique_together {
				constraints.push(ConstraintSpec::Unique {
					fields,
					name: None, // Auto-generate name
					condition: None,
				});
			}
			if let Some(al) = model_attr.app_label {
				app_label = Some(al);
			}
			if let Some(tn) = model_attr.table_name {
				table_name = Some(tn);
			}
			#[cfg(feature = "db-sqlite")]
			{
				if let Some(s) = model_attr.strict {
					strict = Some(s);
				}
				if let Some(wr) = model_attr.without_rowid {
					without_rowid = Some(wr);
				}
			}
		}

		let table_name = table_name.ok_or_else(|| {
			syn::Error::new_spanned(
				struct_name,
				"table_name attribute is required in #[model(...)]",
			)
		})?;

		Ok(Self {
			app_label: app_label.unwrap_or_else(|| "default".to_string()),
			table_name,
			constraints,
			#[cfg(feature = "db-sqlite")]
			strict,
			#[cfg(feature = "db-sqlite")]
			without_rowid,
		})
	}

	/// Parse all model attributes using custom parser
	fn parse_model_attributes(input: syn::parse::ParseStream) -> Result<ModelAttributesParsed> {
		use syn::Token;

		let mut app_label = None;
		let mut table_name = None;
		let mut constraints = None;
		let mut unique_together = Vec::new();
		#[cfg(feature = "db-sqlite")]
		let mut strict = None;
		#[cfg(feature = "db-sqlite")]
		let mut without_rowid = None;

		while !input.is_empty() {
			let ident: Ident = input.parse()?;
			input.parse::<Token![=]>()?;

			if ident == "app_label" {
				let value: LitStr = input.parse()?;
				app_label = Some(value.value());
			} else if ident == "table_name" {
				let value: LitStr = input.parse()?;
				table_name = Some(value.value());
			} else if ident == "unique_together" {
				// Tuple syntax: unique_together = ("field1", "field2")
				use syn::punctuated::Punctuated;
				let content;
				parenthesized!(content in input);
				let fields: Punctuated<LitStr, Token![,]> =
					content.call(Punctuated::parse_terminated)?;
				unique_together.push(fields.iter().map(|lit| lit.value()).collect());
			} else if ident == "constraints" {
				// Parse array: [unique(...), ...]
				let array_content;
				bracketed!(array_content in input);

				let mut specs = Vec::new();
				while !array_content.is_empty() {
					specs.push(Self::parse_constraint(&array_content)?);

					if array_content.peek(Token![,]) {
						array_content.parse::<Token![,]>()?;
					} else {
						break;
					}
				}
				constraints = Some(specs);
			} else if ident == "strict" {
				#[cfg(feature = "db-sqlite")]
				{
					let value: syn::LitBool = input.parse()?;
					strict = Some(value.value);
				}
				#[cfg(not(feature = "db-sqlite"))]
				{
					// Just skip the value
					let _value: syn::LitBool = input.parse()?;
				}
			} else if ident == "without_rowid" {
				#[cfg(feature = "db-sqlite")]
				{
					let value: syn::LitBool = input.parse()?;
					without_rowid = Some(value.value);
				}
				#[cfg(not(feature = "db-sqlite"))]
				{
					// Just skip the value
					let _value: syn::LitBool = input.parse()?;
				}
			} else {
				return Err(syn::Error::new_spanned(
					&ident,
					format!("Unknown model attribute: {}", ident),
				));
			}

			// Parse optional comma
			if input.peek(Token![,]) {
				input.parse::<Token![,]>()?;
			} else {
				break;
			}
		}

		Ok(ModelAttributesParsed {
			app_label,
			table_name,
			constraints,
			unique_together,
			#[cfg(feature = "db-sqlite")]
			strict,
			#[cfg(feature = "db-sqlite")]
			without_rowid,
		})
	}

	/// Parse constraint specification: unique(fields = [...], name = "...", condition = "...")
	fn parse_constraint(input: syn::parse::ParseStream) -> Result<ConstraintSpec> {
		use syn::Token;
		use syn::punctuated::Punctuated;

		// Define custom keyword for "unique"
		mod kw {
			syn::custom_keyword!(unique);
		}

		// Parse constraint type using custom keyword
		let _unique_keyword = input.parse::<kw::unique>()?;

		// Parse parentheses with parameters
		let content;
		parenthesized!(content in input);

		let mut fields = None;
		let mut name = None;
		let mut condition = None;

		// Parse named parameters (fields = [...], name = "...", condition = "...")
		loop {
			if content.is_empty() {
				break;
			}

			let param_name: Ident = content.parse()?;
			content.parse::<Token![=]>()?;

			if param_name == "fields" {
				// Parse array using Punctuated for proper comma handling
				let array_content;
				bracketed!(array_content in content);

				// Use Punctuated::parse_terminated for robust comma-separated parsing
				let field_literals: Punctuated<LitStr, Token![,]> =
					array_content.call(Punctuated::parse_terminated)?;

				fields = Some(field_literals.iter().map(|lit| lit.value()).collect());
			} else if param_name == "name" {
				// Parse string: "constraint_name"
				let value: LitStr = content.parse()?;
				name = Some(value.value());
			} else if param_name == "condition" {
				// Parse string: "WHERE clause"
				let value: LitStr = content.parse()?;
				condition = Some(value.value());
			} else {
				return Err(syn::Error::new_spanned(
					param_name,
					"Unknown parameter. Supported: fields, name, condition",
				));
			}

			// Parse optional comma between parameters
			if content.peek(Token![,]) {
				content.parse::<Token![,]>()?;
			} else {
				break;
			}
		}

		// fields is required
		let fields = fields.ok_or_else(|| {
			syn::Error::new(
				proc_macro2::Span::call_site(),
				"unique constraint requires 'fields' parameter",
			)
		})?;

		Ok(ConstraintSpec::Unique {
			fields,
			name,
			condition,
		})
	}
}

/// Foreign key specification
#[derive(Debug, Clone)]
enum ForeignKeySpec {
	/// Type directly: #[field(foreign_key = User)]
	Type(syn::Type),
	/// app_label.model_name format: #[field(foreign_key = "users.User")]
	AppModel {
		app_label: String,
		model_name: String,
	},
}

/// Storage strategy for PostgreSQL columns
#[cfg(feature = "db-postgres")]
#[derive(Debug, Clone)]
enum StorageStrategy {
	Plain,
	Extended,
	External,
	Main,
}

/// Compression method for PostgreSQL columns
#[cfg(feature = "db-postgres")]
#[derive(Debug, Clone)]
enum CompressionMethod {
	Pglz,
	Lz4,
}

/// Field configuration from #[field(...)] attribute
#[derive(Debug, Clone, Default)]
struct FieldConfig {
	primary_key: bool,
	max_length: Option<u64>,
	null: Option<bool>,
	blank: Option<bool>,
	unique: Option<bool>,
	default: Option<syn::Expr>, // Changed from String to Expr to support bool, int, etc.
	db_column: Option<String>,
	editable: Option<bool>,
	index: Option<bool>,
	check: Option<String>,
	// Validator flags
	email: Option<bool>,
	url: Option<bool>,
	min_length: Option<u64>,
	min_value: Option<i64>,
	max_value: Option<i64>,
	// Time-related fields
	auto_now_add: Option<bool>,
	auto_now: Option<bool>,
	// Relationship fields
	foreign_key: Option<ForeignKeySpec>,

	// Generated Columns (all DBMS)
	generated: Option<String>,
	generated_stored: Option<bool>,
	#[cfg(any(feature = "db-mysql", feature = "db-sqlite"))]
	generated_virtual: Option<bool>,

	// Identity/Auto-increment
	#[cfg(feature = "db-postgres")]
	identity_always: Option<bool>,
	#[cfg(feature = "db-postgres")]
	identity_by_default: Option<bool>,
	#[cfg(feature = "db-mysql")]
	auto_increment: Option<bool>,
	#[cfg(feature = "db-sqlite")]
	autoincrement: Option<bool>,

	// Character Set & Collation
	collate: Option<String>,
	#[cfg(feature = "db-mysql")]
	character_set: Option<String>,

	// Comment
	#[cfg(any(feature = "db-postgres", feature = "db-mysql"))]
	comment: Option<String>,

	// Phase 2.1: Storage Optimization (PostgreSQL)
	#[cfg(feature = "db-postgres")]
	storage: Option<StorageStrategy>,
	#[cfg(feature = "db-postgres")]
	compression: Option<CompressionMethod>,

	// Phase 2.2: ON UPDATE Trigger (MySQL)
	#[cfg(feature = "db-mysql")]
	on_update_current_timestamp: Option<bool>,

	// Phase 2.2: Invisible Columns (MySQL)
	#[cfg(feature = "db-mysql")]
	invisible: Option<bool>,

	// Phase 2.3: Full-Text Index (PostgreSQL, MySQL)
	#[cfg(any(feature = "db-postgres", feature = "db-mysql"))]
	fulltext: Option<bool>,

	// Phase 3.1: Numeric Attributes (MySQL, deprecated)
	#[cfg(feature = "db-mysql")]
	unsigned: Option<bool>,
	#[cfg(feature = "db-mysql")]
	zerofill: Option<bool>,

	// Constructor generation control
	/// Whether to include this field in the new() function arguments
	/// When true, field is included even if it would normally be auto-generated
	/// When false, field is excluded and uses default value
	include_in_new: Option<bool>,
}

impl FieldConfig {
	/// Parse #[field(...)] attribute
	fn from_attrs(attrs: &[syn::Attribute]) -> Result<Self> {
		let mut config = Self::default();

		for attr in attrs {
			if !attr.path().is_ident("field") {
				continue;
			}

			// Support empty #[field] attribute
			if matches!(attr.meta, syn::Meta::Path(_)) {
				continue;
			}

			attr.parse_nested_meta(|meta| {
				if meta.path.is_ident("primary_key") {
					let value: syn::LitBool = meta.value()?.parse()?;
					config.primary_key = value.value;
					Ok(())
				} else if meta.path.is_ident("max_length") {
					let value: syn::LitInt = meta.value()?.parse()?;
					config.max_length = Some(value.base10_parse()?);
					Ok(())
				} else if meta.path.is_ident("null") {
					let value: syn::LitBool = meta.value()?.parse()?;
					config.null = Some(value.value);
					Ok(())
				} else if meta.path.is_ident("blank") {
					let value: syn::LitBool = meta.value()?.parse()?;
					config.blank = Some(value.value);
					Ok(())
				} else if meta.path.is_ident("unique") {
					let value: syn::LitBool = meta.value()?.parse()?;
					config.unique = Some(value.value);
					Ok(())
				} else if meta.path.is_ident("default") {
					// Parse as Expr to support bool, int, string, etc.
					let value: syn::Expr = meta.value()?.parse()?;
					config.default = Some(value);
					Ok(())
				} else if meta.path.is_ident("db_column") {
					let value: syn::LitStr = meta.value()?.parse()?;
					config.db_column = Some(value.value());
					Ok(())
				} else if meta.path.is_ident("editable") {
					let value: syn::LitBool = meta.value()?.parse()?;
					config.editable = Some(value.value);
					Ok(())
				} else if meta.path.is_ident("index") {
					let value: syn::LitBool = meta.value()?.parse()?;
					config.index = Some(value.value);
					Ok(())
				} else if meta.path.is_ident("check") {
					let value: syn::LitStr = meta.value()?.parse()?;
					config.check = Some(value.value());
					Ok(())
				} else if meta.path.is_ident("email") {
					let value: syn::LitBool = meta.value()?.parse()?;
					config.email = Some(value.value);
					Ok(())
				} else if meta.path.is_ident("url") {
					let value: syn::LitBool = meta.value()?.parse()?;
					config.url = Some(value.value);
					Ok(())
				} else if meta.path.is_ident("min_length") {
					let value: syn::LitInt = meta.value()?.parse()?;
					config.min_length = Some(value.base10_parse()?);
					Ok(())
				} else if meta.path.is_ident("min_value") {
					let value: syn::LitInt = meta.value()?.parse()?;
					config.min_value = Some(value.base10_parse()?);
					Ok(())
				} else if meta.path.is_ident("max_value") {
					let value: syn::LitInt = meta.value()?.parse()?;
					config.max_value = Some(value.base10_parse()?);
					Ok(())
				} else if meta.path.is_ident("auto_now_add") {
					let value: syn::LitBool = meta.value()?.parse()?;
					config.auto_now_add = Some(value.value);
					Ok(())
				} else if meta.path.is_ident("auto_now") {
					let value: syn::LitBool = meta.value()?.parse()?;
					config.auto_now = Some(value.value);
					Ok(())
				} else if meta.path.is_ident("foreign_key") {
					// Try parsing as Type first (direct type specification)
					if let Ok(ty) = meta.value()?.parse::<syn::Type>() {
						config.foreign_key = Some(ForeignKeySpec::Type(ty));
						return Ok(());
					}

					// Fall back to string specification
					if let Ok(value) = meta.value()?.parse::<syn::LitStr>() {
						let spec_str = value.value();

						if spec_str.contains('.') {
							// app_label.model_name format
							let parts: Vec<&str> = spec_str.split('.').collect();
							if parts.len() == 2 {
								config.foreign_key = Some(ForeignKeySpec::AppModel {
									app_label: parts[0].to_string(),
									model_name: parts[1].to_string(),
								});
								return Ok(());
							} else {
								return Err(meta.error(
									"foreign_key must be in 'app_label.model_name' format",
								));
							}
						} else {
							// Type name only (for backward compatibility)
							if let Ok(ty) = syn::parse_str::<syn::Type>(&spec_str) {
								config.foreign_key = Some(ForeignKeySpec::Type(ty));
								return Ok(());
							} else {
								return Err(meta.error("Invalid foreign_key specification"));
							}
						}
					}

					Err(meta.error("foreign_key must be a type (User) or string (\"users.User\")"))
				}
				// Generated Columns
				else if meta.path.is_ident("generated") {
					let value: syn::LitStr = meta.value()?.parse()?;
					config.generated = Some(value.value());
					Ok(())
				} else if meta.path.is_ident("generated_stored") {
					let value: syn::LitBool = meta.value()?.parse()?;
					config.generated_stored = Some(value.value);
					Ok(())
				} else if meta.path.is_ident("generated_virtual") {
					#[cfg(any(feature = "db-mysql", feature = "db-sqlite"))]
					{
						let value: syn::LitBool = meta.value()?.parse()?;
						config.generated_virtual = Some(value.value);
						Ok(())
					}
					#[cfg(not(any(feature = "db-mysql", feature = "db-sqlite")))]
					{
						Err(meta.error(
							"generated_virtual is only available with db-mysql or db-sqlite features",
						))
					}
				}
				// Identity/Auto-increment
				else if meta.path.is_ident("identity_always") {
					#[cfg(feature = "db-postgres")]
					{
						let value: syn::LitBool = meta.value()?.parse()?;
						config.identity_always = Some(value.value);
						Ok(())
					}
					#[cfg(not(feature = "db-postgres"))]
					{
						Err(meta
							.error("identity_always is only available with db-postgres feature"))
					}
				} else if meta.path.is_ident("identity_by_default") {
					#[cfg(feature = "db-postgres")]
					{
						let value: syn::LitBool = meta.value()?.parse()?;
						config.identity_by_default = Some(value.value);
						Ok(())
					}
					#[cfg(not(feature = "db-postgres"))]
					{
						Err(meta.error(
							"identity_by_default is only available with db-postgres feature",
						))
					}
				} else if meta.path.is_ident("auto_increment") {
					#[cfg(feature = "db-mysql")]
					{
						let value: syn::LitBool = meta.value()?.parse()?;
						config.auto_increment = Some(value.value);
						Ok(())
					}
					#[cfg(not(feature = "db-mysql"))]
					{
						Err(meta.error("auto_increment is only available with db-mysql feature"))
					}
				} else if meta.path.is_ident("autoincrement") {
					#[cfg(feature = "db-sqlite")]
					{
						let value: syn::LitBool = meta.value()?.parse()?;
						config.autoincrement = Some(value.value);
						Ok(())
					}
					#[cfg(not(feature = "db-sqlite"))]
					{
						Err(meta.error("autoincrement is only available with db-sqlite feature"))
					}
				}
				// Character Set & Collation
				else if meta.path.is_ident("collate") {
					let value: syn::LitStr = meta.value()?.parse()?;
					config.collate = Some(value.value());
					Ok(())
				} else if meta.path.is_ident("character_set") {
					#[cfg(feature = "db-mysql")]
					{
						let value: syn::LitStr = meta.value()?.parse()?;
						config.character_set = Some(value.value());
						Ok(())
					}
					#[cfg(not(feature = "db-mysql"))]
					{
						Err(meta.error("character_set is only available with db-mysql feature"))
					}
				}
				// Comment
				else if meta.path.is_ident("comment") {
					#[cfg(any(feature = "db-postgres", feature = "db-mysql"))]
					{
						let value: syn::LitStr = meta.value()?.parse()?;
						config.comment = Some(value.value());
						Ok(())
					}
					#[cfg(not(any(feature = "db-postgres", feature = "db-mysql")))]
					{
						Err(meta.error(
							"comment is only available with db-postgres or db-mysql features",
						))
					}
				}
				// Phase 2.1: Storage Optimization
				else if meta.path.is_ident("storage") {
					#[cfg(feature = "db-postgres")]
					{
						let value: syn::LitStr = meta.value()?.parse()?;
						let storage_str = value.value();
						let storage = match storage_str.to_lowercase().as_str() {
							"plain" => StorageStrategy::Plain,
							"extended" => StorageStrategy::Extended,
							"external" => StorageStrategy::External,
							"main" => StorageStrategy::Main,
							_ => {
								return Err(meta.error(
									"storage must be one of: plain, extended, external, main",
								));
							}
						};
						config.storage = Some(storage);
						Ok(())
					}
					#[cfg(not(feature = "db-postgres"))]
					{
						Err(meta.error("storage is only available with db-postgres feature"))
					}
				} else if meta.path.is_ident("compression") {
					#[cfg(feature = "db-postgres")]
					{
						let value: syn::LitStr = meta.value()?.parse()?;
						let compression_str = value.value();
						let compression = match compression_str.to_lowercase().as_str() {
							"pglz" => CompressionMethod::Pglz,
							"lz4" => CompressionMethod::Lz4,
							_ => return Err(meta.error("compression must be one of: pglz, lz4")),
						};
						config.compression = Some(compression);
						Ok(())
					}
					#[cfg(not(feature = "db-postgres"))]
					{
						Err(meta.error("compression is only available with db-postgres feature"))
					}
				}
				// Phase 2.2: ON UPDATE Trigger
				else if meta.path.is_ident("on_update_current_timestamp") {
					#[cfg(feature = "db-mysql")]
					{
						let value: syn::LitBool = meta.value()?.parse()?;
						config.on_update_current_timestamp = Some(value.value);
						Ok(())
					}
					#[cfg(not(feature = "db-mysql"))]
					{
						Err(meta.error(
							"on_update_current_timestamp is only available with db-mysql feature",
						))
					}
				}
				// Phase 2.2: Invisible Columns
				else if meta.path.is_ident("invisible") {
					#[cfg(feature = "db-mysql")]
					{
						let value: syn::LitBool = meta.value()?.parse()?;
						config.invisible = Some(value.value);
						Ok(())
					}
					#[cfg(not(feature = "db-mysql"))]
					{
						Err(meta.error("invisible is only available with db-mysql feature"))
					}
				}
				// Phase 2.3: Full-Text Index
				else if meta.path.is_ident("fulltext") {
					#[cfg(any(feature = "db-postgres", feature = "db-mysql"))]
					{
						let value: syn::LitBool = meta.value()?.parse()?;
						config.fulltext = Some(value.value);
						Ok(())
					}
					#[cfg(not(any(feature = "db-postgres", feature = "db-mysql")))]
					{
						Err(meta.error(
							"fulltext is only available with db-postgres or db-mysql features",
						))
					}
				}
				// Phase 3.1: Numeric Attributes (MySQL, deprecated)
				else if meta.path.is_ident("unsigned") {
					#[cfg(feature = "db-mysql")]
					{
						let value: syn::LitBool = meta.value()?.parse()?;
						config.unsigned = Some(value.value);
						Ok(())
					}
					#[cfg(not(feature = "db-mysql"))]
					{
						Err(meta.error("unsigned is only available with db-mysql feature"))
					}
				} else if meta.path.is_ident("zerofill") {
					#[cfg(feature = "db-mysql")]
					{
						let value: syn::LitBool = meta.value()?.parse()?;
						config.zerofill = Some(value.value);
						Ok(())
					}
					#[cfg(not(feature = "db-mysql"))]
					{
						Err(meta.error("zerofill is only available with db-mysql feature"))
					}
				}
				// Constructor generation control
				else if meta.path.is_ident("include_in_new") {
					let value: syn::LitBool = meta.value()?.parse()?;
					config.include_in_new = Some(value.value);
					Ok(())
				} else {
					Err(meta.error("unsupported field attribute"))
				}
			})?;
		}

		Ok(config)
	}

	/// Validate field configuration for mutual exclusivity and logical consistency
	fn validate(&self) -> Result<()> {
		// Check mutual exclusivity of auto-increment attributes
		#[allow(unused_mut)]
		let mut auto_increment_count = 0;

		#[cfg(feature = "db-postgres")]
		{
			if self.identity_always.is_some() {
				auto_increment_count += 1;
			}
			if self.identity_by_default.is_some() {
				auto_increment_count += 1;
			}
		}

		#[cfg(feature = "db-mysql")]
		{
			if self.auto_increment.is_some() {
				auto_increment_count += 1;
			}
		}

		#[cfg(feature = "db-sqlite")]
		{
			if self.autoincrement.is_some() {
				auto_increment_count += 1;
			}
		}

		if auto_increment_count > 1 {
			return Err(syn::Error::new(
				proc_macro2::Span::call_site(),
				"Only one auto-increment attribute (identity_always, identity_by_default, auto_increment, autoincrement) can be specified per field",
			));
		}

		// Generated columns cannot have default values
		if self.generated.is_some() && self.default.is_some() {
			return Err(syn::Error::new(
				proc_macro2::Span::call_site(),
				"Generated columns cannot have default values",
			));
		}

		// Generated columns should have either generated_stored or generated_virtual
		if self.generated.is_some() {
			let has_stored = self.generated_stored.unwrap_or(false);

			#[cfg(any(feature = "db-mysql", feature = "db-sqlite"))]
			let has_virtual = self.generated_virtual.unwrap_or(false);
			#[cfg(not(any(feature = "db-mysql", feature = "db-sqlite")))]
			let has_virtual = false;

			if !has_stored && !has_virtual {
				return Err(syn::Error::new(
					proc_macro2::Span::call_site(),
					"Generated columns must specify either generated_stored=true or generated_virtual=true",
				));
			}

			if has_stored && has_virtual {
				return Err(syn::Error::new(
					proc_macro2::Span::call_site(),
					"Generated columns cannot be both STORED and VIRTUAL",
				));
			}
		}

		Ok(())
	}
}

/// Field information for processing
#[derive(Debug, Clone)]
struct FieldInfo {
	name: syn::Ident,
	ty: Type,
	config: FieldConfig,
	/// Optional relationship attribute from `#[rel(...)]`
	#[allow(dead_code)]
	rel: Option<RelAttribute>,
}

/// Foreign key / One-to-one field information for automatic ID field generation
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields will be used for accessor generation in future
struct ForeignKeyFieldInfo {
	/// Original field name (e.g., "author")
	field_name: syn::Ident,
	/// Target model type (e.g., User)
	target_type: Type,
	/// Generated ID column name (e.g., "author_id" or custom via db_column)
	id_column_name: String,
	/// Related name for reverse accessor
	related_name: Option<String>,
	/// Whether this is a OneToOne field (requires UNIQUE constraint)
	is_one_to_one: bool,
	/// The full RelAttribute for additional options
	rel_attr: RelAttribute,
}

/// Rustの型からフィールドメタデータ文字列を生成
fn field_type_to_metadata_string(ty: &Type, _config: &FieldConfig) -> Result<String> {
	let (_is_option, inner_ty) = extract_option_type(ty);

	match inner_ty {
		Type::Path(type_path) => {
			let last_segment = type_path
				.path
				.segments
				.last()
				.ok_or_else(|| syn::Error::new_spanned(ty, "Invalid type path"))?;

			let type_name = match last_segment.ident.to_string().as_str() {
				"i32" => "IntegerField",
				"i64" => "BigIntegerField",
				"String" => "CharField",
				"bool" => "BooleanField",
				"f32" | "f64" => "FloatField",
				"DateTime" => "DateTimeField",
				"Date" => "DateField",
				"Time" => "TimeField",
				"Decimal" => "DecimalField",
				"Uuid" => "UuidField",
				other => {
					return Err(syn::Error::new_spanned(
						ty,
						format!("Unsupported field type: {}", other),
					));
				}
			};

			Ok(format!("reinhardt.orm.models.{}", type_name))
		}
		_ => Err(syn::Error::new_spanned(ty, "Unsupported field type")),
	}
}

/// Map Rust type to ORM field type
fn map_type_to_field_type(ty: &Type, config: &FieldConfig) -> Result<TokenStream> {
	// Extract the inner type if it's Option<T>
	let (_is_option, inner_ty) = extract_option_type(ty);

	let field_type = match inner_ty {
		Type::Path(type_path) => {
			let last_segment = type_path
				.path
				.segments
				.last()
				.ok_or_else(|| syn::Error::new_spanned(ty, "Invalid type path"))?;

			match last_segment.ident.to_string().as_str() {
				"i32" => {
					quote! { ::reinhardt::db::migrations::FieldType::Integer }
				}
				"i64" => {
					quote! { ::reinhardt::db::migrations::FieldType::BigInteger }
				}
				"String" => {
					let max_length = config.max_length.ok_or_else(|| {
						syn::Error::new_spanned(ty, "String fields require max_length attribute")
					})? as u32;
					quote! { ::reinhardt::db::migrations::FieldType::VarChar(#max_length) }
				}
				"bool" => {
					quote! { ::reinhardt::db::migrations::FieldType::Boolean }
				}
				"DateTime" => {
					quote! { ::reinhardt::db::migrations::FieldType::DateTime }
				}
				"Date" => {
					quote! { ::reinhardt::db::migrations::FieldType::Date }
				}
				"Time" => {
					quote! { ::reinhardt::db::migrations::FieldType::Time }
				}
				"f32" => {
					quote! { ::reinhardt::db::migrations::FieldType::Float }
				}
				"f64" => {
					quote! { ::reinhardt::db::migrations::FieldType::Double }
				}
				"Uuid" => {
					quote! { ::reinhardt::db::migrations::FieldType::Uuid }
				}
				_ => {
					return Err(syn::Error::new_spanned(
						ty,
						format!("Unsupported field type: {}", last_segment.ident),
					));
				}
			}
		}
		_ => {
			return Err(syn::Error::new_spanned(ty, "Unsupported field type"));
		}
	};

	Ok(field_type)
}

/// Extract Option<T> and return (is_option, inner_type)
fn extract_option_type(ty: &Type) -> (bool, &Type) {
	if let Type::Path(type_path) = ty
		&& let Some(last_segment) = type_path.path.segments.last()
		&& last_segment.ident == "Option"
		&& let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments
		&& let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
	{
		return (true, inner_ty);
	}
	(false, ty)
}

/// Generate field accessor methods that return FieldRef<M, T>
///
/// Generates const methods like:
/// ```ignore
/// impl User {
///     pub const fn field_id() -> FieldRef<User, i64> { FieldRef::new("id") }
///     pub const fn field_name() -> FieldRef<User, String> { FieldRef::new("name") }
/// }
/// ```
fn generate_field_accessors(struct_name: &syn::Ident, field_infos: &[FieldInfo]) -> TokenStream {
	let accessor_methods: Vec<_> = field_infos
		.iter()
		.map(|field| {
			let field_name = &field.name;
			let field_type = &field.ty;
			let method_name = syn::Ident::new(&format!("field_{}", field_name), field_name.span());
			let field_name_str = field_name.to_string();

			quote! {
				/// Field accessor for type-safe field references
				///
				/// Returns a `FieldRef<#struct_name, #field_type>` that provides compile-time
				/// type safety for field operations.
				pub const fn #method_name() -> ::reinhardt::db::orm::expressions::FieldRef<#struct_name, #field_type> {
					::reinhardt::db::orm::expressions::FieldRef::new(#field_name_str)
				}
			}
		})
		.collect();

	quote! {
		impl #struct_name {
			#(#accessor_methods)*
		}
	}
}

/// Implementation of the `Model` derive macro
pub fn model_derive_impl(input: DeriveInput) -> Result<TokenStream> {
	let struct_name = &input.ident;
	let generics = &input.generics;
	let where_clause = &generics.where_clause;

	// Parse model configuration
	let model_config = ModelConfig::from_attrs(&input.attrs, struct_name)?;
	let app_label = &model_config.app_label;
	let table_name = &model_config.table_name;

	// Only support structs
	let fields = match &input.data {
		Data::Struct(data_struct) => match &data_struct.fields {
			Fields::Named(fields) => &fields.named,
			_ => {
				return Err(syn::Error::new_spanned(
					struct_name,
					"Model can only be derived for structs with named fields",
				));
			}
		},
		_ => {
			return Err(syn::Error::new_spanned(
				struct_name,
				"Model can only be derived for structs",
			));
		}
	};

	// Process all fields
	let mut field_infos = Vec::new();
	let mut rel_fields = Vec::new();
	// Collect auto-generated FK _id field names for new() constructor
	let mut fk_id_field_names: Vec<syn::Ident> = Vec::new();

	for field in fields {
		// Check if this is auto-generated FK _id field (marked with #[fk_id_field])
		// These are generated by #[model] attribute macro
		let is_fk_id_field = field
			.attrs
			.iter()
			.any(|attr| attr.path().is_ident("fk_id_field"));
		if is_fk_id_field {
			// Collect the field name for new() constructor generation
			if let Some(field_name) = &field.ident {
				fk_id_field_names.push(field_name.clone());
			}
			continue;
		}

		let name = field
			.ident
			.clone()
			.ok_or_else(|| syn::Error::new_spanned(field, "Field must have a name"))?;
		let ty = field.ty.clone();
		let config = FieldConfig::from_attrs(&field.attrs)?;
		config.validate()?;

		// Parse #[rel(...)] attribute if present
		let rel = field
			.attrs
			.iter()
			.find(|attr| attr.path().is_ident("rel"))
			.map(RelAttribute::from_attribute)
			.transpose()?;

		// Collect relationship fields for later processing
		if let Some(ref rel_attr) = rel {
			rel_fields.push((name.clone(), rel_attr.clone()));
		}

		field_infos.push(FieldInfo {
			name,
			ty,
			config,
			rel,
		});
	}

	// Extract ForeignKeyField and OneToOneField information
	let mut fk_field_infos: Vec<ForeignKeyFieldInfo> = Vec::new();
	for field_info in &field_infos {
		if let Some(ref rel_attr) = field_info.rel {
			// Check if this is a ForeignKeyField or OneToOneField type
			if let Some(target_type) = extract_fk_target_type(&field_info.ty) {
				let is_one_to_one = is_one_to_one_field_type(&field_info.ty);

				// Validate relationship type matches field type
				if is_one_to_one && rel_attr.rel_type != crate::rel::RelationType::OneToOne {
					return Err(syn::Error::new(
						rel_attr.span,
						"OneToOneField must use #[rel(one_to_one, ...)]",
					));
				}
				if is_foreign_key_field_type(&field_info.ty)
					&& rel_attr.rel_type != crate::rel::RelationType::ForeignKey
				{
					return Err(syn::Error::new(
						rel_attr.span,
						"ForeignKeyField must use #[rel(foreign_key, ...)]",
					));
				}

				// Generate ID column name: db_column or {field_name}_id
				let id_column_name = rel_attr
					.db_column
					.clone()
					.unwrap_or_else(|| format!("{}_id", field_info.name));

				fk_field_infos.push(ForeignKeyFieldInfo {
					field_name: field_info.name.clone(),
					target_type: target_type.clone(),
					id_column_name,
					related_name: rel_attr.related_name.clone(),
					is_one_to_one,
					rel_attr: rel_attr.clone(),
				});
			}
		}
	}

	// Find all primary key fields
	let pk_fields: Vec<_> = field_infos
		.iter()
		.filter(|f| f.config.primary_key)
		.collect();

	if pk_fields.is_empty() {
		return Err(syn::Error::new_spanned(
			struct_name,
			"Model must have at least one primary key field",
		));
	}

	// Determine if this is a composite primary key
	let is_composite_pk = pk_fields.len() > 1;

	// Find all indexed fields
	let indexed_fields: Vec<_> = field_infos
		.iter()
		.filter(|f| f.config.index.unwrap_or(false))
		.map(|f| f.name.to_string())
		.collect();

	// Find all check constraint fields
	let check_constraints: Vec<(String, String)> = field_infos
		.iter()
		.filter_map(|f| {
			f.config
				.check
				.as_ref()
				.map(|expr| (f.name.to_string(), expr.clone()))
		})
		.collect();

	// Extract check constraint names and expressions for code generation
	let check_constraint_names: Vec<String> = check_constraints
		.iter()
		.map(|(field_name, _)| format!("{}_check", field_name))
		.collect();
	let check_constraint_expressions: Vec<String> = check_constraints
		.iter()
		.map(|(_, expr)| expr.clone())
		.collect();

	// Process unique constraints from model config
	let unique_constraints: Vec<(Vec<String>, Option<String>, Option<String>)> = model_config
		.constraints
		.iter()
		.map(|c| match c {
			ConstraintSpec::Unique {
				fields,
				name,
				condition,
			} => (fields.clone(), name.clone(), condition.clone()),
		})
		.collect();

	// Generate unique constraint names and definitions for code generation
	let unique_constraint_names: Vec<String> = unique_constraints
		.iter()
		.map(|(fields, name, _)| {
			if let Some(n) = name {
				n.clone()
			} else {
				// Auto-generate name: {table_name}_{field1}_{field2}_uniq
				format!("{}_{}_uniq", table_name, fields.join("_"))
			}
		})
		.collect();

	let unique_constraint_definitions: Vec<String> = unique_constraints
		.iter()
		.map(|(fields, _, condition)| {
			let fields_str = fields.join(", ");
			if let Some(cond) = condition {
				format!("UNIQUE ({}) WHERE {}", fields_str, cond)
			} else {
				format!("UNIQUE ({})", fields_str)
			}
		})
		.collect();

	// Define composite_pk_type_def and holder for code generation
	let composite_pk_type_def: Option<TokenStream>;
	// Note: composite_pk_type_holder is only assigned in the composite PK branch,
	// but must be declared here to extend its lifetime beyond the if-else scope
	#[allow(unused_assignments)]
	let mut composite_pk_type_holder: Option<Type> = None;

	// For single PK, extract field info
	let (pk_name, _pk_ty, pk_is_option, pk_type) = if !is_composite_pk {
		composite_pk_type_def = None;
		let pk_field = pk_fields[0];
		let pk_name = &pk_field.name;
		let pk_ty = &pk_field.ty;
		let (pk_is_option, pk_inner_ty) = extract_option_type(pk_ty);
		let pk_type = if pk_is_option { pk_inner_ty } else { pk_ty };
		(pk_name, pk_ty, pk_is_option, pk_type)
	} else {
		// Composite primary key: generate dedicated composite PK type
		let composite_pk_name =
			syn::Ident::new(&format!("{}CompositePk", struct_name), struct_name.span());

		// Generate the composite PK type definition
		composite_pk_type_def = Some(generate_composite_pk_type(struct_name, &pk_fields));

		// Use the generated composite PK type and store in holder (avoid temporary variable)
		composite_pk_type_holder = Some(parse_quote! { #composite_pk_name });
		let composite_pk_type_ref = composite_pk_type_holder.as_ref().unwrap();

		// Use first field name for primary_key_field() (legacy API compatibility)
		let first_pk_name = &pk_fields[0].name;
		(
			first_pk_name,
			composite_pk_type_ref,
			false,
			composite_pk_type_ref,
		)
	};

	// Generate field_metadata implementation
	let field_metadata_items = generate_field_metadata(&field_infos, &fk_field_infos)?;

	// Generate auto-registration code
	let registration_code = generate_registration_code(
		struct_name,
		app_label,
		table_name,
		&field_infos,
		&fk_field_infos,
	)?;

	// Generate primary_key() and set_primary_key() implementations
	let (pk_impl, set_pk_impl, composite_pk_impl) = if is_composite_pk {
		// Composite primary key implementation
		let composite_impl = generate_composite_pk_impl(&pk_fields);

		// For composite PK, use the generated composite PK type
		let pk_field_names: Vec<_> = pk_fields.iter().map(|f| &f.name).collect();

		// Check if any field is Option
		let has_option_fields = pk_fields.iter().any(|f| {
			let (is_option, _) = extract_option_type(&f.ty);
			is_option
		});

		let pk_getter = if has_option_fields {
			// If any field is Option, check all fields have values
			quote! {
				fn primary_key(&self) -> Option<&Self::PrimaryKey> {
					// Check if all fields have values
					if #(self.#pk_field_names.is_some())&&* {
						// For composite PK, we need to construct a new value each time
						// and store it somewhere with a stable address.
						// We use Box::leak to create a 'static reference.
						// Note: This intentionally leaks memory. For production use,
						// consider using an internal cache or modifying the Model trait
						// to return an owned value instead of a reference.
						let pk = Box::new(Self::PrimaryKey::new(
							#(self.#pk_field_names.clone().unwrap()),*
						));
						Some(Box::leak(pk))
					} else {
						None
					}
				}
			}
		} else {
			// All fields are non-Option, construct composite PK directly
			quote! {
				fn primary_key(&self) -> Option<&Self::PrimaryKey> {
					// For composite PK, we need to construct a new value each time
					// and store it somewhere with a stable address.
					// We use Box::leak to create a 'static reference.
					// Note: This intentionally leaks memory. For production use,
					// consider using an internal cache or modifying the Model trait
					// to return an owned value instead of a reference.
					let pk = Box::new(Self::PrimaryKey::new(
						#(self.#pk_field_names.clone()),*
					));
					Some(Box::leak(pk))
				}
			}
		};

		let pk_setter = if has_option_fields {
			quote! {
				fn set_primary_key(&mut self, value: Self::PrimaryKey) {
					#(
						self.#pk_field_names = Some(value.#pk_field_names);
					)*
				}
			}
		} else {
			quote! {
				fn set_primary_key(&mut self, value: Self::PrimaryKey) {
					#(
						self.#pk_field_names = value.#pk_field_names;
					)*
				}
			}
		};

		(pk_getter, pk_setter, composite_impl)
	} else {
		// Single primary key implementation
		let (pk_getter, pk_setter) = if pk_is_option {
			// If primary key is Option<T>, extract the inner value
			(
				quote! {
					fn primary_key(&self) -> Option<&Self::PrimaryKey> {
						self.#pk_name.as_ref()
					}
				},
				quote! {
					fn set_primary_key(&mut self, value: Self::PrimaryKey) {
						self.#pk_name = Some(value);
					}
				},
			)
		} else {
			// If primary key is not Option, wrap in Some
			(
				quote! {
					fn primary_key(&self) -> Option<&Self::PrimaryKey> {
						Some(&self.#pk_name)
					}
				},
				quote! {
					fn set_primary_key(&mut self, value: Self::PrimaryKey) {
						self.#pk_name = value;
					}
				},
			)
		};

		(pk_getter, pk_setter, quote! {})
	};

	// Generate field accessor methods
	let field_accessors = generate_field_accessors(struct_name, &field_infos);

	// Generate relationship metadata
	let relationship_metadata = generate_relationship_metadata(&rel_fields, app_label, struct_name);

	// Generate new() constructor function
	let new_fn_impl = generate_new_function(struct_name, &field_infos, &fk_id_field_names);

	// Generate the Model implementation
	let expanded = quote! {
		// Generate composite PK type definition if needed
		#composite_pk_type_def

		// Generate new() constructor function
		#new_fn_impl

		// Generate field accessor methods for type-safe field references
		#field_accessors

		impl #generics ::reinhardt::db::orm::Model for #struct_name #generics #where_clause {
			type PrimaryKey = #pk_type;

			fn table_name() -> &'static str {
				#table_name
			}

			fn app_label() -> &'static str {
				#app_label
			}

			fn primary_key_field() -> &'static str {
				stringify!(#pk_name)
			}

			#pk_impl

			#set_pk_impl

			#composite_pk_impl

			fn field_metadata() -> Vec<::reinhardt::db::orm::inspection::FieldInfo> {
				vec![
					#(#field_metadata_items),*
				]
			}

			fn index_metadata() -> Vec<::reinhardt::db::orm::inspection::IndexInfo> {
				vec![
					#(
						::reinhardt::db::orm::inspection::IndexInfo {
							name: format!("{}_{}_idx", <Self as ::reinhardt::db::orm::Model>::table_name(), #indexed_fields),
							fields: vec![#indexed_fields.to_string()],
							unique: false,
							condition: None,
						}
					),*
				]
			}

			fn constraint_metadata() -> Vec<::reinhardt::db::orm::inspection::ConstraintInfo> {
				let mut constraints = Vec::new();
				// Check constraints
				#(
					constraints.push(::reinhardt::db::orm::inspection::ConstraintInfo {
						name: #check_constraint_names.to_string(),
						constraint_type: ::reinhardt::db::orm::inspection::ConstraintType::Check,
						definition: #check_constraint_expressions.to_string(),
					});
				)*
				// Unique constraints
				#(
					constraints.push(::reinhardt::db::orm::inspection::ConstraintInfo {
						name: #unique_constraint_names.to_string(),
						constraint_type: ::reinhardt::db::orm::inspection::ConstraintType::Unique,
						definition: #unique_constraint_definitions.to_string(),
					});
				)*
				constraints
			}

			#relationship_metadata
		}

		#registration_code
	};

	Ok(expanded)
}

/// Generate FieldInfo construction for field_metadata()
fn generate_field_metadata(
	field_infos: &[FieldInfo],
	fk_field_infos: &[ForeignKeyFieldInfo],
) -> Result<Vec<TokenStream>> {
	let mut items = Vec::new();

	// Filter out ManyToMany, ForeignKeyField, and OneToOneField - they are virtual
	let regular_fields: Vec<_> = field_infos
		.iter()
		.filter(|f| {
			// Exclude ManyToMany
			if f.rel
				.as_ref()
				.map(|r| matches!(r.rel_type, crate::rel::RelationType::ManyToMany))
				.unwrap_or(false)
			{
				return false;
			}
			// Exclude ForeignKeyField and OneToOneField (we generate _id fields instead)
			if is_relationship_field_type(&f.ty) {
				return false;
			}
			true
		})
		.collect();

	for field_info in regular_fields {
		let name = field_info.name.to_string();
		let field_type_path = field_type_to_metadata_string(&field_info.ty, &field_info.config)?;
		let _field_type = map_type_to_field_type(&field_info.ty, &field_info.config)?;
		let config = &field_info.config;

		let (is_option, _) = extract_option_type(&field_info.ty);
		let nullable = config.null.unwrap_or(is_option);
		let primary_key = config.primary_key;
		let unique = config.unique.unwrap_or(false);
		let blank = config.blank.unwrap_or(false);
		let editable = config.editable.unwrap_or(true);

		// Build attributes map
		let mut attrs = Vec::new();
		if let Some(max_length) = config.max_length {
			attrs.push(quote! {
				attributes.insert(
					"max_length".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::Uint(#max_length)
				);
			});
		}

		// Add validator attributes
		if let Some(email) = config.email
			&& email
		{
			attrs.push(quote! {
				attributes.insert(
					"email".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::Bool(true)
				);
			});
		}
		if let Some(url) = config.url
			&& url
		{
			attrs.push(quote! {
				attributes.insert(
					"url".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::Bool(true)
				);
			});
		}
		if let Some(min_length) = config.min_length {
			attrs.push(quote! {
				attributes.insert(
					"min_length".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::Uint(#min_length)
				);
			});
		}
		if let Some(min_value) = config.min_value {
			attrs.push(quote! {
				attributes.insert(
					"min_value".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::Int(#min_value)
				);
			});
		}
		if let Some(max_value) = config.max_value {
			attrs.push(quote! {
				attributes.insert(
					"max_value".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::Int(#max_value)
				);
			});
		}

		// Generated Columns
		if let Some(ref generated_expr) = config.generated {
			attrs.push(quote! {
				attributes.insert(
					"generated".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::String(#generated_expr.to_string())
				);
			});
		}
		if let Some(generated_stored) = config.generated_stored {
			attrs.push(quote! {
				attributes.insert(
					"generated_stored".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::Bool(#generated_stored)
				);
			});
		}
		#[cfg(any(feature = "db-mysql", feature = "db-sqlite"))]
		if let Some(generated_virtual) = config.generated_virtual {
			attrs.push(quote! {
				attributes.insert(
					"generated_virtual".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::Bool(#generated_virtual)
				);
			});
		}

		// Identity/Auto-increment
		#[cfg(feature = "db-postgres")]
		if let Some(identity_always) = config.identity_always {
			attrs.push(quote! {
				attributes.insert(
					"identity_always".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::Bool(#identity_always)
				);
			});
		}
		#[cfg(feature = "db-postgres")]
		if let Some(identity_by_default) = config.identity_by_default {
			attrs.push(quote! {
				attributes.insert(
					"identity_by_default".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::Bool(#identity_by_default)
				);
			});
		}
		#[cfg(feature = "db-mysql")]
		if let Some(auto_increment) = config.auto_increment {
			attrs.push(quote! {
				attributes.insert(
					"auto_increment".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::Bool(#auto_increment)
				);
			});
		}
		#[cfg(feature = "db-sqlite")]
		if let Some(autoincrement) = config.autoincrement {
			attrs.push(quote! {
				attributes.insert(
					"autoincrement".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::Bool(#autoincrement)
				);
			});
		}

		// Character Set & Collation
		if let Some(ref collate) = config.collate {
			attrs.push(quote! {
				attributes.insert(
					"collate".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::String(#collate.to_string())
				);
			});
		}
		#[cfg(feature = "db-mysql")]
		if let Some(ref character_set) = config.character_set {
			attrs.push(quote! {
				attributes.insert(
					"character_set".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::String(#character_set.to_string())
				);
			});
		}

		// Comment
		#[cfg(any(feature = "db-postgres", feature = "db-mysql"))]
		if let Some(ref comment) = config.comment {
			attrs.push(quote! {
				attributes.insert(
					"comment".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::String(#comment.to_string())
				);
			});
		}

		// Phase 2.1: Storage Optimization (PostgreSQL)
		#[cfg(feature = "db-postgres")]
		if let Some(ref storage) = config.storage {
			let storage_str = match storage {
				StorageStrategy::Plain => "plain",
				StorageStrategy::Extended => "extended",
				StorageStrategy::External => "external",
				StorageStrategy::Main => "main",
			};
			attrs.push(quote! {
				attributes.insert(
					"storage".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::String(#storage_str.to_string())
				);
			});
		}
		#[cfg(feature = "db-postgres")]
		if let Some(ref compression) = config.compression {
			let compression_str = match compression {
				CompressionMethod::Pglz => "pglz",
				CompressionMethod::Lz4 => "lz4",
			};
			attrs.push(quote! {
				attributes.insert(
					"compression".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::String(#compression_str.to_string())
				);
			});
		}

		// Phase 2.2: ON UPDATE Trigger (MySQL)
		#[cfg(feature = "db-mysql")]
		if let Some(on_update_current_timestamp) = config.on_update_current_timestamp {
			attrs.push(quote! {
				attributes.insert(
					"on_update_current_timestamp".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::Bool(#on_update_current_timestamp)
				);
			});
		}

		// Phase 2.3: Invisible Columns (MySQL)
		#[cfg(feature = "db-mysql")]
		if let Some(invisible) = config.invisible {
			attrs.push(quote! {
				attributes.insert(
					"invisible".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::Bool(#invisible)
				);
			});
		}

		// Phase 2.4: Full-Text Index (PostgreSQL, MySQL)
		#[cfg(any(feature = "db-postgres", feature = "db-mysql"))]
		if let Some(fulltext) = config.fulltext {
			attrs.push(quote! {
				attributes.insert(
					"fulltext".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::Bool(#fulltext)
				);
			});
		}

		// Phase 3.1: Numeric Attributes (MySQL, deprecated)
		#[cfg(feature = "db-mysql")]
		if let Some(unsigned) = config.unsigned {
			attrs.push(quote! {
				attributes.insert(
					"unsigned".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::Bool(#unsigned)
				);
			});
		}
		#[cfg(feature = "db-mysql")]
		if let Some(zerofill) = config.zerofill {
			attrs.push(quote! {
				attributes.insert(
					"zerofill".to_string(),
					::reinhardt::db::orm::fields::FieldKwarg::Bool(#zerofill)
				);
			});
		}

		let db_column_value = match &config.db_column {
			Some(col) => quote! { Some(#col.to_string()) },
			None => quote! { None },
		};

		let item = quote! {
			{
				let mut attributes = ::std::collections::HashMap::new();
				#(#attrs)*

				::reinhardt::db::orm::inspection::FieldInfo {
					name: #name.to_string(),
					field_type: #field_type_path.to_string(),
					nullable: #nullable,
					primary_key: #primary_key,
					unique: #unique,
					blank: #blank,
					editable: #editable,
					default: None,
					db_default: None,
					db_column: #db_column_value,
					choices: None,
					attributes,
				}
			}
		};

		items.push(item);
	}

	// Generate _id field metadata for ForeignKeyField and OneToOneField
	for fk_info in fk_field_infos {
		let name = &fk_info.id_column_name;
		let nullable = fk_info.rel_attr.null.unwrap_or(false);
		let unique = fk_info.is_one_to_one; // OneToOne fields have UNIQUE constraint
		let db_index = fk_info.rel_attr.db_index.unwrap_or(true); // FK fields are indexed by default

		// Generate the field type based on target model's primary key
		// We use IntegerField as a safe default; runtime will resolve the actual type
		let field_type_path = "IntegerField";

		let item = quote! {
			{
				let mut attributes = ::std::collections::HashMap::new();
				if #db_index {
					attributes.insert(
						"db_index".to_string(),
						::reinhardt::db::orm::fields::FieldKwarg::Bool(true)
					);
				}

				::reinhardt::db::orm::inspection::FieldInfo {
					name: #name.to_string(),
					field_type: #field_type_path.to_string(),
					nullable: #nullable,
					primary_key: false,
					unique: #unique,
					blank: false,
					editable: true,
					default: None,
					db_default: None,
					db_column: None,
					choices: None,
					attributes,
				}
			}
		};

		items.push(item);
	}

	Ok(items)
}

/// Generate automatic registration code using ctor
fn generate_registration_code(
	struct_name: &syn::Ident,
	app_label: &str,
	table_name: &str,
	field_infos: &[FieldInfo],
	fk_field_infos: &[ForeignKeyFieldInfo],
) -> Result<TokenStream> {
	let model_name = struct_name.to_string();
	let register_fn_name = syn::Ident::new(
		&format!(
			"__register_{}_model",
			struct_name.to_string().to_lowercase()
		),
		struct_name.span(),
	);

	// Separate ManyToMany fields from regular fields (also exclude ForeignKeyField/OneToOneField)
	let (m2m_fields, regular_fields): (Vec<_>, Vec<_>) = field_infos.iter().partition(|f| {
		// Exclude ManyToMany
		if f.rel
			.as_ref()
			.map(|r| matches!(r.rel_type, crate::rel::RelationType::ManyToMany))
			.unwrap_or(false)
		{
			return true;
		}
		// Exclude ForeignKeyField and OneToOneField (they are virtual, we generate _id fields instead)
		if is_relationship_field_type(&f.ty) {
			return true;
		}
		false
	});

	// Generate field registration code for regular fields
	let mut field_registrations = Vec::new();
	for field_info in &regular_fields {
		let field_name = field_info.name.to_string();
		let field_type = map_type_to_field_type(&field_info.ty, &field_info.config)?;
		let config = &field_info.config;

		let mut params = Vec::new();
		if config.primary_key {
			params.push(quote! { .with_param("primary_key", "true") });
		}
		if let Some(max_length) = config.max_length {
			let ml_str = max_length.to_string();
			params.push(quote! { .with_param("max_length", #ml_str) });
		}
		if let Some(null) = config.null {
			let null_str = null.to_string();
			params.push(quote! { .with_param("null", #null_str) });
		}
		if let Some(unique) = config.unique
			&& unique
		{
			params.push(quote! { .with_param("unique", "true") });
		}

		// Generate ForeignKey information if present
		let fk_registration = if let Some(fk_spec) = &config.foreign_key {
			match fk_spec {
				ForeignKeySpec::Type(ty) => {
					// For direct type reference, extract type name and convert to snake_case
					let type_name_str = quote! { #ty }.to_string();
					quote! {
						.with_foreign_key({
							// Extract last segment of type path and convert to snake_case
							let type_name = #type_name_str;
							let last_segment = type_name.split("::").last().unwrap_or(&type_name);
							let referenced_table = ::reinhardt::db::migrations::to_snake_case(last_segment);

							::reinhardt::db::migrations::ForeignKeyInfo {
								referenced_table,
								referenced_column: "id".to_string(),
								on_delete: ::reinhardt::db::migrations::ForeignKeyAction::Cascade,
								on_update: ::reinhardt::db::migrations::ForeignKeyAction::Cascade,
							}
						})
					}
				}
				ForeignKeySpec::AppModel {
					app_label,
					model_name,
				} => {
					let table_name_str = format!("{}_{}", app_label, model_name.to_lowercase());
					quote! {
						.with_foreign_key(::reinhardt::db::migrations::ForeignKeyInfo {
							referenced_table: #table_name_str.to_string(),
							referenced_column: "id".to_string(),
							on_delete: ::reinhardt::db::migrations::ForeignKeyAction::Cascade,
							on_update: ::reinhardt::db::migrations::ForeignKeyAction::Cascade,
						})
					}
				}
			}
		} else {
			quote! {}
		};

		field_registrations.push(quote! {
			metadata.add_field(
				#field_name.to_string(),
				::reinhardt::db::migrations::model_registry::FieldMetadata::new(#field_type)
					#(#params)*
					#fk_registration
			);
		});
	}

	// Generate ManyToMany field registration code
	let mut m2m_registrations = Vec::new();
	for field_info in &m2m_fields {
		let field_name = field_info.name.to_string();

		// Get target model name: from #[rel(to = "...")] or infer from ManyToManyField<Source, Target>
		let to_model = if let Some(rel) = &field_info.rel
			&& let Some(to_type) = &rel.to
		{
			// Explicit 'to' parameter in #[rel(...)]
			quote! { #to_type }.to_string()
		} else if let Some(target_ty) = extract_m2m_target_type(&field_info.ty) {
			// Infer from ManyToManyField<Source, Target> - extract Target type name
			if let Type::Path(type_path) = target_ty
				&& let Some(last_segment) = type_path.path.segments.last()
			{
				last_segment.ident.to_string()
			} else {
				continue; // Skip if cannot extract target type
			}
		} else {
			continue; // Skip if no 'to' parameter and cannot infer from type
		};

		// Get relationship attributes (may be None if no #[rel(...)] attribute)
		let related_name = field_info
			.rel
			.as_ref()
			.and_then(|r| r.related_name.as_ref())
			.map(|r| quote! { Some(#r.to_string()) })
			.unwrap_or(quote! { None });
		let through = field_info
			.rel
			.as_ref()
			.and_then(|r| r.through.as_ref())
			.map(|t| quote! { Some(#t.to_string()) })
			.unwrap_or(quote! { None });
		let source_field = field_info
			.rel
			.as_ref()
			.and_then(|r| r.source_field.as_ref())
			.map(|s| quote! { Some(#s.to_string()) })
			.unwrap_or(quote! { None });
		let target_field = field_info
			.rel
			.as_ref()
			.and_then(|r| r.target_field.as_ref())
			.map(|t| quote! { Some(#t.to_string()) })
			.unwrap_or(quote! { None });

		m2m_registrations.push(quote! {
			metadata.add_many_to_many(
				::reinhardt::db::migrations::model_registry::ManyToManyMetadata {
					field_name: #field_name.to_string(),
					to_model: #to_model.to_string(),
					related_name: #related_name,
					through: #through,
					source_field: #source_field,
					target_field: #target_field,
					db_constraint_prefix: None,
				}
			);
		});
	}

	// Generate FK _id field registration code
	let mut fk_id_registrations = Vec::new();
	for fk_info in fk_field_infos {
		let id_column_name = &fk_info.id_column_name;
		let nullable = fk_info.rel_attr.null.unwrap_or(false);
		let unique = fk_info.is_one_to_one; // OneToOne fields have UNIQUE constraint
		let db_index = fk_info.rel_attr.db_index.unwrap_or(true); // FK fields are indexed by default
		let nullable_str = nullable.to_string();
		let unique_str = unique.to_string();
		let db_index_str = db_index.to_string();

		// ForeignKeyField<User> → "User" を抽出
		let target_model_name = if let Type::Path(type_path) = &fk_info.target_type {
			type_path
				.path
				.segments
				.last()
				.map(|seg| seg.ident.to_string())
				.unwrap_or_else(|| "Unknown".to_string())
		} else {
			"Unknown".to_string()
		};

		fk_id_registrations.push(quote! {
			metadata.add_field(
				#id_column_name.to_string(),
				::reinhardt::db::migrations::model_registry::FieldMetadata::new(
					::reinhardt::db::migrations::FieldType::Uuid
				)
					.with_param("null", #nullable_str)
					.with_param("unique", #unique_str)
					.with_param("db_index", #db_index_str)
					.with_param("fk_target", #target_model_name)
			);
		});
	}

	// Generate type path for global model registry
	let type_path = quote! { #struct_name }.to_string();

	let code = quote! {
		#[::ctor::ctor]
		fn #register_fn_name() {
			use ::reinhardt::db::migrations::model_registry::ModelMetadata;

			// Register in migration registry
			let mut metadata = ModelMetadata::new(
				#app_label,
				#model_name,
				#table_name,
			);

			#(#field_registrations)*
			#(#fk_id_registrations)*
			#(#m2m_registrations)*

			::reinhardt::db::migrations::model_registry::global_registry().register_model(metadata);

			// Register in global model registry for foreign_key resolution
			::reinhardt::db::orm::registry::global_model_registry().register(
				::reinhardt::db::orm::registry::ModelInfo {
					app_label: #app_label.to_string(),
					model_name: #model_name.to_string(),
					type_path: #type_path.to_string(),
					table_name: #table_name.to_string(),
				}
			);
		}
	};

	Ok(code)
}

/// Generate composite primary key implementation
fn generate_composite_pk_impl(pk_fields: &[&FieldInfo]) -> TokenStream {
	let field_name_strings: Vec<String> = pk_fields.iter().map(|f| f.name.to_string()).collect();

	quote! {
		fn composite_primary_key() -> Option<::reinhardt::db::orm::composite_pk::CompositePrimaryKey> {
			Some(
				::reinhardt::db::orm::composite_pk::CompositePrimaryKey::new(
					vec![#(#field_name_strings.to_string()),*]
				)
				.expect("Invalid composite primary key")
			)
		}

		fn get_composite_pk_values(&self) -> ::std::collections::HashMap<String, ::reinhardt::db::orm::composite_pk::PkValue> {
			// Use the generated composite PK type's to_pk_values() method
			if let Some(pk) = self.primary_key() {
				pk.to_pk_values()
			} else {
				::std::collections::HashMap::new()
			}
		}
	}
}

/// Generate composite primary key type definition
///
/// Creates a dedicated struct type for composite primary keys with:
/// - Named fields matching the model's PK fields
/// - Derived traits: Debug, Clone, PartialEq, Eq, Hash
/// - From/Into conversions for tuple types
/// - Individual PkValue conversions for each field
fn generate_composite_pk_type(struct_name: &syn::Ident, pk_fields: &[&FieldInfo]) -> TokenStream {
	// Generate composite PK struct name: {ModelName}CompositePk
	let composite_pk_name =
		syn::Ident::new(&format!("{}CompositePk", struct_name), struct_name.span());

	// Extract field names and types
	let field_names: Vec<_> = pk_fields.iter().map(|f| &f.name).collect();
	let field_types: Vec<_> = pk_fields
		.iter()
		.map(|f| {
			let ty = &f.ty;
			let (is_option, inner_ty) = extract_option_type(ty);
			if is_option { inner_ty } else { ty }
		})
		.collect();

	// Generate From<tuple> implementation for easy construction
	let tuple_type = if field_types.len() == 1 {
		quote! { #(#field_types),* }
	} else {
		quote! { (#(#field_types),*) }
	};

	// Generate individual field conversions for PkValue
	let pk_value_conversions: Vec<_> = field_names
		.iter()
		.map(|name| {
			quote! {
				values.insert(
					stringify!(#name).to_string(),
					::reinhardt::db::orm::composite_pk::PkValue::from(&self.#name)
				);
			}
		})
		.collect();

	quote! {
		/// Composite primary key type for #struct_name
		#[derive(Debug, Clone, PartialEq, Eq, Hash)]
		pub struct #composite_pk_name {
			#(pub #field_names: #field_types),*
		}

		impl #composite_pk_name {
			/// Create a new composite primary key
			pub fn new(#(#field_names: #field_types),*) -> Self {
				Self {
					#(#field_names),*
				}
			}

			/// Convert to a HashMap of PkValues for database operations
			pub fn to_pk_values(&self) -> ::std::collections::HashMap<String, ::reinhardt::db::orm::composite_pk::PkValue> {
				let mut values = ::std::collections::HashMap::new();
				#(#pk_value_conversions)*
				values
			}
		}

		// Conversion from tuple type
		impl ::std::convert::From<#tuple_type> for #composite_pk_name {
			fn from(tuple: #tuple_type) -> Self {
				let (#(#field_names),*) = tuple;
				Self {
					#(#field_names),*
				}
			}
		}

		// Conversion to tuple type
		impl ::std::convert::From<#composite_pk_name> for #tuple_type {
			fn from(pk: #composite_pk_name) -> Self {
				(#(pk.#field_names),*)
			}
		}

		// Display implementation for composite primary key
		impl ::std::fmt::Display for #composite_pk_name {
			fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
				write!(f, "(")?;
				let mut first = true;
				#(
					if !first {
						write!(f, ", ")?;
					}
					write!(f, "{}={}", stringify!(#field_names), self.#field_names)?;
					first = false;
				)*
				write!(f, ")")
			}
		}
	}
}

/// Generate relationship metadata code for `#[rel]` attributes
///
/// Generates two methods:
/// - `relationship_metadata()` for Model trait (returns `Vec<RelationInfo>`)
/// - `__migration_relationships()` for migration system (returns `Vec<RelationshipMetadata>`)
fn generate_relationship_metadata(
	rel_fields: &[(Ident, RelAttribute)],
	_app_label: &str,
	_struct_name: &Ident,
) -> TokenStream {
	use crate::rel::RelationType;

	if rel_fields.is_empty() {
		return quote! {
			fn relationship_metadata() -> Vec<::reinhardt::db::orm::inspection::RelationInfo> {
				Vec::new()
			}
		};
	}

	let relation_info_items: Vec<TokenStream> = rel_fields
		.iter()
		.map(|(field_name, rel)| {
			let field_name_str = field_name.to_string();

			// Map RelationType to RelationshipType
			let relationship_type = match rel.rel_type {
				RelationType::ForeignKey => {
					quote! { ::reinhardt::db::orm::relationship::RelationshipType::ManyToOne }
				}
				RelationType::OneToOne => {
					quote! { ::reinhardt::db::orm::relationship::RelationshipType::OneToOne }
				}
				RelationType::OneToMany => {
					quote! { ::reinhardt::db::orm::relationship::RelationshipType::OneToMany }
				}
				RelationType::ManyToMany | RelationType::PolymorphicManyToMany => {
					quote! { ::reinhardt::db::orm::relationship::RelationshipType::ManyToMany }
				}
				RelationType::Polymorphic => {
					// Polymorphic is treated as ManyToOne for now
					quote! { ::reinhardt::db::orm::relationship::RelationshipType::ManyToOne }
				}
			};

			let related_model = rel.to.as_ref().map_or_else(
				|| quote! { "" },
				|path| {
					let path_str = quote! { #path }.to_string();
					quote! { #path_str }
				},
			);

			let back_populates = rel.related_name.as_ref().map_or_else(
				|| quote! { None },
				|name| quote! { Some(#name.to_string()) },
			);

			// For ForeignKey, the foreign key field is the field itself
			let foreign_key = match rel.rel_type {
				RelationType::ForeignKey | RelationType::OneToOne => {
					quote! { Some(#field_name_str.to_string()) }
				}
				RelationType::OneToMany => rel
					.foreign_key
					.as_ref()
					.map_or_else(|| quote! { None }, |fk| quote! { Some(#fk.to_string()) }),
				_ => quote! { None },
			};

			quote! {
				::reinhardt::db::orm::inspection::RelationInfo {
					name: #field_name_str.to_string(),
					relationship_type: #relationship_type,
					foreign_key: #foreign_key,
					related_model: #related_model.to_string(),
					back_populates: #back_populates,
				}
			}
		})
		.collect();

	quote! {
		fn relationship_metadata() -> Vec<::reinhardt::db::orm::inspection::RelationInfo> {
			vec![
				#(#relation_info_items),*
			]
		}
	}
}

/// Check if a type is Uuid or Option<Uuid>
fn is_uuid_type(ty: &Type) -> bool {
	let (_, inner_ty) = extract_option_type(ty);
	if let Type::Path(type_path) = inner_ty
		&& let Some(last_segment) = type_path.path.segments.last()
	{
		return last_segment.ident == "Uuid";
	}
	false
}

/// Check if a type is a ManyToManyField
fn is_many_to_many_field_type(ty: &Type) -> bool {
	if let Type::Path(type_path) = ty
		&& let Some(last_segment) = type_path.path.segments.last()
	{
		return last_segment.ident == "ManyToManyField";
	}
	false
}

/// Check if a type is a ForeignKeyField
fn is_foreign_key_field_type(ty: &Type) -> bool {
	if let Type::Path(type_path) = ty
		&& let Some(last_segment) = type_path.path.segments.last()
	{
		return last_segment.ident == "ForeignKeyField";
	}
	false
}

/// Check if a type is a OneToOneField
fn is_one_to_one_field_type(ty: &Type) -> bool {
	if let Type::Path(type_path) = ty
		&& let Some(last_segment) = type_path.path.segments.last()
	{
		return last_segment.ident == "OneToOneField";
	}
	false
}

/// Extract target type from ForeignKeyField<T> or OneToOneField<T>
fn extract_fk_target_type(ty: &Type) -> Option<&Type> {
	if let Type::Path(type_path) = ty
		&& let Some(last_segment) = type_path.path.segments.last()
		&& (last_segment.ident == "ForeignKeyField" || last_segment.ident == "OneToOneField")
		&& let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments
		&& let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
	{
		return Some(inner_ty);
	}
	None
}

/// Extract target type from ManyToManyField<Source, Target>
/// Returns the second generic argument (Target model)
fn extract_m2m_target_type(ty: &Type) -> Option<&Type> {
	if let Type::Path(type_path) = ty
		&& let Some(last_segment) = type_path.path.segments.last()
		&& last_segment.ident == "ManyToManyField"
		&& let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments
		&& args.args.len() >= 2
		&& let Some(syn::GenericArgument::Type(target_ty)) = args.args.iter().nth(1)
	{
		return Some(target_ty);
	}
	None
}

/// Check if a type is a relationship field type (ForeignKeyField or OneToOneField)
fn is_relationship_field_type(ty: &Type) -> bool {
	is_foreign_key_field_type(ty) || is_one_to_one_field_type(ty)
}

/// Check if a field is a timestamp field that should be auto-set to Utc::now()
fn is_timestamp_field(field: &FieldInfo) -> bool {
	let config = &field.config;

	// 1. 明示的な属性指定
	if config.auto_now_add == Some(true) || config.auto_now == Some(true) {
		return true;
	}

	// 2. フィールド名による自動検出
	let field_name = field.name.to_string();
	matches!(
		field_name.as_str(),
		"created_at" | "updated_at" | "date_joined" | "last_login" | "last_modified"
	)
}

/// Extract the target model type from ForeignKeyField<T> or OneToOneField<T>
fn extract_foreign_key_target_type(ty: &Type) -> Type {
	// ForeignKeyField<User> -> User
	if let Type::Path(type_path) = ty {
		if let Some(segment) = type_path.path.segments.last() {
			if let PathArguments::AngleBracketed(args) = &segment.arguments {
				if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
					return inner_ty.clone();
				}
			}
		}
	}
	// フォールバック: 型全体を返す
	ty.clone()
}

/// Determine if a field should be auto-generated (excluded from new() function arguments)
fn is_auto_generated_field(field: &FieldInfo) -> bool {
	let config = &field.config;

	// If include_in_new is explicitly set to false, exclude from new()
	if config.include_in_new == Some(false) {
		return true;
	}

	// If include_in_new is explicitly set to true, always include in new()
	if config.include_in_new == Some(true) {
		return false;
	}

	// タイムスタンプフィールドの自動検出（新規追加）
	if is_timestamp_field(field) {
		return true;
	}

	// Generated columns
	if config.generated.is_some() {
		return true;
	}

	// Database-specific ID auto-generation (PostgreSQL)
	#[cfg(feature = "db-postgres")]
	{
		if config.identity_always == Some(true) || config.identity_by_default == Some(true) {
			return true;
		}
	}

	// Database-specific ID auto-generation (MySQL)
	#[cfg(feature = "db-mysql")]
	{
		if config.auto_increment == Some(true) {
			return true;
		}
	}

	// Database-specific ID auto-generation (SQLite)
	#[cfg(feature = "db-sqlite")]
	{
		if config.autoincrement == Some(true) {
			return true;
		}
	}

	// ManyToManyField - always auto-generated with Default::default()
	if is_many_to_many_field_type(&field.ty) {
		return true;
	}

	// ForeignKeyField/OneToOneField - always auto-generated with Default::default()
	if is_relationship_field_type(&field.ty) {
		return true;
	}

	// ManyToMany relationship via #[rel(many_to_many, ...)]
	if let Some(rel) = &field.rel
		&& matches!(rel.rel_type, crate::rel::RelationType::ManyToMany)
	{
		return true;
	}

	// UUID primary key is auto-generated with Uuid::new_v4()
	if config.primary_key && is_uuid_type(&field.ty) {
		return true;
	}

	false
}

/// Get the default value expression for an auto-generated field
fn get_auto_field_default_value(field: &FieldInfo) -> TokenStream {
	let config = &field.config;

	// ManyToManyField or ManyToMany relationship
	if is_many_to_many_field_type(&field.ty) {
		return quote! { ::std::default::Default::default() };
	}
	if let Some(rel) = &field.rel
		&& matches!(rel.rel_type, crate::rel::RelationType::ManyToMany)
	{
		return quote! { ::std::default::Default::default() };
	}

	// ForeignKeyField or OneToOneField - use Default::default()
	if is_relationship_field_type(&field.ty) {
		return quote! { ::std::default::Default::default() };
	}

	// Timestamp fields - use Utc::now()（修正：is_timestamp_fieldを使用）
	if is_timestamp_field(field) {
		return quote! { ::chrono::Utc::now() };
	}

	// UUID primary key - generate new UUID
	if config.primary_key && is_uuid_type(&field.ty) {
		let (is_option, _) = extract_option_type(&field.ty);
		if is_option {
			return quote! { Some(::uuid::Uuid::new_v4()) };
		} else {
			return quote! { ::uuid::Uuid::new_v4() };
		}
	}

	// Generated columns, IDENTITY, or auto-increment fields
	// These are set by the database, so use Default::default() (typically None for Option types)
	quote! { ::std::default::Default::default() }
}

/// Generate the new() constructor function for the model
fn generate_new_function(
	struct_name: &syn::Ident,
	field_infos: &[FieldInfo],
	fk_id_field_names: &[syn::Ident],
) -> TokenStream {
	// Separate user-specified fields from auto-generated fields
	let user_fields: Vec<_> = field_infos
		.iter()
		.filter(|f| !is_auto_generated_field(f))
		.collect();

	let auto_fields: Vec<_> = field_infos
		.iter()
		.filter(|f| is_auto_generated_field(f))
		.collect();

	// FK_idフィールドのマップを作成（例: room_id -> room）
	let fk_id_to_fk_field: HashMap<String, String> = fk_id_field_names
		.iter()
		.filter_map(|id_name| {
			let id_str = id_name.to_string();
			if id_str.ends_with("_id") {
				let fk_name = id_str.trim_end_matches("_id").to_string();
				Some((id_str, fk_name))
			} else {
				None
			}
		})
		.collect();

	// パラメータリストの生成
	let mut params = Vec::new();
	let mut where_clauses = Vec::new();
	let mut generic_params = Vec::new();
	let mut fk_field_assignments = Vec::new();
	let mut fk_id_assignments = Vec::new();

	// ジェネリック型パラメータのカウンター（F0, F1, F2, ...）
	let mut generic_counter = 0;

	for f in user_fields.iter() {
		let field_name = &f.name;
		let field_name_str = field_name.to_string();

		// このフィールドがFK _idフィールドかチェック
		if let Some(fk_field_name) = fk_id_to_fk_field.get(&field_name_str) {
			// これはFK _idフィールド（例: room_id）
			// ジェネリック型パラメータを使用
			let generic_param = syn::Ident::new(&format!("F{}", generic_counter), field_name.span());
			generic_counter += 1;

			// 対応するFKフィールドを見つける
			let fk_field_info = field_infos.iter().find(|fi| fi.name.to_string() == *fk_field_name);

			if let Some(fk_info) = fk_field_info {
				// ForeignKeyField<T>からT を抽出
				let related_model_type = extract_foreign_key_target_type(&fk_info.ty);

				// パラメータ: fk_field_name: GenericParam
				let fk_field_ident = syn::Ident::new(fk_field_name, field_name.span());
				params.push(quote! { #fk_field_ident: #generic_param });

				// Where句: GenericParam: IntoPrimaryKey<RelatedModel>
				where_clauses.push(quote! {
					#generic_param: ::reinhardt::db::orm::IntoPrimaryKey<#related_model_type>
				});

				// ジェネリックパラメータリスト
				generic_params.push(quote! { #generic_param });

				// フィールド割り当て: room_id: fk_field_name.into_primary_key()
				fk_id_assignments.push(quote! {
					#field_name: #fk_field_ident.into_primary_key()
				});
			}
		} else {
			// 通常のユーザーフィールド
			let ty = &f.ty;
			params.push(quote! { #field_name: #ty });
		}
	}

	// ForeignKeyFieldフィールドの割り当て（ForeignKeyField::new()）
	for (_fk_id_str, fk_name_str) in fk_id_to_fk_field.iter() {
		let fk_name = syn::Ident::new(fk_name_str, proc_macro2::Span::call_site());
		fk_field_assignments.push(quote! {
			#fk_name: ::std::default::Default::default()
		});
	}

	// 通常のユーザーフィールドの割り当て（FK関連以外）
	let user_field_assignments: Vec<_> = user_fields
		.iter()
		.filter(|f| !fk_id_to_fk_field.contains_key(&f.name.to_string()))
		.map(|f| {
			let name = &f.name;
			quote! { #name }
		})
		.collect();

	// Auto-generatedフィールドの割り当て
	let auto_field_assignments: Vec<_> = auto_fields
		.iter()
		.map(|f| {
			let name = &f.name;
			let default_value = get_auto_field_default_value(f);
			quote! { #name: #default_value }
		})
		.collect();

	// ジェネリック関数シグネチャの生成
	let generic_signature = if generic_params.is_empty() {
		quote! {}
	} else {
		quote! { <#(#generic_params),*> }
	};

	let where_clause = if where_clauses.is_empty() {
		quote! {}
	} else {
		quote! { where #(#where_clauses),* }
	};

	quote! {
		impl #struct_name {
			/// Create a new instance with user-specified fields.
			///
			/// Auto-generated fields are initialized automatically:
			/// - UUID primary keys: Generated with `Uuid::new_v4()`
			/// - Timestamp fields (created_at, updated_at, etc.): Set to `Utc::now()`
			/// - Fields with `#[field(auto_now_add)]` or `#[field(auto_now)]`: Set to `Utc::now()`
			/// - ManyToManyField: Initialized with `Default::default()`
			/// - ForeignKeyField: Initialized with `Default::default()`
			/// - Identity/AutoIncrement fields: Set to `Default::default()` (DB assigns value)
			///
			/// # Foreign Key Parameters
			///
			/// Foreign key fields accept either:
			/// - The related model instance (e.g., `User { ... }`)
			/// - A reference to the related model (e.g., `&user`)
			/// - The primary key value directly (e.g., `user_id: Uuid`)
			pub fn new #generic_signature(#(#params),*) -> Self
			#where_clause
			{
				Self {
					#(#user_field_assignments,)*
					#(#fk_id_assignments,)*
					#(#fk_field_assignments,)*
					#(#auto_field_assignments,)*
				}
			}
		}
	}
}
