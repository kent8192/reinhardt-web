//! Permission decorator macro

use crate::permission_macro;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    Error, Expr, ExprLit, ItemFn, Lit, LitStr, Meta, Result, Token, parse::Parser,
    punctuated::Punctuated, spanned::Spanned,
};

/// Validate a single permission string at compile time
fn validate_permission(permission: &str, span: Span) -> Result<()> {
    permission_macro::parse_and_validate(permission)
        .map(|_| ())
        .map_err(|e| Error::new(span, format!("Invalid permission string: {}", e)))
}
/// Implementation of the `permission_required` procedural macro
///
/// This function is used internally by the `#[permission_required]` attribute macro.
/// Users should not call this function directly.
pub fn permission_required_impl(args: TokenStream, input: ItemFn) -> Result<TokenStream> {
    let mut permissions = Vec::new();

    // Handle the common case: #[permission_required("auth.view_user")]
    // Try to parse as a single string literal first
    if let Ok(lit) = syn::parse2::<LitStr>(args.clone()) {
        let perm_str = lit.value();
        validate_permission(&perm_str, lit.span())?;
        permissions.push(perm_str);
    } else {
        // Parse permission arguments for other formats
        let meta_list = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(args)?;

        for meta in meta_list {
            match meta {
                Meta::Path(p) => {
                    if let Some(ident) = p.get_ident() {
                        let perm_str = ident.to_string();
                        validate_permission(&perm_str, p.span())?;
                        permissions.push(perm_str);
                    }
                }
                Meta::NameValue(nv) if nv.path.is_ident("permissions") => {
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Str(lit), ..
                    }) = &nv.value
                    {
                        // Parse permissions array
                        let perms_str = lit.value();
                        let perms_str = perms_str.trim_matches(|c| c == '[' || c == ']');

                        for perm in perms_str.split(',') {
                            let perm = perm.trim().trim_matches('"');
                            validate_permission(perm, lit.span())?;
                            permissions.push(perm.to_string());
                        }
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

    let perm_list = permissions.join(", ");
    let perm_doc = format!("Required permissions: {}", perm_list);

    Ok(quote! {
        #(#fn_attrs)*
        #[doc = #perm_doc]
        #fn_vis #asyncness fn #fn_name(#fn_inputs) #fn_output {
            // In a real implementation, this would check permissions first
            #fn_block
        }
    })
}
