//! HTTP method route macros

use crate::path_macro;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Error, Expr, ExprLit, ItemFn, Lit, LitStr, Meta, Result, Token, parse::Parser,
    punctuated::Punctuated, spanned::Spanned,
};

/// Validate a route path at compile time
fn validate_route_path(path: &str, span: Span) -> Result<()> {
    path_macro::parse_and_validate(path)
        .map(|_| ())
        .map_err(|e| Error::new(span, format!("Invalid route path: {}", e)))
}

fn route_impl(method: &str, args: TokenStream, input: ItemFn) -> Result<TokenStream> {
    let mut path: Option<(String, Span)> = None;

    // Handle the common case: #[get("/users/{id}")]
    // Try to parse as a single string literal first
    if let Ok(lit) = syn::parse2::<LitStr>(args.clone()) {
        let path_str = lit.value();
        validate_route_path(&path_str, lit.span())?;
        path = Some((path_str, lit.span()));
    } else {
        // Parse path argument for other formats
        let meta_list = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(args)?;

        for meta in meta_list {
            match meta {
                Meta::Path(p) => {
                    if let Some(ident) = p.get_ident() {
                        let path_str = ident.to_string();
                        validate_route_path(&path_str, p.span())?;
                        path = Some((path_str, p.span()));
                    }
                }
                Meta::NameValue(nv) if nv.path.is_ident("path") => {
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Str(lit), ..
                    }) = &nv.value
                    {
                        let path_str = lit.value();
                        validate_route_path(&path_str, lit.span())?;
                        path = Some((path_str, lit.span()));
                    }
                }
                _ => {}
            }
        }
    }

    let fn_name = &input.sig.ident;
    let fn_block = &input.block;
    let fn_inputs = &input.sig.inputs;
    let fn_output = &input.sig.output;
    let fn_vis = &input.vis;
    let fn_attrs = &input.attrs;
    let asyncness = &input.sig.asyncness;
    let generics = &input.sig.generics;
    let where_clause = &input.sig.generics.where_clause;

    let route_doc = if let Some((p, _)) = &path {
        format!("Route: {} {}", method, p)
    } else {
        format!("HTTP Method: {}", method)
    };

    Ok(quote! {
        #(#fn_attrs)*
        #[doc = #route_doc]
        #fn_vis #asyncness fn #fn_name #generics (#fn_inputs) #fn_output #where_clause {
            #fn_block
        }
    })
}
/// Implementation of GET route macro
pub fn get_impl(args: TokenStream, input: ItemFn) -> Result<TokenStream> {
    route_impl("GET", args, input)
}
/// Implementation of POST route macro
pub fn post_impl(args: TokenStream, input: ItemFn) -> Result<TokenStream> {
    route_impl("POST", args, input)
}
/// Implementation of PUT route macro
pub fn put_impl(args: TokenStream, input: ItemFn) -> Result<TokenStream> {
    route_impl("PUT", args, input)
}
/// Implementation of PATCH route macro
pub fn patch_impl(args: TokenStream, input: ItemFn) -> Result<TokenStream> {
    route_impl("PATCH", args, input)
}
/// Implementation of DELETE route macro
pub fn delete_impl(args: TokenStream, input: ItemFn) -> Result<TokenStream> {
    route_impl("DELETE", args, input)
}
