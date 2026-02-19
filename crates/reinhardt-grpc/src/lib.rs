//! gRPC support for Reinhardt framework
//!
//! This crate provides gRPC infrastructure features for the Reinhardt framework.
//!
//! # Features
//!
//! - Common Protobuf types (Empty, Timestamp, Error, PageInfo, BatchResult)
//! - GraphQL over gRPC types (GraphQLRequest, GraphQLResponse, SubscriptionEvent)
//! - gRPC error handling
//! - gRPC service adapter trait
//! - Server configuration with message size limits
//! - Dependency injection support (with `di` feature)
//!
//! # Usage
//!
//! Users can define their own .proto files in their projects,
//! and utilize the common types and adapter traits from this crate.
//!
//! ## Dependency Injection
//!
//! Enable the `di` feature to use dependency injection in gRPC handlers:
//!
//! ```toml
//! [dependencies]
//! reinhardt-grpc = { version = "0.1", features = ["di"] }
//! ```
//!
//! Then use the `#[grpc_handler]` macro:
//!
//! ```rust,ignore
//! # use reinhardt_grpc::{GrpcRequestExt, grpc_handler};
//! # use tonic::{Request, Response, Status};
//! # struct GetUserRequest;
//! # struct User;
//! # struct DatabaseConnection;
//! # struct Handler;
//! # impl Handler {
//! #[grpc_handler]
//! async fn get_user_impl(
//!     &self,
//!     request: Request<GetUserRequest>,
//!     #[inject] db: DatabaseConnection,
//! ) -> Result<Response<User>, Status> {
//!     // db is automatically resolved
//! #   Ok(Response::new(User))
//! }
//! # }
//! ```

pub mod adapter;
pub mod error;
pub mod server;

#[cfg(feature = "di")]
pub mod di;

// Generated Protobuf code (common types provided by the framework)
pub mod proto {
	pub mod common {
		tonic::include_proto!("reinhardt.common");
	}

	pub mod graphql {
		tonic::include_proto!("reinhardt.graphql");
	}
}

pub use adapter::{GrpcServiceAdapter, GrpcSubscriptionAdapter};
pub use error::{GrpcError, GrpcResult};
pub use server::{GrpcServerConfig, GrpcServerConfigBuilder, MessageSizeLimiter};

#[cfg(feature = "di")]
pub use di::GrpcRequestExt;

#[cfg(feature = "di")]
pub use reinhardt_grpc_macros::grpc_handler;
