//! GraphQL over gRPC service implementation

mod operation_guard;

#[cfg(test)]
mod tests;

use operation_guard::{GuardedSchema, OperationCheck, OperationGuard};

#[cfg(feature = "graphql-grpc")]
use async_graphql::parser::types::OperationType;
#[cfg(feature = "graphql-grpc")]
use async_graphql::{Schema, SchemaBuilder};
#[cfg(feature = "graphql-grpc")]
use reinhardt_grpc::proto::graphql::{
	GraphQlRequest, GraphQlResponse, SubscriptionEvent, graph_ql_service_server::GraphQlService,
};
#[cfg(feature = "graphql-grpc")]
use std::{pin::Pin, sync::Arc};
#[cfg(feature = "graphql-grpc")]
use tokio_stream::{Stream, StreamExt};
#[cfg(feature = "graphql-grpc")]
use tonic::{Request, Response, Status};

/// GraphQL service implementation for gRPC.
///
/// Build the schema with [`Self::schema_builder`] so operation checks run after
/// request preparation and document transformations, before any resolver runs.
/// Invalid documents, operation selections, and class mismatches return gRPC
/// `INVALID_ARGUMENT`. Subscriptions report these errors through their stream.
#[cfg(feature = "graphql-grpc")]
pub struct GraphQLGrpcService<Query, Mutation, Subscription> {
	schema: Arc<Schema<Query, Mutation, Subscription>>,
}

#[cfg(feature = "graphql-grpc")]
impl<Query, Mutation, Subscription> GraphQLGrpcService<Query, Mutation, Subscription>
where
	Query: async_graphql::ObjectType + 'static,
	Mutation: async_graphql::ObjectType + 'static,
	Subscription: async_graphql::SubscriptionType + 'static,
{
	/// Create a new GraphQL gRPC service.
	///
	/// # Panics
	///
	/// Panics if the schema was not created with [`Self::schema_builder`] (or the
	/// built-in `create_schema` helpers with `graphql-grpc` enabled). A completed
	/// schema cannot be retrofitted with the operation guard.
	pub fn new(schema: Schema<Query, Mutation, Subscription>) -> Self {
		assert!(
			schema.data::<GuardedSchema>().is_some(),
			"GraphQLGrpcService requires a schema built with GraphQLGrpcService::schema_builder"
		);
		Self {
			schema: Arc::new(schema),
		}
	}

	/// Build a schema with transport validation surrounding all user extensions.
	///
	/// Replace `Schema::build` with this method and keep subsequent builder calls.
	/// The finished schema can be shared with HTTP handlers; ordinary requests are
	/// unaffected by the gRPC operation constraint.
	///
	/// ```
	/// use async_graphql::{EmptyMutation, EmptySubscription, Object};
	/// use reinhardt_graphql::GraphQLGrpcService;
	/// struct Query;
	/// #[Object]
	/// impl Query {
	///     async fn value(&self) -> i32 { 7 }
	/// }
	/// let schema = GraphQLGrpcService::schema_builder(Query, EmptyMutation, EmptySubscription)
	///     .limit_depth(10)
	///     .finish();
	/// let service = GraphQLGrpcService::new(schema);
	/// ```
	pub fn schema_builder(
		query: Query,
		mutation: Mutation,
		subscription: Subscription,
	) -> SchemaBuilder<Query, Mutation, Subscription> {
		Schema::build(query, mutation, subscription)
			.extension(OperationGuard)
			.data(GuardedSchema)
	}

	/// Convert the request without parsing ahead of schema preparation hooks.
	fn convert_request(
		&self,
		req: GraphQlRequest,
		expected: OperationType,
	) -> (async_graphql::Request, Arc<OperationCheck>) {
		let check = OperationCheck::new(expected);
		let mut gql_req = async_graphql::Request::new(req.query).data(Arc::clone(&check));

		// Add variables if present
		if let Some(variables) = req.variables
			&& !variables.is_empty()
			&& let Ok(vars) = serde_json::from_str::<serde_json::Value>(&variables)
		{
			gql_req = gql_req.variables(async_graphql::Variables::from_json(vars));
		}

		// Add operation name if present
		if let Some(operation_name) = req.operation_name
			&& !operation_name.is_empty()
		{
			gql_req = gql_req.operation_name(operation_name);
		}

		(gql_req, check)
	}

	/// Convert async-graphql response to gRPC response
	fn convert_response(&self, resp: async_graphql::Response) -> GraphQlResponse {
		// Convert data to JSON string
		let data = if matches!(resp.data, async_graphql::Value::Null) {
			None
		} else {
			Some(resp.data.to_string())
		};

		let mut grpc_resp = GraphQlResponse {
			data,
			..Default::default()
		};

		// Convert errors
		if !resp.errors.is_empty() {
			grpc_resp.errors = resp
				.errors
				.into_iter()
				.map(|err| {
					// Convert locations
					let locations = err
						.locations
						.into_iter()
						.map(|loc| reinhardt_grpc::proto::graphql::GraphQlLocation {
							line: loc.line as i32,
							column: loc.column as i32,
						})
						.collect();

					// Convert path
					let path = err
						.path
						.into_iter()
						.map(|segment| {
							let seg = match segment {
								async_graphql::PathSegment::Field(f) => {
									reinhardt_grpc::proto::graphql::path_segment::Segment::Field(
										f.to_string(),
									)
								}
								async_graphql::PathSegment::Index(i) => {
									reinhardt_grpc::proto::graphql::path_segment::Segment::Index(
										i as i32,
									)
								}
							};
							reinhardt_grpc::proto::graphql::PathSegment { segment: Some(seg) }
						})
						.collect();

					// Convert extensions
					let extensions = if err.extensions.is_some()
						&& let Ok(ext_str) = serde_json::to_string(&err.extensions)
					{
						Some(ext_str)
					} else {
						None
					};

					reinhardt_grpc::proto::graphql::GraphQlError {
						message: err.message,
						locations,
						path,
						extensions,
					}
				})
				.collect();
		}

		// Convert extensions
		if !resp.extensions.is_empty()
			&& let Ok(ext_str) = serde_json::to_string(&resp.extensions)
		{
			grpc_resp.extensions = Some(ext_str);
		}

		grpc_resp
	}
}

#[cfg(feature = "graphql-grpc")]
#[tonic::async_trait]
impl<Query, Mutation, Subscription> GraphQlService
	for GraphQLGrpcService<Query, Mutation, Subscription>
where
	Query: async_graphql::ObjectType + 'static,
	Mutation: async_graphql::ObjectType + 'static,
	Subscription: async_graphql::SubscriptionType + 'static,
{
	/// Execute a GraphQL Query operation
	async fn execute_query(
		&self,
		request: Request<GraphQlRequest>,
	) -> Result<Response<GraphQlResponse>, Status> {
		let req = request.into_inner();
		let (gql_req, check) = self.convert_request(req, OperationType::Query);

		// Execute query
		let gql_resp = self.schema.execute(gql_req).await;
		if let Some(error) = check.error() {
			return Err(error);
		}

		// Convert response
		let grpc_resp = self.convert_response(gql_resp);

		Ok(Response::new(grpc_resp))
	}

	/// Execute a GraphQL Mutation operation
	async fn execute_mutation(
		&self,
		request: Request<GraphQlRequest>,
	) -> Result<Response<GraphQlResponse>, Status> {
		let req = request.into_inner();
		let (gql_req, check) = self.convert_request(req, OperationType::Mutation);

		// Execute mutation
		let gql_resp = self.schema.execute(gql_req).await;
		if let Some(error) = check.error() {
			return Err(error);
		}

		// Convert response
		let grpc_resp = self.convert_response(gql_resp);

		Ok(Response::new(grpc_resp))
	}

	/// Execute a GraphQL Subscription operation (server streaming)
	type ExecuteSubscriptionStream =
		Pin<Box<dyn Stream<Item = Result<SubscriptionEvent, Status>> + Send>>;

	async fn execute_subscription(
		&self,
		request: Request<GraphQlRequest>,
	) -> Result<Response<Self::ExecuteSubscriptionStream>, Status> {
		let req = request.into_inner();
		let (gql_req, check) = self.convert_request(req, OperationType::Subscription);

		// Clone schema for 'static lifetime requirement
		let schema = Arc::clone(&self.schema);

		// Convert to gRPC stream
		let output_stream = async_stream::stream! {
			// Execute subscription inside the stream to avoid lifetime issues
			let mut stream = schema.execute_stream(gql_req);

			let mut event_id = 0u64;
			while let Some(resp) = stream.next().await {
				if let Some(error) = check.error() {
					yield Err(error);
					return;
				}
				event_id += 1;

				let grpc_resp = GraphQlResponse {
					data: if matches!(resp.data, async_graphql::Value::Null) {
						None
					} else {
						Some(resp.data.to_string())
					},
					errors: resp.errors.into_iter().map(|err| {
						// Convert locations
						let locations = err
							.locations
							.into_iter()
							.map(|loc| reinhardt_grpc::proto::graphql::GraphQlLocation {
								line: loc.line as i32,
								column: loc.column as i32,
							})
							.collect();

						// Convert path
						let path = err
							.path
							.into_iter()
							.map(|segment| {
								let seg = match segment {
									async_graphql::PathSegment::Field(f) => {
										reinhardt_grpc::proto::graphql::path_segment::Segment::Field(
											f.to_string(),
										)
									}
									async_graphql::PathSegment::Index(i) => {
										reinhardt_grpc::proto::graphql::path_segment::Segment::Index(
											i as i32,
										)
									}
								};
								reinhardt_grpc::proto::graphql::PathSegment { segment: Some(seg) }
							})
							.collect();

						// Convert extensions
						let extensions = if err.extensions.is_some()
							&& let Ok(ext_str) = serde_json::to_string(&err.extensions) {
								Some(ext_str)
							} else {
								None
							};

						reinhardt_grpc::proto::graphql::GraphQlError {
							message: err.message,
							locations,
							path,
							extensions,
						}
					}).collect(),
					extensions: if !resp.extensions.is_empty() {
						serde_json::to_string(&resp.extensions).ok()
					} else {
						None
					},
				};

				let event = SubscriptionEvent {
					id: event_id.to_string(),
					event_type: "data".to_string(),
					payload: Some(grpc_resp),
					timestamp: Some(reinhardt_grpc::proto::common::Timestamp {
						seconds: chrono::Utc::now().timestamp(),
						nanos: 0,
					}),
				};

				yield Ok(event);
			}
		};

		Ok(Response::new(Box::pin(output_stream)))
	}
}
