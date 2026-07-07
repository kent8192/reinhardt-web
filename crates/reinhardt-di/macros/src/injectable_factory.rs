//! Shared implementation for injectable provider function macros.

use crate::crate_paths::get_reinhardt_di_crate;
use crate::utils::{extract_scope_from_args, is_inject_attr};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, GenericArgument, ItemFn, Pat, PatType, PathArguments, Result, Type};

enum ProviderReturn {
	Direct {
		registered_ty: TokenStream,
		validation_ty: Type,
		wrap_expr: TokenStream,
	},
	Keyed {
		registered_ty: Type,
		validation_ty: Type,
		wrap_expr: TokenStream,
	},
}

impl ProviderReturn {
	fn registered_type_tokens(&self) -> TokenStream {
		match self {
			Self::Direct { registered_ty, .. } => quote! { #registered_ty },
			Self::Keyed { registered_ty, .. } => quote! { #registered_ty },
		}
	}

	fn validation_type_tokens(&self) -> TokenStream {
		match self {
			Self::Direct { validation_ty, .. } | Self::Keyed { validation_ty, .. } => {
				quote! { #validation_ty }
			}
		}
	}

	fn wrap_expr_tokens(&self) -> TokenStream {
		match self {
			Self::Direct { wrap_expr, .. } | Self::Keyed { wrap_expr, .. } => wrap_expr.clone(),
		}
	}
}

fn direct_provider_return(di_crate: &TokenStream, value_ty: Type) -> ProviderReturn {
	let registered_ty =
		quote! { #di_crate::KeyedFactoryOutput<#di_crate::SelfKey<#value_ty>, #value_ty> };
	ProviderReturn::Direct {
		registered_ty,
		validation_ty: value_ty,
		wrap_expr: quote! { #di_crate::KeyedFactoryOutput::new(__provider_value) },
	}
}

fn keyed_provider_return(ty: &Type, args: &syn::AngleBracketedGenericArguments) -> ProviderReturn {
	let Some(GenericArgument::Type(value_ty)) = args.args.iter().nth(1) else {
		return ProviderReturn::Keyed {
			registered_ty: ty.clone(),
			validation_ty: ty.clone(),
			wrap_expr: quote! { __provider_value },
		};
	};
	ProviderReturn::Keyed {
		registered_ty: ty.clone(),
		validation_ty: value_ty.clone(),
		wrap_expr: quote! { __provider_value },
	}
}

fn provider_return_shape(di_crate: &TokenStream, ty: &Type) -> ProviderReturn {
	let Type::Path(type_path) = ty else {
		return direct_provider_return(di_crate, ty.clone());
	};
	let Some(segment) = type_path.path.segments.last() else {
		return direct_provider_return(di_crate, ty.clone());
	};
	if segment.ident == "KeyedFactoryOutput" || segment.ident == "FactoryOutput" {
		let PathArguments::AngleBracketed(args) = &segment.arguments else {
			return direct_provider_return(di_crate, ty.clone());
		};
		if args.args.len() == 2
			&& args
				.args
				.iter()
				.all(|arg| matches!(arg, GenericArgument::Type(_)))
		{
			return keyed_provider_return(ty, args);
		}
	}
	direct_provider_return(di_crate, ty.clone())
}

fn generate_inject_resolver_expr(
	di_crate: &TokenStream,
	ty: &Type,
	ctx: TokenStream,
	use_cache: bool,
) -> TokenStream {
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
	let di_crate = get_reinhardt_di_crate();
	let provider_return = provider_return_shape(&di_crate, &return_type);
	let registered_type = provider_return.registered_type_tokens();
	let validation_type = provider_return.validation_type_tokens();
	let wrapper_output_expr = provider_return.wrap_expr_tokens();

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
	let type_name = quote! { #registered_type }.to_string();

	// Generate registration function name for const-safe inventory submission
	let register_fn_name = format_ident!("__reinhardt_register_{}", fn_name);

	// Generate the expanded code
	let expanded = quote! {
		// Original implementation function (private)
		#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
		async fn #original_fn_name(#(#original_params),*) -> #return_type {
			#fn_block
		}

		// Public wrapper factory function
		#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
		#(#fn_attrs)*
		#fn_vis async fn #fn_name(
			ctx: ::std::sync::Arc<#di_crate::InjectionContext>,
		) -> #di_crate::DiResult<#registered_type> {
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

						let __provider_value = #original_fn_name(
							#(#inject_param_names,)*
							#(#regular_param_names),*
						).await;
						Ok(#wrapper_output_expr)
					})
					.await?;
				Ok(result)
			}).await
		}

		#[cfg(all(target_family = "wasm", target_os = "unknown"))]
		#(#fn_attrs)*
		#fn_vis async fn #fn_name() {}

		// Registration function for const-safe inventory::submit
		#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
		fn #register_fn_name(registry: &#di_crate::DependencyRegistry) {
			registry.register_async::<#registered_type, _, _>(#scope_tokens, #fn_name);
			registry.register_type_name(
				::std::any::TypeId::of::<#registered_type>(),
				#type_name,
			);
			registry.register_qualified_type_name(
				::std::any::TypeId::of::<#registered_type>(),
				::std::any::type_name::<#validation_type>(),
			);
		}

		#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
		#di_crate::inventory::submit! {
			#di_crate::DependencyRegistration::new::<#registered_type>(
				#type_name,
				#scope_tokens,
				#register_fn_name
			)
		}

	};

	Ok(expanded)
}
