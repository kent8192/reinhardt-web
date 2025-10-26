//! GraphQL support for Reinhardt framework

pub mod context;
pub mod schema;
pub mod subscription;
pub mod types;

#[cfg(feature = "graphql-grpc")]
pub mod grpc_service;

pub use context::{DataLoader, GraphQLContext, LoaderError};
pub use schema::{create_schema, AppSchema, CreateUserInput, Mutation, Query, User, UserStorage};
pub use subscription::{EventBroadcaster, SubscriptionRoot, UserEvent};

#[cfg(feature = "graphql-grpc")]
pub use grpc_service::GraphQLGrpcService;

// gRPC integration: re-export of adapter traits and derive macros
#[cfg(any(feature = "graphql-grpc", feature = "subscription"))]
pub use reinhardt_grpc::{GrpcServiceAdapter, GrpcSubscriptionAdapter};

#[cfg(any(feature = "graphql-grpc", feature = "subscription"))]
pub use reinhardt_graphql_macros::{GrpcGraphQLConvert, GrpcSubscription};
