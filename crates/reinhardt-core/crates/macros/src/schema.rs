//! `#[derive(Schema)]` macro implementation
//!
//! Automatically implements the `ToSchema` trait for structs,
//! generating OpenAPI schemas from Rust type definitions.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Fields, Lit, Meta, Type};

pub fn derive_schema_impl(input: DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let schema_name = name.to_string();

    match &input.data {
        Data::Struct(data_struct) => {
            let schema_body = generate_struct_schema(&data_struct.fields, &input.attrs)?;

            Ok(quote! {
                impl #impl_generics ::reinhardt_openapi::ToSchema for #name #ty_generics #where_clause {
                    fn schema() -> ::reinhardt_openapi::Schema {
                        #schema_body
                    }

                    fn schema_name() -> Option<String> {
                        Some(#schema_name.to_string())
                    }
                }
            })
        }
        Data::Enum(data_enum) => {
            // For enums, we generate a string schema with enum values
            let variants: Vec<_> = data_enum
                .variants
                .iter()
                .map(|v| v.ident.to_string())
                .collect();

            Ok(quote! {
                impl #impl_generics ::reinhardt_openapi::ToSchema for #name #ty_generics #where_clause {
                    fn schema() -> ::reinhardt_openapi::Schema {
                        use ::reinhardt_openapi::Schema;
                        use ::utoipa::openapi::schema::{SchemaType, Type, ObjectBuilder};

                        let obj = ObjectBuilder::new()
                            .schema_type(SchemaType::Type(Type::String))
                            .enum_values(Some([
                                #(serde_json::Value::String(#variants.to_string())),*
                            ]))
                            .build();

                        Schema::Object(obj)
                    }

                    fn schema_name() -> Option<String> {
                        Some(#schema_name.to_string())
                    }
                }
            })
        }
        Data::Union(_) => Err(syn::Error::new_spanned(
            input,
            "Schema derivation is not supported for unions",
        )),
    }
}

fn generate_struct_schema(fields: &Fields, _attrs: &[Attribute]) -> syn::Result<TokenStream> {
    match fields {
        Fields::Named(fields) => {
            let mut property_statements = Vec::new();
            let mut required_fields = Vec::new();

            for field in &fields.named {
                let field_name = field.ident.as_ref().unwrap();
                let field_name_str = field_name.to_string();
                let field_type = &field.ty;

                // Extract serde attributes
                let serde_attrs = extract_serde_attrs(&field.attrs)?;

                // Skip field if marked with #[serde(skip)]
                if serde_attrs.skip {
                    continue;
                }

                // Use renamed field name if provided
                let schema_field_name = serde_attrs.rename.as_ref().unwrap_or(&field_name_str);

                // Check if field is Option<T>
                let is_optional = is_option_type(field_type);

                // Field is required if:
                // - Not optional AND
                // - Not marked with #[serde(default)]
                if !is_optional && !serde_attrs.default {
                    required_fields.push(schema_field_name.clone());
                }

                // Extract description from doc comments
                let description = extract_doc_comment(&field.attrs);

                let property_with_description = if let Some(desc) = description {
                    quote! {
                        let obj = obj.property(
                            #schema_field_name,
                            {
                                let field_schema = <#field_type as ::reinhardt_openapi::ToSchema>::schema();
                                // Add description if the schema is an Object
                                use ::utoipa::openapi::schema::ObjectBuilder;
                                match field_schema {
                                    ::reinhardt_openapi::Schema::Object(inner_obj) => {
                                        // Convert Object to ObjectBuilder, add description, and build back
                                        ::reinhardt_openapi::Schema::Object(
                                            ObjectBuilder::from(inner_obj)
                                                .description(Some(#desc))
                                                .build()
                                        )
                                    },
                                    other => other
                                }
                            }
                        );
                    }
                } else {
                    quote! {
                        let obj = obj.property(
                            #schema_field_name,
                            <#field_type as ::reinhardt_openapi::ToSchema>::schema()
                        );
                    }
                };

                property_statements.push(property_with_description);
            }

            let required_setters = required_fields.iter().map(|field_name| {
                quote! {
                    .required(#field_name)
                }
            });

            Ok(quote! {
                {
                    use ::reinhardt_openapi::Schema;
                    use ::utoipa::openapi::schema::{SchemaType, Type, ObjectBuilder};

                    let obj = ObjectBuilder::new()
                        .schema_type(SchemaType::Type(Type::Object));

                    #(#property_statements)*

                    let obj = obj #(#required_setters)*;

                    Schema::Object(obj.build())
                }
            })
        }
        Fields::Unnamed(_) => {
            // Tuple structs - not commonly used for API schemas
            Ok(quote! {
                {
                    use ::reinhardt_openapi::Schema;
                    use ::utoipa::openapi::schema::{SchemaType, Type, ObjectBuilder};
                    Schema::Object(
                        ObjectBuilder::new()
                            .schema_type(SchemaType::Type(Type::Object))
                            .build()
                    )
                }
            })
        }
        Fields::Unit => {
            // Unit structs - represent as empty object
            Ok(quote! {
                {
                    use ::reinhardt_openapi::Schema;
                    use ::utoipa::openapi::schema::{SchemaType, Type, ObjectBuilder};
                    Schema::Object(
                        ObjectBuilder::new()
                            .schema_type(SchemaType::Type(Type::Object))
                            .build()
                    )
                }
            })
        }
    }
}

/// Check if a type is `Option<T>`
fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

/// Extract documentation comments from attributes
fn extract_doc_comment(attrs: &[Attribute]) -> Option<String> {
    let mut docs = Vec::new();

    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let Ok(meta) = attr.meta.require_name_value() {
                if let syn::Expr::Lit(expr_lit) = &meta.value {
                    if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                        let doc = lit_str.value().trim().to_string();
                        if !doc.is_empty() {
                            docs.push(doc);
                        }
                    }
                }
            }
        }
    }

    if docs.is_empty() {
        None
    } else {
        Some(docs.join(" "))
    }
}

/// Holds serde-related field information
#[derive(Debug, Default)]
struct SerdeFieldAttrs {
    /// Renamed field name from `#[serde(rename = "...")]`
    rename: Option<String>,
    /// Whether the field should be skipped entirely
    skip: bool,
    /// Whether the field should be skipped during serialization
    skip_serializing: bool,
    /// Whether the field should be skipped during deserialization
    skip_deserializing: bool,
    /// Whether the field has a default value
    default: bool,
}

/// Extract serde attributes from field attributes
///
/// Supports:
/// - `#[serde(rename = "new_name")]`
/// - `#[serde(skip)]`
/// - `#[serde(skip_serializing)]`
/// - `#[serde(skip_deserializing)]`
/// - `#[serde(default)]`
fn extract_serde_attrs(attrs: &[Attribute]) -> syn::Result<SerdeFieldAttrs> {
    let mut serde_attrs = SerdeFieldAttrs::default();

    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }

        match &attr.meta {
            Meta::List(meta_list) => {
                // Parse tokens inside #[serde(...)]
                meta_list.parse_nested_meta(|meta| {
                    if meta.path.is_ident("rename") {
                        let value = meta.value()?;
                        let lit: Lit = value.parse()?;
                        if let Lit::Str(lit_str) = lit {
                            serde_attrs.rename = Some(lit_str.value());
                        }
                    } else if meta.path.is_ident("skip") {
                        serde_attrs.skip = true;
                    } else if meta.path.is_ident("skip_serializing") {
                        serde_attrs.skip_serializing = true;
                    } else if meta.path.is_ident("skip_deserializing") {
                        serde_attrs.skip_deserializing = true;
                    } else if meta.path.is_ident("default") {
                        serde_attrs.default = true;
                    }
                    Ok(())
                })?;
            }
            Meta::Path(path) => {
                // Handle `#[serde(skip)]` without parentheses
                if path.is_ident("skip") {
                    serde_attrs.skip = true;
                }
            }
            _ => {}
        }
    }

    Ok(serde_attrs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_simple_struct() {
        let input: DeriveInput = parse_quote! {
            struct User {
                id: i64,
                username: String,
                email: Option<String>,
            }
        };

        let result = derive_schema_impl(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_enum() {
        let input: DeriveInput = parse_quote! {
            enum Status {
                Active,
                Inactive,
                Pending,
            }
        };

        let result = derive_schema_impl(input);
        assert!(result.is_ok());
    }
}
