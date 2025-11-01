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
//!
//! # Usage
//!
//! Users can define their own .proto files in their projects,
//! and utilize the common types and adapter traits from this crate.

pub mod adapter;
pub mod error;

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
