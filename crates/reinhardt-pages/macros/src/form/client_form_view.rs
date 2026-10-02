//! Expands named-view syntax through the companion's typed presentation adapter.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use reinhardt_manouche::core::{ClientFormViewClause, ClientFormViewMacro, PresentationProperty};

pub(super) fn generate(ast: ClientFormViewMacro) -> TokenStream {
	let pages = crate::crate_paths::get_reinhardt_pages_crate();
	let support = quote!(#pages::__private::client_form);
	let companion = ast.companion;
	let options = format_ident!("__reinhardt_view_options", span = Span::mixed_site());
	let view = format_ident!("__reinhardt_view", span = Span::mixed_site());
	let field = format_ident!("__reinhardt_field", span = Span::mixed_site());
	let mut evaluations = Vec::new();
	let mut settings = Vec::new();
	let mut customizations = Vec::new();
	let mut mutation = TokenStream::new();

	for clause in ast.clauses {
		let context = match &clause {
			ClientFormViewClause::Submit(_) => "submit",
			ClientFormViewClause::Summary(_) => "summary",
			_ => "styling",
		};
		match clause {
			ClientFormViewClause::Mutation(expression) => {
				let temporary =
					format_ident!("__reinhardt_view_mutation", span = Span::mixed_site());
				evaluations.push(quote!(let #temporary = #expression;));
				mutation = quote!(#temporary);
			}
			ClientFormViewClause::Id(expression) => {
				let temporary = evaluate_string(&expression, &mut evaluations);
				settings.push(quote!(let #options = #options.id(#temporary);));
			}
			ClientFormViewClause::Styling(properties)
			| ClientFormViewClause::Submit(properties)
			| ClientFormViewClause::Summary(properties) => {
				for property in properties {
					let property_name = property.name.to_string();
					let mapped = match (context, property_name.as_str()) {
						("submit", "label") => "submit_label",
						("submit", "class") => "submit_class",
						("summary", "label") => "summary_label",
						_ => property_name.as_str(),
					};
					let name = syn::Ident::new(mapped, property.name.span());
					let temporary = evaluate_string(&property.value, &mut evaluations);
					settings.push(
						quote_spanned!(name.span()=> let #options = #options.#name(#temporary);),
					);
				}
			}
			ClientFormViewClause::Customize(fields) => {
				for customization in fields {
					let name = customization.field.to_string();
					let method = format_ident!(
						"__reinhardt_field_{}",
						name.trim_start_matches("r#"),
						span = customization.field.span()
					);
					let setters = customization
						.properties
						.into_iter()
						.map(|PresentationProperty { name, value }| {
							if name == "widget" {
								quote_spanned!(name.span()=> .widget(#support::#value))
							} else {
								let temporary = evaluate_string(&value, &mut evaluations);
								quote_spanned!(name.span()=> .#name(#temporary))
							}
						})
						.collect::<Vec<_>>();
					customizations
						.push(quote!(let #view = #view.#method(|#field| #field #(#setters)*);));
				}
			}
		}
	}
	quote! {{
		#(#evaluations)*
		let #options = #support::ViewOptions::default();
		#(#settings)*
		let #view = #companion::__reinhardt_view(#mutation, #options);
		#(#customizations)*
		#view
	}}
}

fn evaluate_string(expression: &syn::Expr, evaluations: &mut Vec<TokenStream>) -> syn::Ident {
	let temporary = format_ident!(
		"__reinhardt_view_value_{}",
		evaluations.len(),
		span = Span::mixed_site()
	);
	evaluations.push(quote! {
		let #temporary: ::std::string::String = ::core::convert::Into::into(#expression);
	});
	temporary
}
