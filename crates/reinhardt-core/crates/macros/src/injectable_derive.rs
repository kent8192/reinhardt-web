//! Injectable derive macro for automatic dependency injection
//!
//! Provides automatic `Injectable` trait implementation for structs with `#[inject]` fields.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Result, Type};

/// Check if a field has #[inject] attribute
fn has_inject_attr(field: &syn::Field) -> bool {
    field
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("inject"))
}

/// Check if #[inject] has cache = false
fn should_use_cache(field: &syn::Field) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident("inject") {
            if let Ok(meta) = attr.parse_args::<syn::Meta>() {
                if let syn::Meta::NameValue(nv) = meta {
                    if nv.path.is_ident("cache") {
                        if let syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Bool(lit_bool),
                            ..
                        }) = &nv.value
                        {
                            return lit_bool.value;
                        }
                    }
                }
            }
        }
    }
    true // Default: use cache
}

/// Field information for processing
struct FieldInfo {
    name: syn::Ident,
    ty: Type,
    inject: bool,
    use_cache: bool,
}

/// Implementation of the `Injectable` derive macro
///
/// Generates an `Injectable` trait implementation for structs with `#[inject]` fields.
pub fn injectable_derive_impl(input: DeriveInput) -> Result<TokenStream> {
    let struct_name = &input.ident;
    let generics = &input.generics;
    let where_clause = &generics.where_clause;

    // Only support structs
    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    struct_name,
                    "Injectable can only be derived for structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                struct_name,
                "Injectable can only be derived for structs",
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
        let inject = has_inject_attr(field);
        let use_cache = should_use_cache(field);

        field_infos.push(FieldInfo {
            name,
            ty,
            inject,
            use_cache,
        });
    }

    // Generate injection code for #[inject] fields
    let mut inject_stmts = Vec::new();
    for field_info in &field_infos {
        if field_info.inject {
            let name = &field_info.name;
            let ty = &field_info.ty;

            let resolve_call = if field_info.use_cache {
                quote! {
                    {
                        let __depends = ::reinhardt_di::Depends::<#ty>::new()
                            .resolve(__di_ctx)
                            .await
                            .map_err(|e| {
                                eprintln!("Dependency injection failed for {} in {}: {:?}",
                                    stringify!(#name), stringify!(#struct_name), e);
                                e
                            })?;
                        (*__depends).clone()
                    }
                }
            } else {
                quote! {
                    {
                        let __depends = ::reinhardt_di::Depends::<#ty>::no_cache()
                            .resolve(__di_ctx)
                            .await
                            .map_err(|e| {
                                eprintln!("Dependency injection failed for {} in {}: {:?}",
                                    stringify!(#name), stringify!(#struct_name), e);
                                e
                            })?;
                        (*__depends).clone()
                    }
                }
            };

            inject_stmts.push(quote! {
                let #name = #resolve_call;
            });
        }
    }

    // Generate field initialization
    let mut field_inits = Vec::new();
    for field_info in &field_infos {
        let name = &field_info.name;
        if field_info.inject {
            // Use the injected value
            field_inits.push(quote! { #name });
        } else {
            // Use Default::default() for non-injected fields
            field_inits.push(quote! { #name: Default::default() });
        }
    }

    // Generate the Injectable implementation
    let expanded = quote! {
        #[::async_trait::async_trait]
        impl #generics ::reinhardt_di::Injectable for #struct_name #generics #where_clause {
            async fn inject(__di_ctx: &::reinhardt_di::InjectionContext)
                -> ::reinhardt_di::DiResult<Self>
            {
                #(#inject_stmts)*

                Ok(Self {
                    #(#field_inits),*
                })
            }
        }
    };

    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_injectable_derive_simple() {
        let input: DeriveInput = parse_quote! {
            struct TestStruct {
                #[inject]
                db: Database,
                name: String,
            }
        };

        let result = injectable_derive_impl(input);
        assert!(result.is_ok());

        let output = result.unwrap().to_string();
        assert!(output.contains("Injectable"));
        assert!(output.contains("inject"));
        assert!(output.contains("Database"));
    }

    #[test]
    fn test_injectable_derive_with_cache_control() {
        let input: DeriveInput = parse_quote! {
            struct TestStruct {
                #[inject(cache = false)]
                db: Database,
            }
        };

        let result = injectable_derive_impl(input);
        assert!(result.is_ok());

        let output = result.unwrap().to_string();
        assert!(output.contains("no_cache"));
    }

    #[test]
    fn test_injectable_derive_enum_error() {
        let input: DeriveInput = parse_quote! {
            enum TestEnum {
                Variant1,
                Variant2,
            }
        };

        let result = injectable_derive_impl(input);
        assert!(result.is_err());
    }
}
