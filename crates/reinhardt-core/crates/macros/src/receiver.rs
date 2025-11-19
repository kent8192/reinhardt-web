use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemFn, Result};

/// Parse receiver macro arguments
#[derive(Default)]
struct ReceiverArgs {
	signal_name: String,
	sender_type: Option<syn::Type>,
	dispatch_uid: Option<String>,
	priority: i32,
}

fn parse_receiver_args(args: TokenStream) -> Result<ReceiverArgs> {
	let mut result = ReceiverArgs::default();

	if args.is_empty() {
		return Ok(result);
	}

	// Parse arguments using ParseNestedMeta
	let parser = syn::meta::parser(|meta| {
		if meta.path.is_ident("signal") {
			result.signal_name = meta.value()?.parse::<syn::LitStr>()?.value();
			Ok(())
		} else if meta.path.is_ident("sender") {
			result.sender_type = Some(meta.value()?.parse()?);
			Ok(())
		} else if meta.path.is_ident("dispatch_uid") {
			result.dispatch_uid = Some(meta.value()?.parse::<syn::LitStr>()?.value());
			Ok(())
		} else if meta.path.is_ident("priority") {
			result.priority = meta.value()?.parse::<syn::LitInt>()?.base10_parse()?;
			Ok(())
		} else {
			Err(meta.error("unsupported receiver attribute"))
		}
	});

	syn::parse::Parser::parse2(parser, args)?;
	Ok(result)
}

/// Implementation of the `receiver` procedural macro
///
/// This macro automatically registers receiver functions with the signal system
/// using the `inventory` crate for static collection.
///
/// # Arguments
///
/// - `signal`: Name of the signal to connect to (required)
/// - `sender`: Optional sender type filter
/// - `dispatch_uid`: Optional unique identifier for this receiver
/// - `priority`: Optional priority (higher values execute first, default: 0)
///
/// # Examples
///
/// ```ignore
/// use reinhardt_macros::receiver;
/// use reinhardt_signals::SignalError;
/// use std::sync::Arc;
///
/// #[receiver(signal = "post_save")]
/// async fn on_user_saved(instance: Arc<User>) -> Result<(), SignalError> {
///     println!("User saved: {:?}", instance);
///     Ok(())
/// }
/// ```
pub fn receiver_impl(args: TokenStream, input: ItemFn) -> Result<TokenStream> {
	let args = parse_receiver_args(args)?;

	let fn_name = &input.sig.ident;
	let fn_vis = &input.vis;
	let fn_block = &input.block;
	let fn_attrs = &input.attrs;
	let fn_sig = &input.sig;

	// Generate registration code
	let signal_name = &args.signal_name;
	let receiver_name = fn_name.to_string();
	let dispatch_uid = args.dispatch_uid.as_deref();
	let priority = args.priority;

	// Validate that the function has at least one parameter
	if fn_sig.inputs.is_empty() {
		return Err(syn::Error::new_spanned(
			fn_sig,
			"Receiver function must have at least one parameter",
		));
	}

	// Build the receiver factory function with type erasure
	let factory_fn = quote! {
		|| -> ::std::sync::Arc<
			dyn Fn(::std::sync::Arc<dyn ::std::any::Any + Send + Sync>)
				-> ::std::pin::Pin<Box<dyn ::std::future::Future<Output = Result<(), reinhardt_signals::SignalError>> + Send>>
				+ Send + Sync
		> {
			::std::sync::Arc::new(|data| {
				Box::pin(async move {
					// For type-erased data, we use Arc<dyn Any> directly
					// The receiver function will need to accept Arc<dyn Any + Send + Sync>
					// or we need runtime type checking
					#fn_name(data).await
				})
			})
		}
	};

	// Build dispatch_uid setter
	let dispatch_uid_setter = if let Some(uid) = dispatch_uid {
		quote! { .with_dispatch_uid(#uid) }
	} else {
		quote! {}
	};

	// Build priority setter
	let priority_setter = if priority != 0 {
		quote! { .with_priority(#priority) }
	} else {
		quote! {}
	};

	// Build sender type setter
	let sender_type_setter = if let Some(sender_type) = &args.sender_type {
		// Generate a function that returns TypeId for the sender type
		quote! { .with_sender_type(|| ::std::any::TypeId::of::<#sender_type>()) }
	} else {
		quote! {}
	};

	let expanded = quote! {
		// Keep the original function
		#(#fn_attrs)*
		#fn_vis #fn_sig {
			#fn_block
		}

		// Generate static registration
		::inventory::submit! {
			reinhardt_signals::ReceiverRegistryEntry::new(
				#signal_name,
				#receiver_name,
				#factory_fn,
			)
			#dispatch_uid_setter
			#priority_setter
			#sender_type_setter
		}
	};

	Ok(expanded)
}
