//! Code generation for low-level `server_fnset` declarations.

use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::ext::IdentExt;
use syn::{FnArg, ImplItem, ImplItemFn, Pat, Path, Type, parse_quote};

use crate::crate_paths::get_reinhardt_pages_crate_info;
use crate::server_fn::{
	generate_internal_server_fn, generate_internal_server_fn_with_tokens, is_extractor_type,
};

use super::validate::{ValidatedFnSet, ValidatedImplSet, ValidatedSet};

pub(crate) fn expand(input: ValidatedSet) -> TokenStream {
	match input {
		ValidatedSet::Function(function) => expand_function(*function),
		ValidatedSet::Implementation(implementation) => expand_impl(*implementation),
	}
}

fn expand_function(mut input: ValidatedFnSet) -> TokenStream {
	if let Some(resource) = input.resource.clone() {
		return expand_model(input, resource);
	}
	let crate_info = get_reinhardt_pages_crate_info();
	let crate_use = crate_info.use_statement;
	let pages = crate_info.ident;
	let name = input
		.options
		.name
		.expect("validated function name should exist");
	let original_body = input.function.block;

	input.function.block = Box::new(parse_quote!({
		#crate_use
		#pages::server_fn::ServerFnSetChainExt::named(#original_body, #name)
	}));

	let function = input.function;
	quote!(#function)
}

fn expand_model(mut input: ValidatedFnSet, resource: syn::Type) -> TokenStream {
	if let Some(actions) = input.options.actions.take() {
		return expand_linked_model(input, resource, actions);
	}
	let crate_info = get_reinhardt_pages_crate_info();
	let crate_use = crate_info.use_statement;
	let pages = crate_info.ident;
	let name_option = input
		.options
		.name
		.take()
		.expect("validated function name should exist");
	let name = name_option.value();
	let function_name = input.function.sig.ident.clone();
	let default_actions_ident = quote::format_ident!(
		"__ReinhardtDefaultServerFnSetActions{}",
		pascal(&function_name.to_string())
	);
	let vis = input.function.vis.clone();
	let action_specs = standard_actions(&resource);
	let generated = action_specs.iter().map(|action| {
		generate_internal_server_fn(
			action.function.clone(),
			format!("/api/server_fn/{}/{}", name, action.segment),
			format!("{}-{}", name, action.segment),
			action.detail,
			action.transactional,
		)
	});
	let markers = action_specs.iter().map(|action| {
		let ident = &action.function.sig.ident;
		quote!(.server_fn(#function_name::#ident::marker))
	});
	let marker_idents: Vec<_> = action_specs
		.iter()
		.map(|action| action.function.sig.ident.clone())
		.collect();
	let registration_type = marker_idents.iter().fold(
		quote!(#pages::server_fn::ServerFnSetNil),
		|tail, ident| quote!(#pages::server_fn::ServerFnSetCons<#function_name::#ident::marker, #tail>),
	);
	let name_literal = syn::LitStr::new(&name, name_option.span());
	input.function.sig.output = parse_quote!(-> impl #pages::server_fn::ServerFnSetRegistration);
	input.function.block = Box::new(parse_quote!({
		#crate_use
		use #pages::server_fn::ServerFnSetChainExt as _;
		#pages::server_fn::ServerFnSetChainExt::named(
			<#default_actions_ident as #pages::server_fn::ServerFnSetActions<#resource>>::registration(),
			#name_literal,
		)
	}));
	let function = input.function;
	quote! {
		#function
		#vis mod #function_name {
			use super::*;
			use #pages::server_fn::*;
			#(#generated)*
		}
		struct #default_actions_ident;
		impl #pages::server_fn::ServerFnSetActions<#resource> for #default_actions_ident {
			type Registration = #registration_type;
			fn registration() -> Self::Registration {
				use #pages::server_fn::ServerFnSetChainExt as _;
				#pages::server_fn::ServerFnSet::new() #(#markers)*
			}
		}
	}
}

fn expand_linked_model(
	mut input: ValidatedFnSet,
	resource: syn::Type,
	actions: syn::Path,
) -> TokenStream {
	let crate_info = get_reinhardt_pages_crate_info();
	let crate_use = crate_info.use_statement;
	let pages = crate_info.ident;
	let name = input
		.options
		.name
		.expect("validated function name should exist");
	let link_ident = quote::format_ident!(
		"__ReinhardtServerFnSetLink{}",
		pascal(&input.function.sig.ident.to_string())
	);
	let link_macro_ident =
		quote::format_ident!("__reinhardt_server_fnset_link_{}", input.function.sig.ident);
	let link_vis = input.function.vis.clone();
	let macro_vis: syn::Visibility = parse_quote!(pub(crate));
	let callback_matcher: TokenStream = "($callback:ident)"
		.parse()
		.expect("static callback matcher should parse");
	let callback_invocation: TokenStream = "$callback"
		.parse()
		.expect("static callback invocation should parse");
	input.function.sig.output = parse_quote!(-> impl #pages::server_fn::ServerFnSetRegistration);
	input.function.block = Box::new(parse_quote!({
		#crate_use
		#pages::server_fn::ServerFnSetChainExt::named(
			<#actions as #pages::server_fn::ServerFnSetActions<#resource>>::registration(),
			#name,
		)
	}));
	let function = input.function;
	quote! {
		#function
		#[doc(hidden)]
		#link_vis struct #link_ident;
		impl #pages::server_fn::ModelServerFnSetLink for #link_ident {
			type Resource = #resource;
			const NAME: &'static str = #name;
		}
		macro_rules! #link_macro_ident {
			#callback_matcher => { #callback_invocation!(#name, (#link_vis), #resource); };
		}
		#macro_vis use #link_macro_ident;
	}
}

fn expand_impl(mut input: ValidatedImplSet) -> TokenStream {
	let crate_info = get_reinhardt_pages_crate_info();
	let pages = crate_info.ident;
	let actions_type = input.implementation.self_ty.clone();
	let syn::Type::Path(actions_path) = actions_type.as_ref() else {
		return syn::Error::new_spanned(actions_type, "server_fnset impl type must be a path")
			.into_compile_error();
	};
	let actions_ident = actions_path
		.path
		.segments
		.last()
		.expect("path should have a segment")
		.ident
		.clone();
	let link_fn_ident = input
		.link
		.segments
		.last()
		.expect("link should have a segment")
		.ident
		.clone();
	let action_module_ident = generated_action_module_ident(&input.link, &link_fn_ident);
	let link_macro_ident = quote::format_ident!("__reinhardt_server_fnset_link_{}", link_fn_ident);
	let generator_macro_ident =
		quote::format_ident!("__reinhardt_generate_server_fnset_{}", action_module_ident,);
	let set_name_matcher: TokenStream = "($set_name:literal, ($($set_vis:tt)*), $set_resource:ty)"
		.parse()
		.expect("static set name matcher should parse");
	let set_name_metavariable: TokenStream = "$set_name"
		.parse()
		.expect("static set name metavariable should parse");
	let set_vis_metavariable: TokenStream = "$($set_vis)*"
		.parse()
		.expect("static set visibility metavariable should parse");
	let action_module_vis = if input.link.segments.len() == 1 {
		quote!(#set_vis_metavariable)
	} else {
		quote!()
	};
	let set_resource_metavariable: TokenStream = "$set_resource"
		.parse()
		.expect("static set resource metavariable should parse");
	let link_macro_path: Path = if input.link.segments.len() == 1 {
		parse_quote!(self::#link_macro_ident)
	} else {
		let mut path = input.link.clone();
		path.segments
			.last_mut()
			.expect("validated link path should have a segment")
			.ident = link_macro_ident.clone();
		path
	};
	let resource_alias_ident = quote::format_ident!(
		"__ReinhardtServerFnSetResource{}{}",
		pascal(&action_module_ident.to_string()),
		pascal(&actions_ident.to_string()),
	);
	let resource: Type = parse_quote!(#resource_alias_ident);

	let mut methods = Vec::new();
	let mut normalized = HashSet::new();
	for item in &mut input.implementation.items {
		let ImplItem::Fn(method) = item else { continue };
		match parse_method(method) {
			Ok(method_info) => {
				if !normalized.insert(method_info.segment.clone()) {
					return syn::Error::new_spanned(
						&method.sig.ident,
						"duplicate normalized server function set action",
					)
					.into_compile_error();
				}
				methods.push(method_info);
				for argument in &mut method.sig.inputs {
					if let FnArg::Typed(parameter) = argument {
						parameter
							.attrs
							.retain(|attribute| !is_inject_attribute(attribute));
					}
				}
			}
			Err(error) => return error.into_compile_error(),
		}
	}

	let standard_names = [
		"list",
		"retrieve",
		"create",
		"update",
		"partial_update",
		"destroy",
	];
	let mut generated = Vec::new();
	let mut marker_idents = Vec::new();
	for spec in standard_actions(&resource) {
		let ident = spec.function.sig.ident.clone();
		let method = methods.iter().find(|method| method.ident == ident);
		let function = if let Some(method) = method {
			if method.custom {
				return syn::Error::new_spanned(
					&method.ident,
					"standard action overrides must not use `#[action]`",
				)
				.into_compile_error();
			}
			let effective = MethodInfo {
				detail: spec.detail,
				transactional: spec.transactional,
				..method.clone()
			};
			match override_wrapper(
				&resource,
				actions_type.as_ref(),
				&effective,
				Some(&spec.function),
			) {
				Ok(function) => function,
				Err(error) => return error.into_compile_error(),
			}
		} else {
			spec.function
		};
		let segment = spec.segment;
		generated.push(generate_internal_server_fn_with_tokens(
			function,
			quote!(concat!("/api/server_fn/", #set_name_metavariable, "/", #segment)),
			quote!(concat!(#set_name_metavariable, "-", #segment)),
			spec.detail,
			spec.transactional,
		));
		marker_idents.push(ident);
	}
	for method in methods
		.iter()
		.filter(|method| !standard_names.iter().any(|name| method.ident == *name))
	{
		if !method.custom {
			return syn::Error::new_spanned(
				&method.ident,
				"custom server function set methods require `#[action(detail = ...)]`",
			)
			.into_compile_error();
		}
		let function = match override_wrapper(&resource, actions_type.as_ref(), method, None) {
			Ok(function) => function,
			Err(error) => return error.into_compile_error(),
		};
		let segment = method.segment.as_str();
		generated.push(generate_internal_server_fn_with_tokens(
			function,
			quote!(concat!("/api/server_fn/", #set_name_metavariable, "/", #segment)),
			quote!(concat!(#set_name_metavariable, "-", #segment)),
			method.detail,
			method.transactional,
		));
		marker_idents.push(method.ident.clone());
	}
	let registration = marker_idents.iter().fold(
		quote!(#pages::server_fn::ServerFnSet::new()),
		|tail, ident| {
			quote!(#pages::server_fn::ServerFnSetChainExt::server_fn(
				#tail,
				#action_module_ident::#ident::marker,
			))
		},
	);
	let registration_type = marker_idents.iter().fold(
		quote!(#pages::server_fn::ServerFnSetNil),
		|tail, ident| quote!(#pages::server_fn::ServerFnSetCons<#action_module_ident::#ident::marker, #tail>),
	);
	let implementation = input.implementation;
	quote! {
		#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
		#implementation
		macro_rules! #generator_macro_ident {
			#set_name_matcher => {
					type #resource_alias_ident = #set_resource_metavariable;
					// Generated action signatures and bodies conditionally use caller-local imports.
					#[doc(hidden)]
					#[allow(unused_imports)]
					#action_module_vis mod #action_module_ident {
					use super::*;
					use #pages::server_fn::*;
					#(#generated)*
				}
				impl #pages::server_fn::ServerFnSetActions<#resource> for #actions_type {
					type Registration = #registration_type;
					fn registration() -> Self::Registration {
						#registration
					}
				}
			}
		}
		#link_macro_path!(#generator_macro_ident);
	}
}

fn generated_action_module_ident(link: &Path, link_fn_ident: &syn::Ident) -> syn::Ident {
	if link.segments.len() == 1 {
		return link_fn_ident.clone();
	}
	let path = link
		.segments
		.iter()
		.map(|segment| {
			let name = segment.ident.unraw().to_string();
			format!("{}_{}", name.len(), name)
		})
		.collect::<Vec<_>>()
		.join("_");
	quote::format_ident!("__reinhardt_server_fnset_actions_{path}")
}

#[derive(Clone)]
struct MethodInfo {
	ident: syn::Ident,
	method: ImplItemFn,
	custom: bool,
	detail: bool,
	transactional: bool,
	segment: String,
}

fn is_inject_attribute(attribute: &syn::Attribute) -> bool {
	attribute.path().is_ident("inject")
		|| attribute
			.path()
			.segments
			.last()
			.is_some_and(|segment| segment.ident == "inject")
}

fn parse_method(method: &mut ImplItemFn) -> syn::Result<MethodInfo> {
	if method.sig.receiver().is_some() || method.sig.asyncness.is_none() {
		return Err(syn::Error::new_spanned(
			&method.sig,
			"server function set actions must be associated async functions",
		));
	}
	let mut custom = false;
	let mut detail = false;
	let mut detail_seen = false;
	let mut transactional = false;
	let mut retained = Vec::new();
	for attribute in std::mem::take(&mut method.attrs) {
		if !attribute.path().is_ident("action") {
			retained.push(attribute);
			continue;
		}
		custom = true;
		attribute.parse_nested_meta(|meta| {
			if meta.path.is_ident("detail") {
				detail = meta.value()?.parse::<syn::LitBool>()?.value;
				detail_seen = true;
				Ok(())
			} else if meta.path.is_ident("transactional") {
				transactional = meta.value()?.parse::<syn::LitBool>()?.value;
				Ok(())
			} else if meta.path.is_ident("methods")
				|| meta.path.is_ident("url_path")
				|| meta.path.is_ident("url_name")
			{
				Err(meta
					.error("REST-only action options are not supported by server function sets"))
			} else {
				Err(meta.error("unknown server function set action option"))
			}
		})?;
	}
	method.attrs = retained;
	for argument in &mut method.sig.inputs {
		if let FnArg::Typed(parameter) = argument
			&& parameter.attrs.iter().any(is_inject_attribute)
		{
			ensure_context_lifetime(&mut parameter.ty);
		}
	}
	if custom && !detail_seen {
		return Err(syn::Error::new_spanned(
			&method.sig.ident,
			"custom actions require an explicit `detail = true` or `detail = false`",
		));
	}
	let ident = method.sig.ident.clone();
	Ok(MethodInfo {
		segment: ident.unraw().to_string().replace('_', "-"),
		ident,
		method: method.clone(),
		custom,
		detail,
		transactional,
	})
}

fn ensure_context_lifetime(ty: &mut Type) {
	let Type::Path(path) = ty else { return };
	let Some(segment) = path.path.segments.last_mut() else {
		return;
	};
	if !segment.ident.to_string().ends_with("ActionContext") {
		return;
	}
	let syn::PathArguments::AngleBracketed(arguments) = &mut segment.arguments else {
		return;
	};
	if arguments
		.args
		.iter()
		.any(|argument| matches!(argument, syn::GenericArgument::Lifetime(_)))
	{
		return;
	}
	arguments
		.args
		.insert(0, syn::GenericArgument::Lifetime(parse_quote!('_)));
}

fn override_wrapper(
	resource: &Type,
	actions: &syn::Type,
	info: &MethodInfo,
	canonical: Option<&syn::ItemFn>,
) -> syn::Result<syn::ItemFn> {
	let mut generated_parameter_names = info
		.method
		.sig
		.inputs
		.iter()
		.filter_map(|argument| match argument {
			FnArg::Typed(parameter) => match parameter.pat.as_ref() {
				Pat::Ident(pattern) => Some(pattern.ident.to_string()),
				_ => None,
			},
			FnArg::Receiver(_) => None,
		})
		.collect::<HashSet<_>>();
	let context = unique_generated_parameter_ident(
		"__reinhardt_server_fnset_context",
		&mut generated_parameter_names,
	);
	let mut declared_client_inputs = Vec::new();
	let mut injected_inputs = Vec::new();
	let mut extractor_inputs = Vec::new();
	let mut call_args = Vec::new();
	let mut context_count = 0;
	let mut canonical_inputs = canonical.map(client_parameters);
	if let Some(inputs) = &mut canonical_inputs {
		let mut reserved_client_names = info
			.method
			.sig
			.inputs
			.iter()
			.filter_map(|argument| match argument {
				FnArg::Typed(parameter)
					if parameter.attrs.iter().any(is_inject_attribute)
						|| is_extractor_type(&parameter.ty)
						|| is_model_policy_principal_type(&parameter.ty) =>
				{
					match parameter.pat.as_ref() {
						Pat::Ident(pattern) => Some(pattern.ident.to_string()),
						_ => None,
					}
				}
				FnArg::Typed(_) | FnArg::Receiver(_) => None,
			})
			.collect::<HashSet<_>>();
		for parameter in inputs {
			let Pat::Ident(pattern) = parameter.pat.as_mut() else {
				unreachable!()
			};
			let ident = unique_generated_parameter_ident(
				&pattern.ident.to_string(),
				&mut reserved_client_names,
			);
			pattern.ident = ident.clone();
			generated_parameter_names.insert(ident.to_string());
		}
	}
	let mut client_index = 0;
	for argument in &info.method.sig.inputs {
		let FnArg::Typed(parameter) = argument else {
			continue;
		};
		let injected = parameter.attrs.iter().any(is_inject_attribute);
		if injected {
			let context_name = type_last_ident(&parameter.ty).map(ToString::to_string);
			if info.transactional && context_name.as_deref() == Some("DatabaseConnection") {
				return Err(syn::Error::new_spanned(
					&parameter.ty,
					"transactional actions must use a transaction-bound action context, not `DatabaseConnection`",
				));
			}
			if context_name
				.as_deref()
				.is_some_and(|name| name.ends_with("ActionContext"))
			{
				let expected_context = if canonical.is_some() && info.ident == "create" {
					"CreateActionContext"
				} else {
					match (info.detail, info.transactional) {
						(false, false) => "CollectionReadActionContext",
						(false, true) => "CollectionActionContext",
						(true, false) => "DetailReadActionContext",
						(true, true) => "DetailActionContext",
					}
				};
				if context_name.as_deref() != Some(expected_context) {
					return Err(syn::Error::new_spanned(
						&parameter.ty,
						format!("this action requires `{expected_context}`"),
					));
				}
				context_count += 1;
				call_args.push(quote!(#context));
				continue;
			}
		}
		if injected
			|| is_extractor_type(&parameter.ty)
			|| is_model_policy_principal_type(&parameter.ty)
		{
			let Pat::Ident(pattern) = parameter.pat.as_ref() else {
				return Err(syn::Error::new_spanned(
					&parameter.pat,
					"action parameters must use identifier patterns",
				));
			};
			if injected {
				injected_inputs.push(parameter.clone());
			} else {
				extractor_inputs.push(parameter.clone());
			}
			call_args.push(quote!(#pattern));
		} else {
			let Pat::Ident(pattern) = parameter.pat.as_ref() else {
				return Err(syn::Error::new_spanned(
					&parameter.pat,
					"action parameters must use identifier patterns",
				));
			};
			declared_client_inputs.push(parameter.clone());
			if let Some(inputs) = &canonical_inputs {
				let Some(canonical_parameter) = inputs.get(client_index) else {
					return Err(syn::Error::new_spanned(
						&info.method.sig,
						"standard action override has too many client parameters",
					));
				};
				let Pat::Ident(canonical_pattern) = canonical_parameter.pat.as_ref() else {
					unreachable!()
				};
				call_args.push(quote!(#canonical_pattern));
			} else {
				call_args.push(quote!(#pattern));
			}
			client_index += 1;
		}
	}
	if let Some(inputs) = &canonical_inputs
		&& client_index != inputs.len()
	{
		return Err(syn::Error::new_spanned(
			&info.method.sig,
			"standard action override has the wrong number of client parameters",
		));
	}
	let client_inputs = canonical_inputs.unwrap_or(declared_client_inputs);
	if context_count != 1 {
		return Err(syn::Error::new_spanned(
			&info.method.sig,
			"action methods require exactly one `#[inject]` action context",
		));
	}
	if info.detail && client_inputs.is_empty() {
		return Err(syn::Error::new_spanned(
			&info.method.sig,
			"detail actions require the resource lookup as the first client argument",
		));
	}
	let ident = &info.ident;
	let output = canonical.map_or(&info.method.sig.output, |function| &function.sig.output);
	let cfg_attributes = info
		.method
		.attrs
		.iter()
		.filter(|attribute| attribute.path().is_ident("cfg"))
		.collect::<Vec<_>>();
	let action = match ident.to_string().as_str() {
		"list" => quote!(ServerFnSetAction::List),
		"retrieve" => quote!(ServerFnSetAction::Retrieve),
		"create" => quote!(ServerFnSetAction::Create),
		"update" => quote!(ServerFnSetAction::Update),
		"partial_update" => quote!(ServerFnSetAction::PartialUpdate),
		"destroy" => quote!(ServerFnSetAction::Destroy),
		_ => {
			let name = ident.unraw().to_string();
			quote!(ServerFnSetAction::Custom(#name))
		}
	};
	let connection = unique_generated_parameter_ident(
		"__reinhardt_server_fnset_connection",
		&mut generated_parameter_names,
	);
	let principal = unique_generated_parameter_ident(
		"__reinhardt_server_fnset_principal",
		&mut generated_parameter_names,
	);
	let call = quote!(#actions::#ident(#(#call_args),*));
	let body = if canonical.is_some() && info.ident == "create" {
		quote!(ModelServerFnSet::<#resource>::transactional_create_action(&#principal.0, &#connection, |#context| Box::pin(#call)).await)
	} else if info.transactional {
		if info.detail {
			let Pat::Ident(lookup) = client_inputs[0].pat.as_ref() else {
				unreachable!()
			};
			quote!(ModelServerFnSet::<#resource>::transactional_detail_action(&#principal.0, &#connection, #lookup.clone(), #action, |#context| Box::pin(#call)).await)
		} else {
			quote!(ModelServerFnSet::<#resource>::transactional_collection_action(&#principal.0, &#connection, #action, |#context| Box::pin(#call)).await)
		}
	} else if info.detail {
		let Pat::Ident(lookup) = client_inputs[0].pat.as_ref() else {
			unreachable!()
		};
		quote!(ModelServerFnSet::<#resource>::read_detail_action(&#principal.0, &#connection, #lookup.clone(), #action, |#context| Box::pin(#call)).await)
	} else {
		quote!(ModelServerFnSet::<#resource>::read_collection_action(&#principal.0, &#connection, #action, |#context| Box::pin(#call)).await)
	};
	Ok(parse_quote! {
		#(#cfg_attributes)*
		pub async fn #ident(
			#(#client_inputs,)*
			#(#injected_inputs,)*
			#[inject] #connection: reinhardt_db::orm::DatabaseConnection,
			#(#extractor_inputs,)*
			#principal: PolicyPrincipal<#resource>,
		) #output { #body }
	})
}

fn unique_generated_parameter_ident(base: &str, used: &mut HashSet<String>) -> syn::Ident {
	let mut suffix = 0;
	loop {
		let name = if suffix == 0 {
			base.to_string()
		} else {
			format!("{base}_{suffix}")
		};
		if used.insert(name.clone()) {
			return syn::Ident::new(&name, proc_macro2::Span::call_site());
		}
		suffix += 1;
	}
}

fn client_parameters(function: &syn::ItemFn) -> Vec<syn::PatType> {
	function
		.sig
		.inputs
		.iter()
		.filter_map(|argument| {
			let FnArg::Typed(parameter) = argument else {
				return None;
			};
			if parameter.attrs.iter().any(is_inject_attribute)
				|| is_extractor_type(&parameter.ty)
				|| is_model_policy_principal_type(&parameter.ty)
			{
				None
			} else {
				Some(parameter.clone())
			}
		})
		.collect()
}

fn is_model_policy_principal_type(ty: &Type) -> bool {
	let Type::Path(type_path) = ty else {
		return false;
	};
	if type_path.path.leading_colon.is_some() || type_path.path.segments.len() != 1 {
		return false;
	}
	let Some(segment) = type_path.path.segments.last() else {
		return false;
	};
	if segment.ident != "PolicyPrincipal" {
		return false;
	}
	let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
		return false;
	};
	let Some(syn::GenericArgument::Type(Type::Path(resource))) = arguments.args.first() else {
		return false;
	};
	resource.path.segments.last().is_some_and(|segment| {
		let name = segment.ident.to_string();
		name.ends_with("Resource") || name.starts_with("__ReinhardtServerFnSetResource")
	})
}

fn type_last_ident(ty: &Type) -> Option<&syn::Ident> {
	let Type::Path(path) = ty else { return None };
	path.path.segments.last().map(|segment| &segment.ident)
}

struct ActionSpec {
	function: syn::ItemFn,
	segment: &'static str,
	detail: bool,
	transactional: bool,
}

fn standard_actions(resource: &syn::Type) -> Vec<ActionSpec> {
	vec![
		action(
			parse_quote! {
				pub async fn list(
					query: <#resource as ServerFnResource>::ListQuery,
					#[inject] connection: reinhardt_db::orm::DatabaseConnection,
					principal: PolicyPrincipal<#resource>,
				) -> Result<Page<<#resource as ServerFnResource>::Read>, ServerFnSetError> {
					ModelServerFnSet::<#resource>::list(&principal.0, &connection, query).await
				}
			},
			"list",
			false,
			false,
		),
		action(
			parse_quote! {
				pub async fn retrieve(
					lookup: <#resource as ServerFnResource>::Lookup,
					#[inject] connection: reinhardt_db::orm::DatabaseConnection,
					principal: PolicyPrincipal<#resource>,
				) -> Result<<#resource as ServerFnResource>::Read, ServerFnSetError> {
					ModelServerFnSet::<#resource>::retrieve(&principal.0, &connection, lookup).await
				}
			},
			"retrieve",
			true,
			false,
		),
		action(
			parse_quote! {
				pub async fn create(
					input: <#resource as ServerFnResource>::Create,
					#[inject] connection: reinhardt_db::orm::DatabaseConnection,
					principal: PolicyPrincipal<#resource>,
				) -> Result<<#resource as ServerFnResource>::Read, ServerFnSetError> {
					ModelServerFnSet::<#resource>::create(&principal.0, &connection, input).await
				}
			},
			"create",
			false,
			true,
		),
		action(
			parse_quote! {
				pub async fn update(
					lookup: <#resource as ServerFnResource>::Lookup,
					input: <#resource as ServerFnResource>::Update,
					#[inject] connection: reinhardt_db::orm::DatabaseConnection,
					principal: PolicyPrincipal<#resource>,
				) -> Result<<#resource as ServerFnResource>::Read, ServerFnSetError> {
					ModelServerFnSet::<#resource>::update(&principal.0, &connection, lookup, input).await
				}
			},
			"update",
			true,
			true,
		),
		action(
			parse_quote! {
				pub async fn partial_update(
					lookup: <#resource as ServerFnResource>::Lookup,
					input: <#resource as ServerFnResource>::Patch,
					#[inject] connection: reinhardt_db::orm::DatabaseConnection,
					principal: PolicyPrincipal<#resource>,
				) -> Result<<#resource as ServerFnResource>::Read, ServerFnSetError> {
					ModelServerFnSet::<#resource>::partial_update(&principal.0, &connection, lookup, input).await
				}
			},
			"partial-update",
			true,
			true,
		),
		action(
			parse_quote! {
				pub async fn destroy(
					lookup: <#resource as ServerFnResource>::Lookup,
					#[inject] connection: reinhardt_db::orm::DatabaseConnection,
					principal: PolicyPrincipal<#resource>,
				) -> Result<(), ServerFnSetError> {
					ModelServerFnSet::<#resource>::destroy(&principal.0, &connection, lookup).await
				}
			},
			"destroy",
			true,
			true,
		),
	]
}

fn action(
	function: syn::ItemFn,
	segment: &'static str,
	detail: bool,
	transactional: bool,
) -> ActionSpec {
	ActionSpec {
		function,
		segment,
		detail,
		transactional,
	}
}

fn pascal(value: &str) -> String {
	value
		.split('_')
		.filter(|part| !part.is_empty())
		.map(|part| {
			let mut chars = part.chars();
			chars
				.next()
				.map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
				.unwrap_or_default()
		})
		.collect()
}
