//! Integration tests for GraphQL over gRPC

#[cfg(feature = "graphql-grpc")]
mod grpc_tests {
    use async_graphql::{EmptySubscription, Object, Schema};
    use reinhardt_graphql::grpc_service::GraphQLGrpcService;
    use reinhardt_grpc::proto::graphql::{graph_ql_service_server::GraphQlService, GraphQlRequest};
    use tonic::Request;

    // Simple Query type for testing
    struct Query;

    #[Object]
    impl Query {
        async fn hello(&self, name: Option<String>) -> String {
            format!("Hello, {}!", name.unwrap_or_else(|| "World".to_string()))
        }

        async fn add(&self, a: i32, b: i32) -> i32 {
            a + b
        }
    }

    // Simple Mutation type for testing
    struct Mutation;

    #[Object]
    impl Mutation {
        async fn echo(&self, message: String) -> String {
            format!("Echo: {}", message)
        }
    }

    #[tokio::test]
    async fn test_execute_query_hello() {
        // Create schema
        let schema = Schema::build(Query, Mutation, EmptySubscription).finish();
        let service = GraphQLGrpcService::new(schema);

        // Create query request
        let request = Request::new(GraphQlRequest {
            query: r#"{ hello }"#.to_string(),
            variables: None,
            operation_name: None,
        });

        // Execute query
        let response = service.execute_query(request).await.unwrap();
        let grpc_resp = response.into_inner();

        // Verify response
        assert!(grpc_resp.data.is_some());
        let data = grpc_resp.data.unwrap();
        assert!(data.contains("Hello, World!"));
        assert!(grpc_resp.errors.is_empty());
    }

    #[tokio::test]
    async fn test_execute_query_with_arguments() {
        // Create schema
        let schema = Schema::build(Query, Mutation, EmptySubscription).finish();
        let service = GraphQLGrpcService::new(schema);

        // Create query request with arguments
        let request = Request::new(GraphQlRequest {
            query: r#"{ hello(name: "GraphQL") }"#.to_string(),
            variables: None,
            operation_name: None,
        });

        // Execute query
        let response = service.execute_query(request).await.unwrap();
        let grpc_resp = response.into_inner();

        // Verify response
        assert!(grpc_resp.data.is_some());
        let data = grpc_resp.data.unwrap();
        assert!(data.contains("Hello, GraphQL!"));
        assert!(grpc_resp.errors.is_empty());
    }

    #[tokio::test]
    async fn test_execute_query_with_variables() {
        // Create schema
        let schema = Schema::build(Query, Mutation, EmptySubscription).finish();
        let service = GraphQLGrpcService::new(schema);

        // Create query request with variables
        let request = Request::new(GraphQlRequest {
            query: r#"query AddNumbers($a: Int!, $b: Int!) { add(a: $a, b: $b) }"#.to_string(),
            variables: Some(r#"{"a": 5, "b": 3}"#.to_string()),
            operation_name: Some("AddNumbers".to_string()),
        });

        // Execute query
        let response = service.execute_query(request).await.unwrap();
        let grpc_resp = response.into_inner();

        // Verify response
        assert!(grpc_resp.data.is_some());
        let data = grpc_resp.data.unwrap();
        assert!(data.contains("8"));
        assert!(grpc_resp.errors.is_empty());
    }

    #[tokio::test]
    async fn test_execute_mutation() {
        // Create schema
        let schema = Schema::build(Query, Mutation, EmptySubscription).finish();
        let service = GraphQLGrpcService::new(schema);

        // Create mutation request
        let request = Request::new(GraphQlRequest {
            query: r#"mutation { echo(message: "Hello gRPC") }"#.to_string(),
            variables: None,
            operation_name: None,
        });

        // Execute mutation
        let response = service.execute_mutation(request).await.unwrap();
        let grpc_resp = response.into_inner();

        // Verify response
        assert!(grpc_resp.data.is_some());
        let data = grpc_resp.data.unwrap();
        assert!(data.contains("Echo: Hello gRPC"));
        assert!(grpc_resp.errors.is_empty());
    }

    #[tokio::test]
    async fn test_query_with_invalid_syntax() {
        // Create schema
        let schema = Schema::build(Query, Mutation, EmptySubscription).finish();
        let service = GraphQLGrpcService::new(schema);

        // Create query request with invalid syntax
        let request = Request::new(GraphQlRequest {
            query: r#"{ invalid syntax }"#.to_string(),
            variables: None,
            operation_name: None,
        });

        // Execute query
        let response = service.execute_query(request).await.unwrap();
        let grpc_resp = response.into_inner();

        // Verify error response
        assert!(!grpc_resp.errors.is_empty(), "Expected errors but got none");
        // Just verify that there's an error, don't check specific message
        // (error messages may vary between async-graphql versions)
    }

    #[tokio::test]
    async fn test_query_with_unknown_field() {
        // Create schema
        let schema = Schema::build(Query, Mutation, EmptySubscription).finish();
        let service = GraphQLGrpcService::new(schema);

        // Create query request with unknown field
        let request = Request::new(GraphQlRequest {
            query: r#"{ unknownField }"#.to_string(),
            variables: None,
            operation_name: None,
        });

        // Execute query
        let response = service.execute_query(request).await.unwrap();
        let grpc_resp = response.into_inner();

        // Verify error response
        assert!(!grpc_resp.errors.is_empty(), "Expected errors but got none");
        // Just verify that there's an error, don't check specific message
        // (error messages may vary between async-graphql versions)
    }

    #[tokio::test]
    async fn test_empty_query() {
        // Create schema
        let schema = Schema::build(Query, Mutation, EmptySubscription).finish();
        let service = GraphQLGrpcService::new(schema);

        // Create empty query request
        let request = Request::new(GraphQlRequest {
            query: "".to_string(),
            variables: None,
            operation_name: None,
        });

        // Execute query
        let response = service.execute_query(request).await.unwrap();
        let grpc_resp = response.into_inner();

        // Verify error response
        assert!(!grpc_resp.errors.is_empty());
    }
}
