//! Procedural macros for OpenAPI schema generation in Reinhardt.
//!
//! This crate provides derive macros and attribute macros for automatic
//! OpenAPI schema generation from Rust types.
//!
//! # Examples
//!
//! ```ignore
//! use reinhardt_openapi_macros::Schema;
//!
//! #[derive(Schema)]
//! struct User {
//!     /// User's unique identifier
//!     #[schema(example = "42")]
//!     id: i64,
//!
//!     /// User's full name
//!     #[schema(example = "John Doe")]
//!     name: String,
//!
//!     /// User's email address (optional)
//!     #[schema(example = "john@example.com")]
//!     email: Option<String>,
//! }
//! ```

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

mod schema;

use schema::{extract_field_attributes, FieldAttributes};

/// Derive macro for automatic OpenAPI schema generation.
///
/// This macro implements the `ToSchema` trait for your struct,
/// generating an OpenAPI schema based on the struct's fields and attributes.
///
/// # Attributes
///
/// ## Container Attributes
///
/// - `#[schema(title = "...")]` - Override the schema title (default: struct name)
/// - `#[schema(description = "...")]` - Schema description
/// - `#[schema(example = "...")]` - Example value for the entire struct
///
/// ## Field Attributes
///
/// - `#[schema(description = "...")]` - Field description (also reads doc comments)
/// - `#[schema(example = "...")]` - Example value for this field
/// - `#[schema(default)]` - Mark field as having a default value
/// - `#[schema(deprecated)]` - Mark field as deprecated
/// - `#[schema(read_only)]` - Field is read-only (GET responses only)
/// - `#[schema(write_only)]` - Field is write-only (POST/PUT requests only)
/// - `#[schema(format = "...")]` - OpenAPI format (e.g., "email", "uri", "date-time")
/// - `#[schema(minimum = N)]` - Minimum value for numbers
/// - `#[schema(maximum = N)]` - Maximum value for numbers
/// - `#[schema(min_length = N)]` - Minimum length for strings/arrays
/// - `#[schema(max_length = N)]` - Maximum length for strings/arrays
/// - `#[schema(pattern = "...")]` - Regex pattern for string validation
///
/// # Examples
///
/// ```ignore
/// use reinhardt_openapi_macros::Schema;
///
/// #[derive(Schema)]
/// #[schema(title = "User", description = "A user account")]
/// struct User {
///     #[schema(description = "Unique user ID", example = "123")]
///     id: i64,
///
///     #[schema(description = "Username", min_length = 3, max_length = 50)]
///     username: String,
///
///     #[schema(description = "Email address", format = "email")]
///     email: String,
///
///     #[schema(description = "User's age", minimum = 0, maximum = 150)]
///     age: Option<i32>,
/// }
/// ```
#[proc_macro_derive(Schema, attributes(schema))]
pub fn derive_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Only support structs for now
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    "Schema can only be derived for structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(&input, "Schema can only be derived for structs")
                .to_compile_error()
                .into();
        }
    };

    // Extract container-level attributes
    let struct_name = name.to_string();

    // Generate schema for each field
    let mut field_schemas = Vec::new();
    let mut required_fields = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let field_type = &field.ty;

        // Extract field attributes
        let attrs = extract_field_attributes(&field.attrs);

        // Check if field is Option<T> (makes it optional)
        let is_option = is_option_type(field_type);
        if !is_option {
            required_fields.push(field_name_str.clone());
        }

        // Build field schema with attributes
        let schema_builder = build_field_schema(field_type, &attrs);

        field_schemas.push(quote! {
            builder = builder.property(#field_name_str, #schema_builder);
        });
    }

    // Add required fields
    let required_builder = if !required_fields.is_empty() {
        quote! {
            #(builder = builder.required(#required_fields);)*
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        impl #impl_generics ::reinhardt_openapi::ToSchema for #name #ty_generics #where_clause {
            fn schema() -> ::reinhardt_openapi::Schema {
                use ::reinhardt_openapi::Schema;
                use ::utoipa::openapi::schema::{ObjectBuilder, SchemaType, Type};

                let mut builder = ObjectBuilder::new()
                    .schema_type(SchemaType::Type(Type::Object));

                #(#field_schemas)*
                #required_builder

                Schema::Object(builder.build())
            }

            fn schema_name() -> Option<String> {
                Some(#struct_name.to_string())
            }
        }
    };

    TokenStream::from(expanded)
}

/// Helper function to check if a type is Option<T>
fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

/// Build schema for a field type with attributes
fn build_field_schema(field_type: &syn::Type, attrs: &FieldAttributes) -> proc_macro2::TokenStream {
    let base_schema = quote! {
        <#field_type as ::reinhardt_openapi::ToSchema>::schema()
    };

    // If no attributes, return base schema
    if attrs.is_empty() {
        return base_schema;
    }

    // Build schema with attributes applied
    let mut modifications = Vec::new();

    if let Some(ref description) = attrs.description {
        modifications.push(quote! {
            if let Schema::Object(mut obj) = schema {
                obj.description = Some(#description.to_string());
                schema = Schema::Object(obj);
            }
        });
    }

    if let Some(ref example) = attrs.example {
        modifications.push(quote! {
            if let Schema::Object(mut obj) = schema {
                obj.example = Some(serde_json::json!(#example));
                schema = Schema::Object(obj);
            }
        });
    }

    if let Some(ref format) = attrs.format {
        modifications.push(quote! {
            if let Schema::Object(mut obj) = schema {
                obj.format = Some(#format.to_string());
                schema = Schema::Object(obj);
            }
        });
    }

    if attrs.read_only {
        modifications.push(quote! {
            if let Schema::Object(mut obj) = schema {
                obj.read_only = Some(true);
                schema = Schema::Object(obj);
            }
        });
    }

    if attrs.write_only {
        modifications.push(quote! {
            if let Schema::Object(mut obj) = schema {
                obj.write_only = Some(true);
                schema = Schema::Object(obj);
            }
        });
    }

    if attrs.deprecated {
        modifications.push(quote! {
            if let Schema::Object(mut obj) = schema {
                obj.deprecated = Some(true);
                schema = Schema::Object(obj);
            }
        });
    }

    if let Some(min) = attrs.minimum {
        modifications.push(quote! {
            if let Schema::Object(mut obj) = schema {
                obj.minimum = Some(#min as f64);
                schema = Schema::Object(obj);
            }
        });
    }

    if let Some(max) = attrs.maximum {
        modifications.push(quote! {
            if let Schema::Object(mut obj) = schema {
                obj.maximum = Some(#max as f64);
                schema = Schema::Object(obj);
            }
        });
    }

    if let Some(min_len) = attrs.min_length {
        modifications.push(quote! {
            if let Schema::Object(mut obj) = schema {
                obj.min_length = Some(#min_len);
                schema = Schema::Object(obj);
            }
        });
    }

    if let Some(max_len) = attrs.max_length {
        modifications.push(quote! {
            if let Schema::Object(mut obj) = schema {
                obj.max_length = Some(#max_len);
                schema = Schema::Object(obj);
            }
        });
    }

    if let Some(ref pattern) = attrs.pattern {
        modifications.push(quote! {
            if let Schema::Object(mut obj) = schema {
                obj.pattern = Some(#pattern.to_string());
                schema = Schema::Object(obj);
            }
        });
    }

    if modifications.is_empty() {
        base_schema
    } else {
        quote! {
            {
                let mut schema = #base_schema;
                #(#modifications)*
                schema
            }
        }
    }
}
