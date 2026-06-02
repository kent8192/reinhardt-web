use proc_macro::TokenStream;

use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Ident, parse_macro_input, parse_quote};

use crate::crate_paths::get_reinhardt_pages_crate_info;

pub(crate) fn derive_form_values(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	match expand_form_values(&input) {
		Ok(tokens) => tokens.into(),
		Err(err) => err.to_compile_error().into(),
	}
}

fn expand_form_values(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
	let values_ident = &input.ident;
	let fields_ident = fields_ident(values_ident);
	let named = match &input.data {
		Data::Struct(data) => match &data.fields {
			Fields::Named(named) => named,
			_ => {
				return Err(syn::Error::new_spanned(
					values_ident,
					"FormValues can only be derived for structs with named fields",
				));
			}
		},
		_ => {
			return Err(syn::Error::new_spanned(
				values_ident,
				"FormValues can only be derived for structs with named fields",
			));
		}
	};

	let field_idents: Vec<_> = named
		.named
		.iter()
		.map(|field| field.ident.as_ref().expect("named field"))
		.collect();
	let field_types: Vec<_> = named.named.iter().map(|field| &field.ty).collect();
	let field_visibilities: Vec<_> = named.named.iter().map(|field| &field.vis).collect();
	let field_names: Vec<_> = field_idents.iter().map(|ident| ident.to_string()).collect();
	let (struct_generics, type_generics, struct_where_clause) = input.generics.split_for_impl();
	let mut trait_generics = input.generics.clone();
	{
		let where_clause = trait_generics.make_where_clause();
		where_clause
			.predicates
			.push(parse_quote!(#values_ident #type_generics: ::core::clone::Clone + ::core::cmp::PartialEq + 'static));
		for field_type in &field_types {
			where_clause
				.predicates
				.push(parse_quote!(#field_type: ::core::clone::Clone + 'static));
		}
	}
	let (impl_generics, _, trait_where_clause) = trait_generics.split_for_impl();
	let visibility = &input.vis;
	let crate_info = get_reinhardt_pages_crate_info();
	let use_statement = &crate_info.use_statement;
	let pages_crate = &crate_info.ident;

	Ok(quote! {
		#use_statement

		#[derive(Clone)]
		#visibility struct #fields_ident #struct_generics #struct_where_clause {
			#(
				#field_visibilities #field_idents: #pages_crate::Signal<#field_types>,
			)*
		}

		impl #impl_generics #pages_crate::FormFields for #fields_ident #type_generics #trait_where_clause {
			type Values = #values_ident #type_generics;

			fn from_values(values: &Self::Values) -> Self {
				Self {
					#(
						#field_idents: #pages_crate::Signal::new(values.#field_idents.clone()),
					)*
				}
			}

			fn values(&self) -> Self::Values {
				#values_ident {
					#(
						#field_idents: self.#field_idents.get(),
					)*
				}
			}

			fn apply_values(&self, values: &Self::Values) {
				#(
					self.#field_idents.set(values.#field_idents.clone());
				)*
			}
		}

		impl #impl_generics #pages_crate::FormValues for #values_ident #type_generics #trait_where_clause {
			type Fields = #fields_ident #type_generics;

			fn field_names() -> &'static [&'static str] {
				&[
					#(
						#field_names,
					)*
				]
			}
		}
	})
}

fn fields_ident(values_ident: &Ident) -> Ident {
	let name = values_ident.to_string();
	let fields_name = name
		.strip_suffix("Values")
		.map(|prefix| format!("{prefix}Fields"))
		.unwrap_or_else(|| format!("{name}Fields"));
	format_ident!("{}", fields_name, span = Span::call_site())
}
