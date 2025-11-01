//! gRPC service adapter
//!
//! This module provides traits for integrating gRPC services into
//! other framework components (e.g., GraphQL resolvers).

use async_trait::async_trait;

/// Trait for integrating gRPC services into GraphQL resolvers
///
/// # Examples
///
/// ```ignore
/// use reinhardt_grpc::adapter::GrpcServiceAdapter;
/// use async_trait::async_trait;
///
/// struct UserServiceAdapter {
///     // gRPC client connection
/// }
///
/// #[async_trait]
/// impl GrpcServiceAdapter for UserServiceAdapter {
///     type Input = String; // User ID
///     type Output = User;  // GraphQL User type
///     type Error = anyhow::Error;
///
///     async fn call(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
///         // Example implementation: Fetch user information using gRPC client
///         // let response = self.grpc_client.get_user(input).await?;
///         // Ok(User::from_proto(response))
///         # unimplemented!("Replace with actual gRPC implementation")
///     }
/// }
/// ```
#[async_trait]
pub trait GrpcServiceAdapter: Send + Sync {
	/// Input type (typically corresponds to gRPC request)
	type Input;

	/// Output type (typically corresponds to GraphQL type)
	type Output;

	/// Error type
	type Error: std::error::Error + Send + Sync + 'static;

	/// Call gRPC service and convert result to GraphQL type
	async fn call(&self, input: Self::Input) -> Result<Self::Output, Self::Error>;
}

/// Trait for integrating gRPC Subscriptions into GraphQL Subscriptions
///
/// # Examples
///
/// ```ignore
/// use reinhardt_grpc::adapter::GrpcSubscriptionAdapter;
///
/// struct UserEventsAdapter;
///
/// impl GrpcSubscriptionAdapter for UserEventsAdapter {
///     type Proto = proto::UserEvent;
///     type GraphQL = User;
///     type Error = anyhow::Error;
///
///     fn map_event(&self, proto: Self::Proto) -> Option<Self::GraphQL> {
///         // Example implementation: Convert Protobuf event to GraphQL type
///         // Some(User {
///         //     id: proto.user_id,
///         //     name: proto.name,
///         // })
///         # None
///     }
///
///     fn handle_error(&self, error: Self::Error) -> String {
///         error.to_string()
///     }
/// }
/// ```
pub trait GrpcSubscriptionAdapter: Send + Sync {
	/// Protobuf event type
	type Proto;

	/// GraphQL event type
	type GraphQL;

	/// Error type
	type Error: std::error::Error + Send + Sync + 'static;

	/// Map Protobuf event to GraphQL type
	///
	/// Returns None to filter out events
	fn map_event(&self, proto: Self::Proto) -> Option<Self::GraphQL>;

	/// Handle error
	fn handle_error(&self, error: Self::Error) -> String {
		error.to_string()
	}
}
