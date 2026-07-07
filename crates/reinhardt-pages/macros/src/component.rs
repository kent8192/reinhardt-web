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

struct LayoutArgs {
	path: LitStr,
	name: LitStr,
}

const COMPONENT_ARGS_EXPECTED: &str = "expected #[component(\"/path/\", name = \"name\")]";
const LAYOUT_ARGS_EXPECTED: &str = "expected #[layout(\"/path/\", name = \"name\")]";

impl Parse for ComponentArgs {
	fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
		let (path, name) = parse_route_macro_args(input, "component", COMPONENT_ARGS_EXPECTED)?;
		Ok(Self { path, name })
	}
}

impl Parse for LayoutArgs {
	fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
		let (path, name) = parse_route_macro_args(input, "layout", LAYOUT_ARGS_EXPECTED)?;
		Ok(Self { path, name })
	}
}

fn parse_route_macro_args(
	input: ParseStream<'_>,
	macro_name: &str,
	expected: &str,
) -> syn::Result<(LitStr, LitStr)> {
	let path: LitStr = input.parse()?;
	if input.is_empty() {
		return Err(input.error(expected));
	}
	input.parse::<Token![,]>()?;
	if input.peek(LitStr) {
		let name: LitStr = input.parse()?;
		return Err(syn::Error::new(
			name.span(),
			"expected named route argument `name = \"...\"`; positional route names are no longer supported",
		));
	}
	let key: Ident = input.parse()?;
	if key != "name" {
		return Err(syn::Error::new(
			key.span(),
			"expected route name argument `name = \"...\"`",
		));
	}
	input.parse::<Token![=]>()?;
	if !input.peek(LitStr) {
		return Err(input.error(format!(
			"expected string literal route name in #[{macro_name}(\"/path/\", name = \"name\")]"
		)));
	}
	let name: LitStr = input.parse()?;
	if !input.is_empty() {
		return Err(input.error(expected));
	}
	Ok((path, name))
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

struct LayoutFunctionArgs {
	extracted: Vec<ExtractedArg>,
	outlet_name: Ident,
	outlet_ty: Type,
}

pub(crate) fn component_impl(args: TokenStream, input: TokenStream) -> TokenStream {
	let args = parse_macro_input!(args as ComponentArgs);
	let input = parse_macro_input!(input as ItemFn);
	expand_component(args, input)
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

pub(crate) fn layout_impl(args: TokenStream, input: TokenStream) -> TokenStream {
	let args = parse_macro_input!(args as LayoutArgs);
	let input = parse_macro_input!(input as ItemFn);
	expand_layout(args, input)
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
		ReturnType::Type(_, ty) if is_page_type(ty) => {}
		_ => {
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
		#[derive(#pages_crate::__private::bon::Builder)]
		#[builder(crate = #pages_crate::__private::bon)]
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

fn expand_layout(args: LayoutArgs, input: ItemFn) -> syn::Result<proc_macro2::TokenStream> {
	if input.sig.asyncness.is_some() {
		return Err(syn::Error::new_spanned(
			input.sig.asyncness,
			"#[layout] functions must not be async",
		));
	}
	if !input.sig.generics.params.is_empty() || input.sig.generics.where_clause.is_some() {
		return Err(syn::Error::new_spanned(
			input.sig.generics,
			"#[layout] functions must not be generic",
		));
	}
	match &input.sig.output {
		ReturnType::Type(_, ty) if is_page_type(ty) => {}
		_ => {
			return Err(syn::Error::new_spanned(
				&input.sig,
				"#[layout] functions must return Page",
			));
		}
	}

	let pages_crate = get_reinhardt_pages_crate();
	let fn_name = input.sig.ident.clone();
	let component_name = fn_name.to_string().to_case(Case::Pascal);
	let component_ident = format_ident!("{component_name}", span = fn_name.span());
	let props_ident = format_ident!("{}Props", component_name, span = fn_name.span());
	let original_ident = format_ident!("__{}_layout_body", fn_name, span = fn_name.span());
	let output = input.sig.output.clone();
	let vis = input.vis.clone();
	let field_vis = field_visibility_tokens(&vis);
	let attrs = input.attrs.clone();
	let block = input.block.clone();
	let args_info = parse_layout_args(&input.sig.inputs)?;
	let extracted_args = &args_info.extracted;
	let outlet_name = &args_info.outlet_name;
	let outlet_ty = &args_info.outlet_ty;

	let props_fields = extracted_args.iter().map(|arg| {
		let name = &arg.name;
		let ty = &arg.ty;
		quote! { #field_vis #name: #ty }
	});
	let destructure_fields = extracted_args.iter().map(|arg| &arg.name);
	let original_inputs = extracted_args.iter().map(|arg| {
		let name = &arg.name;
		let ty = &arg.ty;
		quote! { #name: #ty }
	});
	let call_args = extracted_args.iter().map(|arg| &arg.name);
	let from_request_fields = extracted_args.iter().map(|arg| {
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
	let extractor_type_aliases = extracted_args.iter().enumerate().map(|(index, arg)| {
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
		#[derive(#pages_crate::__private::bon::Builder)]
		#[builder(crate = #pages_crate::__private::bon)]
		#vis struct #props_ident {
			#(#props_fields,)*
			#field_vis #outlet_name: #pages_crate::component::Outlet,
		}

		#(#extractor_type_aliases)*

		impl #pages_crate::__private::reinhardt_urls::routers::client_router::FromLayoutRequest
			for #props_ident
		{
			fn from_layout_request(
				ctx: &#pages_crate::router::request::RouteContext,
				outlet: #pages_crate::component::Outlet,
			) -> ::std::result::Result<Self, #pages_crate::router::request::ExtractError> {
				::std::result::Result::Ok(Self {
					#(#from_request_fields,)*
					#outlet_name: outlet,
				})
			}
		}

		impl #pages_crate::__private::reinhardt_urls::routers::client_router::LayoutInfo
			for #props_ident
		{
			fn path() -> &'static str { #path }
			fn name() -> &'static str { #route_name }
			fn component_name() -> &'static str { #component_name_literal }
			fn function_name() -> &'static str { #fn_name_literal }
			fn props_type_name() -> &'static str { #props_type_literal }
		}

		#pages_crate::__private::inventory::submit! {
			#pages_crate::__private::reinhardt_urls::routers::client_router::LayoutMetadata {
				path: #path,
				name: #route_name,
				component_name: #component_name_literal,
				function_name: #fn_name_literal,
				props_type_name: #props_type_literal,
				module_path: ::core::module_path!(),
			}
		}

		fn #original_ident(#(#original_inputs,)* #outlet_name: #outlet_ty) #output {
			#block
		}

		#(#attrs)*
		#vis fn #fn_name(props: #props_ident) #output {
			let #props_ident { #(#destructure_fields,)* #outlet_name } = props;
			#original_ident(#(#call_args,)* #outlet_name)
		}
	})
}

fn field_visibility_tokens(vis: &Visibility) -> proc_macro2::TokenStream {
	match vis {
		Visibility::Inherited => quote! {},
		_ => quote! { #vis },
	}
}

fn is_page_type(ty: &Type) -> bool {
	let Type::Path(type_path) = ty else {
		return false;
	};
	type_path
		.path
		.segments
		.last()
		.is_some_and(|segment| segment.ident == "Page")
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

		let arg = parse_extractor_arg(pat, ty, "#[component]")?;
		let name = arg.name.clone();
		if !seen.insert(name.to_string()) {
			return Err(syn::Error::new_spanned(
				&name,
				format!("duplicate component props field `{name}`"),
			));
		}
		out.push(arg);
	}

	Ok(out)
}

fn parse_layout_args(inputs: &Punctuated<FnArg, Token![,]>) -> syn::Result<LayoutFunctionArgs> {
	let mut seen = HashSet::new();
	let mut extracted = Vec::new();
	let mut outlet = None;

	for input in inputs {
		let FnArg::Typed(PatType { pat, ty, .. }) = input else {
			return Err(syn::Error::new_spanned(
				input,
				"#[layout] does not support receiver arguments",
			));
		};

		if is_outlet_type(ty) {
			let Pat::Ident(pat_ident) = &**pat else {
				return Err(syn::Error::new_spanned(
					pat,
					"#[layout] Outlet parameter must bind a plain identifier",
				));
			};
			let name = pat_ident.ident.clone();
			if outlet.is_some() {
				return Err(syn::Error::new_spanned(
					&name,
					"#[layout] functions must accept exactly one Outlet parameter",
				));
			}
			if !seen.insert(name.to_string()) {
				return Err(syn::Error::new_spanned(
					&name,
					format!("duplicate layout props field `{name}`"),
				));
			}
			outlet = Some((name, (**ty).clone()));
			continue;
		}

		let arg = parse_extractor_arg(pat, ty, "#[layout]")?;
		let name = arg.name.clone();
		if !seen.insert(name.to_string()) {
			return Err(syn::Error::new_spanned(
				&name,
				format!("duplicate layout props field `{name}`"),
			));
		}
		extracted.push(arg);
	}

	let Some((outlet_name, outlet_ty)) = outlet else {
		return Err(syn::Error::new_spanned(
			inputs,
			"#[layout] functions must accept exactly one Outlet parameter",
		));
	};

	Ok(LayoutFunctionArgs {
		extracted,
		outlet_name,
		outlet_ty,
	})
}

fn parse_extractor_arg(pat: &Pat, ty: &Type, macro_name: &str) -> syn::Result<ExtractedArg> {
	let Pat::TupleStruct(PatTupleStruct { path, elems, .. }) = pat else {
		return Err(syn::Error::new_spanned(
			pat,
			format!(
				"{macro_name} arguments must use extractor destructuring such as Path(id): Path<i64>"
			),
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
	Ok(ExtractedArg {
		source,
		name,
		ty: inner_ty,
		extractor_ty: ty.clone(),
	})
}

fn is_outlet_type(ty: &Type) -> bool {
	let Type::Path(type_path) = ty else {
		return false;
	};
	type_path
		.path
		.segments
		.last()
		.is_some_and(|segment| segment.ident == "Outlet")
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
