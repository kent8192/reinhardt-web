//! Shared implementation for injectable provider function macros.

use crate::crate_paths::get_reinhardt_di_crate;
use crate::utils::{extract_scope_from_args, is_inject_attr};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, GenericArgument, ItemFn, Pat, PatType, PathArguments, Result, Type};

fn depends_key_value_types(ty: &Type) -> Option<(&Type, &Type)> {
	let Type::Path(type_path) = ty else {
		return None;
	};
	let segment = type_path.path.segments.last()?;
	if segment.ident != "Depends" {
		return None;
	}
	let PathArguments::AngleBracketed(args) = &segment.arguments else {
		return None;
	};
	if args.args.len() != 2 {
		return None;
	}
	let mut generic_args = args.args.iter();
	let GenericArgument::Type(key_ty) = generic_args.next()? else {
		return None;
	};
	let GenericArgument::Type(value_ty) = generic_args.next()? else {
		return None;
	};
	Some((key_ty, value_ty))
}

fn is_factory_output_type(ty: &Type) -> bool {
	let Type::Path(type_path) = ty else {
		return false;
	};
	let Some(segment) = type_path.path.segments.last() else {
		return false;
	};
	if segment.ident != "FactoryOutput" {
		return false;
	}
	let PathArguments::AngleBracketed(args) = &segment.arguments else {
		return false;
	};
	if args.args.len() != 2 {
		return false;
	}
	args.args
		.iter()
		.all(|arg| matches!(arg, GenericArgument::Type(_)))
}

fn generate_inject_resolver_expr(
	di_crate: &TokenStream,
	ty: &Type,
	ctx: TokenStream,
	use_cache: bool,
) -> TokenStream {
	if let Some((key_ty, value_ty)) = depends_key_value_types(ty) {
		quote! {
			{
				#di_crate::Depends::<#key_ty, #value_ty>::resolve_from_registry(#ctx, #use_cache)
					.await
			}
		}
	} else {
		quote! {
			{
				use #di_crate::{
					__InjectFallbackResolver as _,
					__InjectWrapperResolver as _,
				};
				#di_crate::__InjectResolver::<#ty>::new()
					.__resolve_inject_parameter(#ctx, #use_cache)
					.await
			}
		}
	}
}

/// Implementation of injectable provider function attribute macros.
///
/// This macro:
/// 1. Extracts the return type of the factory function
/// 2. Analyzes parameters marked with `#[inject]`
/// 3. Generates a wrapper that resolves dependencies
/// 4. Registers the factory with the global registry using inventory
pub(crate) fn injectable_factory_impl(args: TokenStream, input: ItemFn) -> Result<TokenStream> {
	let fn_name = &input.sig.ident;
	let fn_vis = &input.vis;
	let fn_block = &input.block;
	let fn_attrs = &input.attrs;

	// Extract scope from macro arguments
	let scope = extract_scope_from_args(args)?;
	let scope_tokens = scope.into_tokens();

	// Validate that the function is async
	if input.sig.asyncness.is_none() {
		return Err(syn::Error::new_spanned(
			&input.sig,
			"#[injectable] provider functions must be async",
		));
	}

	// Extract return type
	let return_type = match &input.sig.output {
		syn::ReturnType::Type(_, ty) => ty.as_ref().clone(),
		syn::ReturnType::Default => {
			return Err(syn::Error::new_spanned(
				&input.sig,
				"#[injectable] provider functions must have an explicit return type",
			));
		}
	};
	if !is_factory_output_type(&return_type) {
		return Err(syn::Error::new_spanned(
			&return_type,
			"#[injectable] provider functions must return FactoryOutput<K, T>",
		));
	}

	// Analyze function parameters
	let mut inject_params = Vec::new();
	let mut regular_params = Vec::new();

	for arg in &input.sig.inputs {
		if let FnArg::Typed(PatType { attrs, pat, ty, .. }) = arg {
			let has_inject = attrs.iter().any(is_inject_attr);

			if has_inject {
				inject_params.push((pat.clone(), ty.clone()));
			} else {
				regular_params.push((pat.clone(), ty.clone()));
			}
		}
	}

	// Reject non-inject parameters with a clear compile error.
	// The generated wrapper function only receives an InjectionContext, so non-inject
	// parameters would be undefined in the generated code.
	if !regular_params.is_empty() {
		return Err(syn::Error::new_spanned(
			&input.sig,
			"#[injectable] provider functions must have all parameters marked with #[inject]. \
			 Non-inject parameters are not supported because the generated wrapper function \
			 only receives an InjectionContext.",
		));
	}

	// Get dynamic crate path (needed by inject_resolutions below)
	let di_crate = get_reinhardt_di_crate();

	// Generate dependency resolution code (kent8192/reinhardt-web#4938).
	// The runtime resolver lets the compiler choose the `InjectableType`
	// wrapper path first and falls back to normal `Injectable` resolution for
	// non-wrapper parameters.
	let inject_resolutions: Vec<_> = inject_params
		.iter()
		.map(|(pat, ty)| {
			let resolve_expr = generate_inject_resolver_expr(&di_crate, ty, quote! { &*ctx }, true);
			quote! {
				let #pat: #ty = #resolve_expr?;
			}
		})
		.collect();

	// Generate parameter names for the original function call
	let inject_param_names: Vec<_> = inject_params
		.iter()
		.map(|(pat, _)| {
			// Extract the identifier from the pattern
			if let Pat::Ident(pat_ident) = pat.as_ref() {
				let ident = &pat_ident.ident;
				quote! { #ident }
			} else {
				quote! { #pat }
			}
		})
		.collect();

	let regular_param_names: Vec<_> = regular_params
		.iter()
		.map(|(pat, _)| {
			if let Pat::Ident(pat_ident) = pat.as_ref() {
				let ident = &pat_ident.ident;
				quote! { #ident }
			} else {
				quote! { #pat }
			}
		})
		.collect();

	// Generate the original function with a hygienic internal name.
	// Uses double-underscore `__reinhardt_` prefix to avoid collisions with user-defined names.
	let original_fn_name =
		syn::Ident::new(&format!("__reinhardt_{}_impl", fn_name), fn_name.span());
	let original_params: Vec<_> = inject_params
		.iter()
		.chain(regular_params.iter())
		.map(|(pat, ty)| quote! { #pat: #ty })
		.collect();

	// Generate type name for registration
	let type_name = quote! { #return_type }.to_string();

	// Generate registration function name for const-safe inventory submission
	let register_fn_name = format_ident!("__reinhardt_register_{}", fn_name);

	// Generate the expanded code
	let expanded = quote! {
		// Original implementation function (private)
		async fn #original_fn_name(#(#original_params),*) -> #return_type {
			#fn_block
		}

		// Public wrapper factory function
		#(#fn_attrs)*
		#fn_vis async fn #fn_name(
			ctx: ::std::sync::Arc<#di_crate::InjectionContext>,
		) -> #di_crate::DiResult<#return_type> {
			#di_crate::with_cycle_detection_scope(async {
				// Set task-local resolve context before resolving dependencies.
				// Inherit root from outer scope (for nested factory calls),
				// or default to ctx itself at the top level.
				// This must be established first so that Depends::resolve() and
				// any Injectable::inject() impl can use get_di_context().
				let __resolve_ctx = #di_crate::resolve_context::ResolveContext {
					root: #di_crate::resolve_context::RESOLVE_CTX
						.try_with(|__outer| ::std::sync::Arc::clone(&__outer.root))
						.unwrap_or_else(|_| ::std::sync::Arc::clone(&ctx)),
					current: ::std::sync::Arc::clone(&ctx),
				};

				let result = #di_crate::resolve_context::RESOLVE_CTX
					.scope(__resolve_ctx, async {
						// Resolve #[inject] dependencies
						#(#inject_resolutions)*

						Ok(#original_fn_name(#(#inject_param_names,)* #(#regular_param_names),*).await)
					})
					.await?;
				Ok(result)
			}).await
		}

		// Registration function for const-safe inventory::submit
		fn #register_fn_name(registry: &#di_crate::DependencyRegistry) {
			registry.register_async::<#return_type, _, _>(#scope_tokens, #fn_name);
			registry.register_type_name(
				::std::any::TypeId::of::<#return_type>(),
				#type_name,
			);
			registry.register_qualified_type_name(
				::std::any::TypeId::of::<#return_type>(),
				::std::any::type_name::<#return_type>(),
			);
		}

		#di_crate::inventory::submit! {
			#di_crate::DependencyRegistration::new::<#return_type>(
				#type_name,
				#scope_tokens,
				#register_fn_name
			)
		}

	};

	Ok(expanded)
}
