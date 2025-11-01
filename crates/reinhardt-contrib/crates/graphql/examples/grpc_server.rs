//! Example: GraphQL over gRPC Server
//!
//! This example demonstrates how to run a GraphQL server using gRPC as the transport protocol.
//!
//! Run this server with:
//! ```bash
//! cargo run --package reinhardt-graphql --features graphql-grpc --example grpc_server
//! ```
//!
//! Then, in another terminal, run the client:
//! ```bash
//! cargo run --package reinhardt-graphql --features graphql-grpc --example grpc_client
//! ```

#[cfg(feature = "graphql-grpc")]
use async_graphql::{EmptySubscription, Schema};
#[cfg(feature = "graphql-grpc")]
use reinhardt_graphql::{
	grpc_service::GraphQLGrpcService,
	schema::{Mutation, Query, UserStorage},
};
#[cfg(feature = "graphql-grpc")]
use reinhardt_grpc::proto::graphql::graph_ql_service_server::GraphQlServiceServer;
#[cfg(feature = "graphql-grpc")]
use tonic::transport::Server;

#[cfg(feature = "graphql-grpc")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("Starting GraphQL gRPC Server...");

	// Server address
	let addr = "127.0.0.1:50051".parse()?;
	println!("Server listening on {}", addr);

	// Create GraphQL schema
	let storage = UserStorage::new();
	let schema = Schema::build(Query, Mutation, EmptySubscription)
		.data(storage)
		.finish();

	println!("GraphQL schema created with Query and Mutation support");

	// Create gRPC service
	let service = GraphQLGrpcService::new(schema);
	let grpc_service = GraphQlServiceServer::new(service);

	println!("\nAvailable GraphQL operations:");
	println!("  Queries:");
	println!("    - hello(name: String): String");
	println!("    - user(id: ID!): User");
	println!("    - users: [User!]!");
	println!("  Mutations:");
	println!("    - createUser(input: CreateUserInput!): User!");
	println!("    - updateUserStatus(id: ID!, active: Boolean!): User");

	println!("\nServer ready! Waiting for requests...\n");

	// Start gRPC server
	Server::builder()
		.add_service(grpc_service)
		.serve(addr)
		.await?;

	Ok(())
}

#[cfg(not(feature = "graphql-grpc"))]
fn main() {
	eprintln!("This example requires the 'graphql-grpc' feature.");
	eprintln!(
		"Run with: cargo run --package reinhardt-graphql --features graphql-grpc --example grpc_server"
	);
	std::process::exit(1);
}
