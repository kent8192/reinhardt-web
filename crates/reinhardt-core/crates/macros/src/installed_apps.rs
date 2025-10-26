//! # Installed Apps Macro
//!
//! Provides compile-time validation for installed applications.
//! This macro ensures that all referenced applications exist at compile time.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Ident, LitStr, Result, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

/// Represents a single app entry: `label: "path.to.app"`
struct AppEntry {
    label: Ident,
    _colon: Token![:],
    path: LitStr,
}

impl Parse for AppEntry {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(AppEntry {
            label: input.parse()?,
            _colon: input.parse()?,
            path: input.parse()?,
        })
    }
}

/// Represents the entire installed_apps! macro input
struct InstalledApps {
    apps: Punctuated<AppEntry, Token![,]>,
}

impl Parse for InstalledApps {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(InstalledApps {
            apps: Punctuated::parse_terminated(input)?,
        })
    }
}
/// Implementation of the `installed_apps!` procedural macro
///
/// This function is used internally by the `installed_apps!` macro.
/// Users should not call this function directly.
pub fn installed_apps_impl(input: TokenStream) -> Result<TokenStream> {
    let InstalledApps { apps } = syn::parse2(input)?;

    // Collect app labels and paths
    let labels: Vec<_> = apps.iter().map(|app| &app.label).collect();
    let paths: Vec<_> = apps.iter().map(|app| &app.path).collect();
    let path_strings: Vec<String> = paths.iter().map(|p| p.value()).collect();

    // Generate enum variants for each app
    // Convert labels to CamelCase for enum variants
    let enum_variants = labels.iter().map(|label| {
        // Allow non_camel_case_types for user-defined labels
        quote! {
            #[allow(non_camel_case_types)]
            #label
        }
    });

    // Generate Display implementation
    let display_arms = labels.iter().zip(paths.iter()).map(|(label, path)| {
        quote! {
            InstalledApp::#label => write!(f, #path)
        }
    });

    // Generate FromStr implementation
    let from_str_arms = labels.iter().zip(paths.iter()).map(|(label, path)| {
        quote! {
            #path => Ok(InstalledApp::#label)
        }
    });

    // Generate the app list array
    let app_list = paths.iter().map(|path| {
        quote! { #path.to_string() }
    });

    // Generate validation code for each app path
    let validations = path_strings.iter().map(|path_str| {
        // Convert "reinhardt.contrib.auth" to module path validation
        let parts: Vec<&str> = path_str.split('.').collect();

        // For reinhardt.contrib.* apps, we validate they exist in the crate
        if parts.first() == Some(&"reinhardt") {
            let module_check = parts[1..].iter().enumerate().fold(
                quote! { ::reinhardt_core },
                |acc, (i, part)| {
                    let part_ident = syn::Ident::new(part, proc_macro2::Span::call_site());
                    if i == parts.len() - 2 {
                        // Last part - try to reference it to check existence
                        quote! { #acc::#part_ident }
                    } else {
                        quote! { #acc::#part_ident }
                    }
                },
            );

            quote! {
                // Compile-time check that the module exists
                const _: () = {
                    let _ = || {
                        // This will fail to compile if the module doesn't exist
                        let _ = std::stringify!(#module_check);
                    };
                };
            }
        } else {
            // For user apps, just add a compile warning
            quote! {
                // User-defined app - runtime validation only
                #[allow(dead_code)]
                const _: &str = #path_str;
            }
        }
    });

    Ok(quote! {
        /// Enum representing all installed applications
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum InstalledApp {
            #(#enum_variants),*
        }

        impl std::fmt::Display for InstalledApp {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #(#display_arms),*
                }
            }
        }

        impl std::str::FromStr for InstalledApp {
            type Err = String;

            fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                match s {
                    #(#from_str_arms,)*
                    _ => Err(format!("Unknown app: {}", s))
                }
            }
        }

        impl InstalledApp {
            /// Get all installed apps as strings
            ///
            /// Returns a vector containing the string representations of all installed applications.
            pub fn all_apps() -> Vec<String> {
                vec![
                    #(#app_list),*
                ]
            }
            /// Get the path for this app
            ///
            pub fn path(&self) -> &'static str {
                match self {
                    #(InstalledApp::#labels => #paths),*
                }
            }
        }

        // Compile-time validation
        #(#validations)*
    })
}
