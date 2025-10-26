//! GraphQL over gRPC service implementation

#[cfg(feature = "graphql-grpc")]
use async_graphql::Schema;
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

/// GraphQL service implementation for gRPC
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
    /// Create a new GraphQL gRPC service
    pub fn new(schema: Schema<Query, Mutation, Subscription>) -> Self {
        Self {
            schema: Arc::new(schema),
        }
    }

    /// Convert GraphQL request to async-graphql request
    fn convert_request(&self, req: GraphQlRequest) -> async_graphql::Request {
        let mut gql_req = async_graphql::Request::new(&req.query);

        // Add variables if present
        if let Some(variables) = req.variables {
            if !variables.is_empty() {
                if let Ok(vars) = serde_json::from_str::<serde_json::Value>(&variables) {
                    gql_req = gql_req.variables(async_graphql::Variables::from_json(vars));
                }
            }
        }

        // Add operation name if present
        if let Some(operation_name) = req.operation_name {
            if !operation_name.is_empty() {
                gql_req = gql_req.operation_name(operation_name);
            }
        }

        gql_req
    }

    /// Convert async-graphql response to gRPC response
    fn convert_response(&self, resp: async_graphql::Response) -> GraphQlResponse {
        let mut grpc_resp = GraphQlResponse::default();

        // Convert data to JSON string
        if matches!(resp.data, async_graphql::Value::Null) {
            grpc_resp.data = None;
        } else {
            grpc_resp.data = Some(resp.data.to_string());
        }

        // Convert errors
        if !resp.errors.is_empty() {
            grpc_resp.errors = resp
                .errors
                .into_iter()
                .map(|err| {
                    let mut grpc_err = reinhardt_grpc::proto::graphql::GraphQlError::default();
                    grpc_err.message = err.message;

                    // Convert locations
                    grpc_err.locations = err
                        .locations
                        .into_iter()
                        .map(|loc| reinhardt_grpc::proto::graphql::GraphQlLocation {
                            line: loc.line as i32,
                            column: loc.column as i32,
                        })
                        .collect();

                    // Convert path
                    grpc_err.path = err
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
                    if !err.extensions.is_none() {
                        if let Ok(ext_str) = serde_json::to_string(&err.extensions) {
                            grpc_err.extensions = Some(ext_str);
                        }
                    }

                    grpc_err
                })
                .collect();
        }

        // Convert extensions
        if !resp.extensions.is_empty() {
            if let Ok(ext_str) = serde_json::to_string(&resp.extensions) {
                grpc_resp.extensions = Some(ext_str);
            }
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
        let gql_req = self.convert_request(req);

        // Execute query
        let gql_resp = self.schema.execute(gql_req).await;

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
        let gql_req = self.convert_request(req);

        // Execute mutation
        let gql_resp = self.schema.execute(gql_req).await;

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
        let gql_req = self.convert_request(req);

        // Clone schema for 'static lifetime requirement
        let schema = Arc::clone(&self.schema);

        // Convert to gRPC stream
        let output_stream = async_stream::stream! {
            // Execute subscription inside the stream to avoid lifetime issues
            let mut stream = schema.execute_stream(gql_req);

            let mut event_id = 0u64;
            while let Some(resp) = stream.next().await {
                event_id += 1;

                let grpc_resp = GraphQlResponse {
                    data: if matches!(resp.data, async_graphql::Value::Null) {
                        None
                    } else {
                        Some(resp.data.to_string())
                    },
                    errors: resp.errors.into_iter().map(|err| {
                        let mut grpc_err = reinhardt_grpc::proto::graphql::GraphQlError::default();
                        grpc_err.message = err.message;

                        // Convert locations
                        grpc_err.locations = err
                            .locations
                            .into_iter()
                            .map(|loc| reinhardt_grpc::proto::graphql::GraphQlLocation {
                                line: loc.line as i32,
                                column: loc.column as i32,
                            })
                            .collect();

                        // Convert path
                        grpc_err.path = err
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
                        if !err.extensions.is_none() {
                            if let Ok(ext_str) = serde_json::to_string(&err.extensions) {
                                grpc_err.extensions = Some(ext_str);
                            }
                        }

                        grpc_err
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
