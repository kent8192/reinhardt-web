use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};
/// Implementation of the `receiver` procedural macro
///
/// This function is used internally by the `#[receiver]` attribute macro.
/// Users should not call this function directly.
///
/// Note: This is a marker macro. The actual signal connection must be done
/// in the application's initialization code. This approach is more idiomatic
/// for Rust as signal registration typically happens at runtime.
pub fn receiver_impl(_args: TokenStream, input: TokenStream) -> TokenStream {
	let input_fn = parse_macro_input!(input as ItemFn);
	let _fn_name = &input_fn.sig.ident;
	let fn_vis = &input_fn.vis;
	let fn_block = &input_fn.block;
	let fn_attrs = &input_fn.attrs;
	let fn_sig = &input_fn.sig;

	// For now, we just pass through the function
	// In a real implementation, you would track these functions
	// and generate registration code in a build script or init function
	let expanded = quote! {
		#(#fn_attrs)*
		#fn_vis #fn_sig {
			#fn_block
		}
	};

	TokenStream::from(expanded)
}
