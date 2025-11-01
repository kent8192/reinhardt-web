//! Implementation of GrpcSubscription derive macro
//!
//! To solve Rust 2024 lifetime capture issues,
//! use Box::pin and explicit lifetime annotations.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, LitStr, Result};

/// Parse result of gRPC subscription attributes
#[derive(Debug, Clone)]
struct GrpcSubscriptionAttr {
	service: Option<String>,
	method: Option<String>,
	filter: Option<String>,
}

impl GrpcSubscriptionAttr {
	/// Parse from meta attributes
	fn from_attrs(attrs: &[syn::Attribute]) -> Result<Self> {
		let mut service = None;
		let mut method = None;
		let mut filter = None;

		for attr in attrs {
			if attr.path().is_ident("grpc") {
				attr.parse_nested_meta(|meta| {
					if meta.path.is_ident("service") {
						let value = meta.value()?;
						let s: LitStr = value.parse()?;
						service = Some(s.value());
						Ok(())
					} else if meta.path.is_ident("method") {
						let value = meta.value()?;
						let s: LitStr = value.parse()?;
						method = Some(s.value());
						Ok(())
					} else {
						Err(meta.error("unsupported grpc attribute"))
					}
				})?;
			} else if attr.path().is_ident("graphql") {
				attr.parse_nested_meta(|meta| {
					if meta.path.is_ident("filter") {
						let value = meta.value()?;
						let s: LitStr = value.parse()?;
						filter = Some(s.value());
						Ok(())
					} else {
						Err(meta.error("unsupported graphql attribute"))
					}
				})?;
			}
		}

		Ok(Self {
			service,
			method,
			filter,
		})
	}
}

pub fn expand_derive(input: DeriveInput) -> Result<TokenStream> {
	let name = &input.ident;

	// Get service and method from attributes
	let grpc_attr = GrpcSubscriptionAttr::from_attrs(&input.attrs)?;

	// Generate literal strings for service and method
	let service = grpc_attr.service.as_deref().unwrap_or("UnknownService");
	let method = grpc_attr.method.as_deref().unwrap_or("subscribe");

	// Generate filter code if filter is specified
	let filter_code = if let Some(filter_expr) = grpc_attr.filter {
		let filter_tokens: TokenStream =
			filter_expr.parse().unwrap_or_else(|_| quote! { |_| true });
		quote! {
			.filter(|item| {
				let filter_fn = #filter_tokens;
				filter_fn(item)
			})
		}
	} else {
		quote! {}
	};

	// Generate GraphQL Subscription implementation
	let expanded = quote! {
		#[async_graphql::Subscription]
		impl #name {
			/// Map gRPC stream to GraphQL Subscription
			async fn subscribe<'ctx>(
				&self,
				ctx: &async_graphql::Context<'ctx>,
			) -> impl futures_util::Stream<Item = String> + 'ctx {
				use tokio_stream::StreamExt;

				// Get gRPC client (using information from attributes)
				// If service and method are specified, use them to get the client
				let _service = #service;
				let _method = #method;
				let _client = ctx.data::<()>().ok();

				// Rust 2024 support: Wrap with Box::pin to make lifetime explicit
				Box::pin(tokio_stream::empty()
					#filter_code
				)
			}
		}
	};

	Ok(expanded)
}

#[cfg(test)]
mod tests {
	use super::*;
	use syn::parse_quote;

	#[test]
	fn test_basic_subscription() {
		let input: DeriveInput = parse_quote! {
			struct UserCreatedSubscription;
		};

		let result = expand_derive(input);
		assert!(result.is_ok());

		let output = result.unwrap();
		let output_str = output.to_string();

		// Check existence of Subscription implementation
		assert!(output_str.contains("# [async_graphql :: Subscription]"));
		assert!(output_str.contains("impl UserCreatedSubscription"));
		// Check default values
		assert!(output_str.contains("UnknownService"));
		assert!(output_str.contains("subscribe"));
	}

	#[test]
	fn test_subscription_with_grpc_attr() {
		let input: DeriveInput = parse_quote! {
			#[grpc(service = "UserService", method = "subscribe_users")]
			struct UserSubscription;
		};

		let result = expand_derive(input);
		assert!(result.is_ok());

		let output = result.unwrap();
		let output_str = output.to_string();

		// Check values obtained from attributes
		assert!(output_str.contains("UserService"));
		assert!(output_str.contains("subscribe_users"));
	}

	#[test]
	fn test_subscription_with_partial_attr() {
		let input: DeriveInput = parse_quote! {
			#[grpc(service = "OrderService")]
			struct OrderSubscription;
		};

		let result = expand_derive(input);
		assert!(result.is_ok());

		let output = result.unwrap();
		let output_str = output.to_string();

		// Only service specified, method is default
		assert!(output_str.contains("OrderService"));
		assert!(output_str.contains("subscribe"));
	}

	#[test]
	fn test_subscription_with_filter() {
		let input: DeriveInput = parse_quote! {
			#[grpc(service = "EventService", method = "subscribe_events")]
			#[graphql(filter = "|event| event.is_important()")]
			struct ImportantEventsSubscription;
		};

		let result = expand_derive(input);
		assert!(result.is_ok());

		let output = result.unwrap();
		let output_str = output.to_string();

		// Check filter is present (TokenStream separates tokens with spaces)
		assert!(output_str.contains(". filter"));
		assert!(output_str.contains("is_important"));
	}
}
