//! Model derive macro for automatic ORM model registration
//!
//! Provides automatic `Model` trait implementation and registration to the global ModelRegistry.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Result, Type, parse_quote};

/// Model configuration from #[model(...)] attribute
#[derive(Debug, Clone)]
struct ModelConfig {
	app_label: String,
	table_name: String,
}

impl ModelConfig {
	/// Parse #[model(...)] attribute
	fn from_attrs(attrs: &[syn::Attribute], struct_name: &syn::Ident) -> Result<Self> {
		let mut app_label = None;
		let mut table_name = None;

		for attr in attrs {
			if !attr.path().is_ident("model") {
				continue;
			}

			attr.parse_nested_meta(|meta| {
				if meta.path.is_ident("app_label") {
					let value: syn::LitStr = meta.value()?.parse()?;
					app_label = Some(value.value());
					Ok(())
				} else if meta.path.is_ident("table_name") {
					let value: syn::LitStr = meta.value()?.parse()?;
					table_name = Some(value.value());
					Ok(())
				} else {
					Err(meta.error("unsupported model attribute"))
				}
			})?;
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
		})
	}
}

/// Field configuration from #[field(...)] attribute
#[derive(Debug, Clone, Default)]
struct FieldConfig {
	primary_key: bool,
	max_length: Option<u64>,
	null: Option<bool>,
	blank: Option<bool>,
	unique: Option<bool>,
	default: Option<String>,
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
}

impl FieldConfig {
	/// Parse #[field(...)] attribute
	fn from_attrs(attrs: &[syn::Attribute]) -> Result<Self> {
		let mut config = Self::default();

		for attr in attrs {
			if !attr.path().is_ident("field") {
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
					let value: syn::LitStr = meta.value()?.parse()?;
					config.default = Some(value.value());
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
				} else {
					Err(meta.error("unsupported field attribute"))
				}
			})?;
		}

		Ok(config)
	}
}

/// Field information for processing
#[derive(Debug, Clone)]
struct FieldInfo {
	name: syn::Ident,
	ty: Type,
	config: FieldConfig,
}

/// Map Rust type to ORM field type
fn map_type_to_field_type(ty: &Type, config: &FieldConfig) -> Result<String> {
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
				"i32" => "IntegerField",
				"i64" => "BigIntegerField",
				"String" => {
					if config.max_length.is_none() {
						return Err(syn::Error::new_spanned(
							ty,
							"String fields require max_length attribute",
						));
					}
					"CharField"
				}
				"bool" => "BooleanField",
				"DateTime" => "DateTimeField",
				"Date" => "DateField",
				"Time" => "TimeField",
				"f32" | "f64" => "FloatField",
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

	Ok(field_type.to_string())
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
				pub const fn #method_name() -> ::reinhardt_db::orm::expressions::FieldRef<#struct_name, #field_type> {
					::reinhardt_db::orm::expressions::FieldRef::new(#field_name_str)
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
	for field in fields {
		let name = field
			.ident
			.clone()
			.ok_or_else(|| syn::Error::new_spanned(field, "Field must have a name"))?;
		let ty = field.ty.clone();
		let config = FieldConfig::from_attrs(&field.attrs)?;

		field_infos.push(FieldInfo { name, ty, config });
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

	// Extract constraint names and expressions for code generation
	let constraint_names: Vec<String> = check_constraints
		.iter()
		.map(|(field_name, _)| format!("{}_check", field_name))
		.collect();
	let constraint_expressions: Vec<String> = check_constraints
		.iter()
		.map(|(_, expr)| expr.clone())
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
	let field_metadata_items = generate_field_metadata(&field_infos)?;

	// Generate auto-registration code
	let registration_code =
		generate_registration_code(struct_name, app_label, table_name, &field_infos)?;

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

	// Generate the Model implementation
	let expanded = quote! {
		// Generate composite PK type definition if needed
		#composite_pk_type_def

		// Generate field accessor methods for type-safe field references
		#field_accessors

		impl #generics ::reinhardt_db::orm::Model for #struct_name #generics #where_clause {
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

			fn field_metadata() -> Vec<::reinhardt_db::orm::inspection::FieldInfo> {
				vec![
					#(#field_metadata_items),*
				]
			}

			fn index_metadata() -> Vec<::reinhardt_db::orm::inspection::IndexInfo> {
				vec![
					#(
						::reinhardt_db::orm::inspection::IndexInfo {
							fields: vec![#indexed_fields.to_string()],
							unique: false,
							name: None,
						}
					),*
				]
			}

			fn constraint_metadata() -> Vec<::reinhardt_db::orm::inspection::ConstraintInfo> {
				vec![
					#(
						::reinhardt_db::orm::inspection::ConstraintInfo {
							name: #constraint_names.to_string(),
							constraint_type: ::reinhardt_db::orm::inspection::ConstraintType::Check,
							definition: #constraint_expressions.to_string(),
						}
					),*
				]
			}
		}

		#registration_code
	};

	Ok(expanded)
}

/// Generate FieldInfo construction for field_metadata()
fn generate_field_metadata(field_infos: &[FieldInfo]) -> Result<Vec<TokenStream>> {
	let mut items = Vec::new();

	for field_info in field_infos {
		let name = field_info.name.to_string();
		let field_type = map_type_to_field_type(&field_info.ty, &field_info.config)?;
		let field_type_path = format!("reinhardt.orm.models.{}", field_type);
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
					::reinhardt_db::orm::fields::FieldKwarg::Uint(#max_length)
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
					::reinhardt_db::orm::fields::FieldKwarg::Bool(true)
				);
			});
		}
		if let Some(url) = config.url
			&& url
		{
			attrs.push(quote! {
				attributes.insert(
					"url".to_string(),
					::reinhardt_db::orm::fields::FieldKwarg::Bool(true)
				);
			});
		}
		if let Some(min_length) = config.min_length {
			attrs.push(quote! {
				attributes.insert(
					"min_length".to_string(),
					::reinhardt_db::orm::fields::FieldKwarg::Uint(#min_length)
				);
			});
		}
		if let Some(min_value) = config.min_value {
			attrs.push(quote! {
				attributes.insert(
					"min_value".to_string(),
					::reinhardt_db::orm::fields::FieldKwarg::Int(#min_value)
				);
			});
		}
		if let Some(max_value) = config.max_value {
			attrs.push(quote! {
				attributes.insert(
					"max_value".to_string(),
					::reinhardt_db::orm::fields::FieldKwarg::Int(#max_value)
				);
			});
		}

		let item = quote! {
			{
				let mut attributes = ::std::collections::HashMap::new();
				#(#attrs)*

				::reinhardt_db::orm::inspection::FieldInfo {
					name: #name.to_string(),
					field_type: #field_type_path.to_string(),
					nullable: #nullable,
					primary_key: #primary_key,
					unique: #unique,
					blank: #blank,
					editable: #editable,
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
) -> Result<TokenStream> {
	let model_name = struct_name.to_string();
	let register_fn_name = syn::Ident::new(
		&format!(
			"__register_{}_model",
			struct_name.to_string().to_lowercase()
		),
		struct_name.span(),
	);

	// Generate field registration code
	let mut field_registrations = Vec::new();
	for field_info in field_infos {
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

		field_registrations.push(quote! {
			metadata.add_field(
				#field_name.to_string(),
				::reinhardt_db::migrations::model_registry::FieldMetadata::new(#field_type)
					#(#params)*
			);
		});
	}

	let code = quote! {
		#[::ctor::ctor]
		fn #register_fn_name() {
			use ::reinhardt_db::migrations::model_registry::*;

			let mut metadata = ModelMetadata::new(
				#app_label,
				#model_name,
				#table_name,
			);

			#(#field_registrations)*

			global_registry().register_model(metadata);
		}
	};

	Ok(code)
}

/// Generate composite primary key implementation
fn generate_composite_pk_impl(pk_fields: &[&FieldInfo]) -> TokenStream {
	let field_name_strings: Vec<String> = pk_fields.iter().map(|f| f.name.to_string()).collect();

	quote! {
		fn composite_primary_key() -> Option<::reinhardt_db::orm::composite_pk::CompositePrimaryKey> {
			Some(
				::reinhardt_db::orm::composite_pk::CompositePrimaryKey::new(
					vec![#(#field_name_strings.to_string()),*]
				)
				.expect("Invalid composite primary key")
			)
		}

		fn get_composite_pk_values(&self) -> ::std::collections::HashMap<String, ::reinhardt_db::orm::composite_pk::PkValue> {
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
					::reinhardt_db::orm::composite_pk::PkValue::from(&self.#name)
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
			pub fn to_pk_values(&self) -> ::std::collections::HashMap<String, ::reinhardt_db::orm::composite_pk::PkValue> {
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
	}
}
