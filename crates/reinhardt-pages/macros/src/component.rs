use std::collections::HashSet;

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
	FnArg, Ident, ItemFn, LitStr, Pat, PatTupleStruct, PatType, ReturnType, Token, Type,
	Visibility,
	parse::{Parse, ParseStream},
	parse_macro_input,
	punctuated::Punctuated,
};

use crate::crate_paths::get_reinhardt_pages_crate;

struct ComponentArgs {
	path: LitStr,
	name: LitStr,
}

impl Parse for ComponentArgs {
	fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
		let path: LitStr = input.parse()?;
		if input.is_empty() {
			return Err(input.error("expected #[component(\"/path/\", \"name\")]"));
		}
		input.parse::<Token![,]>()?;
		let name = if input.peek(LitStr) {
			input.parse()?
		} else {
			let ident: Ident = input.parse()?;
			LitStr::new(&ident.to_string(), ident.span())
		};
		if !input.is_empty() {
			return Err(input.error("expected #[component(\"/path/\", \"name\")]"));
		}
		Ok(Self { path, name })
	}
}

#[derive(Clone, Copy)]
enum Source {
	Path,
	Query,
}

struct ExtractedArg {
	source: Source,
	name: Ident,
	ty: Type,
	extractor_ty: Type,
}

pub(crate) fn component_impl(args: TokenStream, input: TokenStream) -> TokenStream {
	let args = parse_macro_input!(args as ComponentArgs);
	let input = parse_macro_input!(input as ItemFn);
	expand_component(args, input)
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

fn expand_component(args: ComponentArgs, input: ItemFn) -> syn::Result<proc_macro2::TokenStream> {
	if input.sig.asyncness.is_some() {
		return Err(syn::Error::new_spanned(
			input.sig.asyncness,
			"#[component] functions must not be async",
		));
	}
	if !input.sig.generics.params.is_empty() || input.sig.generics.where_clause.is_some() {
		return Err(syn::Error::new_spanned(
			input.sig.generics,
			"#[component] functions must not be generic",
		));
	}
	match &input.sig.output {
		ReturnType::Type(_, _) => {}
		ReturnType::Default => {
			return Err(syn::Error::new_spanned(
				&input.sig,
				"#[component] functions must return Page",
			));
		}
	}

	let pages_crate = get_reinhardt_pages_crate();
	let fn_name = input.sig.ident.clone();
	let component_name = fn_name.to_string().to_case(Case::Pascal);
	let component_ident = format_ident!("{component_name}", span = fn_name.span());
	let props_ident = format_ident!("{}Props", component_name, span = fn_name.span());
	let original_ident = format_ident!("__{}_component_body", fn_name, span = fn_name.span());
	let output = input.sig.output.clone();
	let vis = input.vis.clone();
	let field_vis = field_visibility_tokens(&vis);
	let attrs = input.attrs.clone();
	let block = input.block.clone();
	let args_info = parse_args(&input.sig.inputs)?;

	let props_fields = args_info.iter().map(|arg| {
		let name = &arg.name;
		let ty = &arg.ty;
		quote! { #field_vis #name: #ty }
	});
	let destructure_fields = args_info.iter().map(|arg| &arg.name);
	let original_inputs = args_info.iter().map(|arg| {
		let name = &arg.name;
		let ty = &arg.ty;
		quote! { #name: #ty }
	});
	let call_args = args_info.iter().map(|arg| &arg.name);
	let from_request_fields = args_info.iter().map(|arg| {
		let name = &arg.name;
		let key = name.to_string();
		let ty = &arg.ty;
		match arg.source {
			Source::Path => quote! {
				#name: #pages_crate::router::request::PathParam::<#ty>::extract(ctx, #key)?.into_inner()
			},
			Source::Query => quote! {
				#name: #pages_crate::router::request::QueryParam::<#ty>::extract(ctx, #key)?.into_inner()
			},
		}
	});
	let extractor_type_aliases = args_info.iter().enumerate().map(|(index, arg)| {
		let alias = format_ident!(
			"__{}PropsExtractor{}",
			component_name,
			index,
			span = fn_name.span()
		);
		let extractor_ty = &arg.extractor_ty;
		quote! {
			// Keep the extractor type from the original signature referenced after
			// the attribute macro rewrites the function into a props argument.
			#[allow(non_camel_case_types, dead_code)]
			type #alias = #extractor_ty;
		}
	});

	let path = args.path;
	let route_name = args.name;
	let fn_name_literal = fn_name.to_string();
	let props_type_literal = props_ident.to_string();
	let component_name_literal = component_ident.to_string();

	Ok(quote! {
		#[derive(::bon::Builder)]
		#vis struct #props_ident {
			#(#props_fields,)*
		}

		#(#extractor_type_aliases)*

		impl #pages_crate::router::request::FromRequest for #props_ident {
			fn from_request(
				ctx: &#pages_crate::router::request::RouteContext,
			) -> ::std::result::Result<Self, #pages_crate::router::request::ExtractError> {
				::std::result::Result::Ok(Self {
					#(#from_request_fields,)*
				})
			}
		}

		impl #pages_crate::__private::reinhardt_urls::routers::client_router::ComponentInfo
			for #props_ident
		{
			fn path() -> &'static str { #path }
			fn name() -> &'static str { #route_name }
			fn component_name() -> &'static str { #component_name_literal }
			fn function_name() -> &'static str { #fn_name_literal }
			fn props_type_name() -> &'static str { #props_type_literal }
		}

		#pages_crate::__private::inventory::submit! {
			#pages_crate::__private::reinhardt_urls::routers::client_router::ComponentMetadata {
				path: #path,
				name: #route_name,
				component_name: #component_name_literal,
				function_name: #fn_name_literal,
				props_type_name: #props_type_literal,
				module_path: ::core::module_path!(),
			}
		}

		fn #original_ident(#(#original_inputs,)*) #output {
			#block
		}

		#(#attrs)*
		#vis fn #fn_name(props: #props_ident) #output {
			let #props_ident { #(#destructure_fields,)* } = props;
			#original_ident(#(#call_args,)*)
		}
	})
}

fn field_visibility_tokens(vis: &Visibility) -> proc_macro2::TokenStream {
	match vis {
		Visibility::Inherited => quote! {},
		_ => quote! { #vis },
	}
}

fn parse_args(inputs: &Punctuated<FnArg, Token![,]>) -> syn::Result<Vec<ExtractedArg>> {
	let mut seen = HashSet::new();
	let mut out = Vec::new();

	for input in inputs {
		let FnArg::Typed(PatType { pat, ty, .. }) = input else {
			return Err(syn::Error::new_spanned(
				input,
				"#[component] does not support receiver arguments",
			));
		};

		let Pat::TupleStruct(PatTupleStruct { path, elems, .. }) = &**pat else {
			return Err(syn::Error::new_spanned(
				pat,
				"#[component] arguments must use extractor destructuring such as Path(id): Path<i64>",
			));
		};

		if elems.len() != 1 {
			return Err(syn::Error::new_spanned(
				elems,
				"extractor destructuring must bind exactly one identifier",
			));
		}

		let Pat::Ident(pat_ident) = &elems[0] else {
			return Err(syn::Error::new_spanned(
				&elems[0],
				"extractor destructuring must bind an identifier",
			));
		};
		let name = pat_ident.ident.clone();
		if !seen.insert(name.to_string()) {
			return Err(syn::Error::new_spanned(
				&name,
				format!("duplicate component props field `{name}`"),
			));
		}

		let source_ident = path
			.segments
			.last()
			.map(|seg| seg.ident.to_string())
			.ok_or_else(|| syn::Error::new_spanned(path, "expected Path or Query extractor"))?;
		let source = match source_ident.as_str() {
			"Path" => Source::Path,
			"Query" => Source::Query,
			_ => {
				return Err(syn::Error::new_spanned(
					path,
					"expected Path(...) or Query(...) extractor",
				));
			}
		};

		let inner_ty = extractor_inner_type(ty, source_ident.as_str())?;
		out.push(ExtractedArg {
			source,
			name,
			ty: inner_ty,
			extractor_ty: (**ty).clone(),
		});
	}

	Ok(out)
}

fn extractor_inner_type(ty: &Type, expected: &str) -> syn::Result<Type> {
	let Type::Path(type_path) = ty else {
		return Err(syn::Error::new_spanned(ty, "expected Path<T> or Query<T>"));
	};
	let segment = type_path
		.path
		.segments
		.last()
		.ok_or_else(|| syn::Error::new_spanned(ty, "expected Path<T> or Query<T>"))?;
	if segment.ident != expected {
		return Err(syn::Error::new_spanned(
			ty,
			"extractor pattern and argument type must match",
		));
	}
	let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
		return Err(syn::Error::new_spanned(ty, "expected Path<T> or Query<T>"));
	};
	let Some(syn::GenericArgument::Type(inner)) = args.args.first() else {
		return Err(syn::Error::new_spanned(ty, "expected Path<T> or Query<T>"));
	};
	Ok(inner.clone())
}
