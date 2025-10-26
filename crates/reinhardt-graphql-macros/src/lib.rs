//! Derive macros for reinhardt-graphql
//!
//! This crate provides derive macros to simplify gRPC ↔ GraphQL integration.

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod convert;
mod subscription;

/// Generate automatic conversion between Protobuf and GraphQL types
///
/// # Examples
///
/// ```ignore
/// use reinhardt_graphql_macros::GrpcGraphQLConvert;
///
/// #[derive(GrpcGraphQLConvert)]
/// #[graphql(rename_all = "camelCase")]
/// struct User {
///     id: String,
///     name: String,
///     #[graphql(skip_if = "Option::is_none")]
///     email: Option<String>,
/// }
/// ```
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
/// # Examples
///
/// ```ignore
/// use reinhardt_graphql_macros::GrpcSubscription;
///
/// #[derive(GrpcSubscription)]
/// #[grpc(service = "UserEventsServiceClient", method = "subscribe_user_events")]
/// #[graphql(filter = "event_type == Created")]
/// struct UserCreatedSubscription;
/// ```
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
