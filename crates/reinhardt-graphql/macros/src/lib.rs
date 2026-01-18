//! Derive macros for reinhardt-graphql
//!
//! This crate provides derive macros to simplify gRPC ↔ GraphQL integration
//! and dependency injection support.

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod convert;
mod crate_paths;
mod graphql_handler;
mod subscription;

/// Generate automatic conversion between Protobuf and GraphQL types
///
/// This automatically generates:
/// - `From<proto::User> for User`
/// - `From<User> for proto::User`
#[proc_macro_derive(GrpcGraphQLConvert, attributes(graphql, proto))]
pub fn derive_grpc_graphql_convert(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	convert::expand_derive(input)
		.unwrap_or_else(|err| err.to_compile_error())
		.into()
}

/// Automatically map gRPC Subscription to GraphQL Subscription
///
/// This automatically generates GraphQL Subscription implementation.
/// Handles Rust 2024 lifetime issues.
#[proc_macro_derive(GrpcSubscription, attributes(grpc, graphql))]
pub fn derive_grpc_subscription(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	subscription::expand_derive(input)
		.unwrap_or_else(|err| err.to_compile_error())
		.into()
}

/// Attribute macro for GraphQL resolvers with dependency injection support
///
/// This macro enables the use of `#[inject]` parameters in GraphQL resolver functions,
/// allowing automatic dependency resolution from the `InjectionContext`.
///
/// # Parameters
///
/// Regular parameters are passed through as-is. Parameters marked with `#[inject]`
/// are automatically resolved from the DI context.
///
/// # Requirements
///
/// 1. The function must have an `async_graphql::Context<'_>` parameter
/// 2. The schema must have an `InjectionContext` in its data (use `.data(injection_ctx)`)
/// 3. All injected types must implement `Injectable`
/// 4. The function must be `async`
///
/// # Error Handling
///
/// If dependency injection fails, the function returns `async_graphql::Error`
/// with an error message describing the failure.
#[proc_macro_attribute]
pub fn graphql_handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
	let input = parse_macro_input!(item as syn::ItemFn);

	graphql_handler::expand_graphql_handler(input)
		.unwrap_or_else(|err| err.to_compile_error())
		.into()
}
