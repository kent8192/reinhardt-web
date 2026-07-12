use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, quote_spanned};

use crate::crate_paths::get_reinhardt_pages_crate_info;

pub(super) fn generate_style_items(
	item: &syn::ItemStatic,
	style_type: &Ident,
	compiled: &reinhardt_manouche::CompiledStyle,
) -> syn::Result<TokenStream> {
	let crate_info = get_reinhardt_pages_crate_info();
	let (pages, use_statement) = if crate_info.needs_conditional {
		let alias = format_ident!("__reinhardt_pages_for_{style_type}");
		(
			quote!(#alias),
			quote! {
				#[cfg(all(target_family = "wasm", target_os = "unknown"))]
				use ::reinhardt_pages as #alias;
				#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
				use ::reinhardt::pages as #alias;
			},
		)
	} else {
		(crate_info.ident, crate_info.use_statement)
	};
	let attributes = &item.attrs;
	let visibility = &item.vis;
	let static_name = &item.ident;
	let builder = format_ident!("{}Vars", style_type);
	let variable_count = compiled.variables.len();
	// Generated public methods inherit the authored item's effective visibility through
	// their enclosing type, so the reachability lint would otherwise reject valid private APIs.
	let generated_reachability_allow = quote!(#[allow(unreachable_pub)]);

	let class_accessors = compiled.classes.iter().map(|class| {
		let accessor = format_ident!("{}", class.accessor, span = class.span);
		let css_name = &class.css_name;
		quote_spanned! {class.span=>
			/// Returns the generated scoped class token.
			pub const fn #accessor(&self) -> #pages::ClassToken {
				#pages::ClassToken::new(#css_name)
			}
		}
	});

	let setters = compiled.variables.iter().map(|variable| {
		let setter = format_ident!("{}", variable.authored_name, span = variable.span);
		let custom_property = &variable.custom_property_name;
		let source_index = variable.source_index;
		let runtime_type = runtime_type_path(variable.runtime_type, &pages);
		quote_spanned! {variable.span=>
			/// Sets this generated component variable override.
			pub fn #setter(mut self, value: #runtime_type) -> Self {
				self.inner.set(#source_index, #custom_property, value);
				self
			}
		}
	});

	Ok(quote! {
		#use_statement

		#(#attributes)*
		#visibility struct #style_type;

		#(#attributes)*
		#visibility static #static_name: #style_type = #style_type;

		#(#attributes)*
		#generated_reachability_allow
		impl #style_type {
			#(#class_accessors)*

			/// Starts an ordered component-variable override builder.
			pub fn vars(&self) -> #builder {
				#builder { inner: #pages::StyleVars::with_slots(#variable_count) }
			}
		}

		#(#attributes)*
		#visibility struct #builder {
			inner: #pages::StyleVars,
		}

		#(#attributes)*
		#generated_reachability_allow
		impl #builder {
			#(#setters)*
		}

		#(#attributes)*
		impl ::std::convert::From<#builder> for ::std::borrow::Cow<'static, str> {
			fn from(value: #builder) -> Self {
				value.inner.into()
			}
		}
	})
}

fn runtime_type_path(
	runtime_type: reinhardt_manouche::StyleRuntimeType,
	pages: &TokenStream,
) -> TokenStream {
	use reinhardt_manouche::StyleRuntimeType;

	match runtime_type {
		StyleRuntimeType::Color => quote!(#pages::CssColor),
		StyleRuntimeType::Length => quote!(#pages::CssLength),
		StyleRuntimeType::LengthPercentage => quote!(#pages::CssLengthPercentage),
		StyleRuntimeType::Percentage => quote!(#pages::CssPercentage),
		StyleRuntimeType::Angle => quote!(#pages::CssAngle),
		StyleRuntimeType::Time => quote!(#pages::CssTime),
		StyleRuntimeType::Number => quote!(#pages::CssNumber),
		StyleRuntimeType::Integer => quote!(#pages::CssInteger),
	}
}
