//! End-to-End tests for GraphQL over gRPC with real network communication

#[cfg(feature = "graphql-grpc")]
mod e2e_tests {
    use async_graphql::{EmptySubscription, Schema};
    use reinhardt_graphql::{
        grpc_service::GraphQLGrpcService,
        schema::{Mutation, Query, UserStorage},
    };
    use reinhardt_grpc::proto::graphql::{
        GraphQlRequest, graph_ql_service_client::GraphQlServiceClient,
        graph_ql_service_server::GraphQlServiceServer,
    };
    use std::time::Duration;
    use tokio::time::sleep;
    use tonic::transport::{Channel, Server};

    /// Start a gRPC server on a random port and return the address
    async fn start_test_server() -> String {
        // Find available port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind to port");
        let addr = listener.local_addr().expect("Failed to get local address");
        let port = addr.port();
        let server_addr = format!("127.0.0.1:{}", port);

        // Create GraphQL schema
        let storage = UserStorage::new();
        let schema = Schema::build(Query, Mutation, EmptySubscription)
            .data(storage)
            .finish();

        // Create gRPC service
        let service = GraphQLGrpcService::new(schema);
        let grpc_service = GraphQlServiceServer::new(service);

        // Start server in background
        tokio::spawn(async move {
            Server::builder()
                .add_service(grpc_service)
                .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
                .await
                .expect("Server failed");
        });

        // Wait for server to start
        sleep(Duration::from_millis(100)).await;

        format!("http://{}", server_addr)
    }

    /// Create a gRPC client connected to the test server
    async fn create_test_client(server_url: &str) -> GraphQlServiceClient<Channel> {
        GraphQlServiceClient::connect(server_url.to_string())
            .await
            .expect("Failed to connect to server")
    }

    #[tokio::test]
    async fn test_e2e_simple_query() {
        // Start server
        let server_url = start_test_server().await;

        // Create client
        let mut client = create_test_client(&server_url).await;

        // Execute query
        let request = tonic::Request::new(GraphQlRequest {
            query: r#"{ hello }"#.to_string(),
            variables: None,
            operation_name: None,
        });

        let response = client
            .execute_query(request)
            .await
            .expect("Failed to execute query");

        let grpc_resp = response.into_inner();

        // Verify response
        assert!(grpc_resp.data.is_some());
        let data = grpc_resp.data.unwrap();
        assert!(data.contains("Hello, World!"));
        assert!(grpc_resp.errors.is_empty());
    }

    #[tokio::test]
    async fn test_e2e_query_with_arguments() {
        // Start server
        let server_url = start_test_server().await;

        // Create client
        let mut client = create_test_client(&server_url).await;

        // Execute query with arguments
        let request = tonic::Request::new(GraphQlRequest {
            query: r#"{ hello(name: "gRPC E2E") }"#.to_string(),
            variables: None,
            operation_name: None,
        });

        let response = client
            .execute_query(request)
            .await
            .expect("Failed to execute query");

        let grpc_resp = response.into_inner();

        // Verify response
        assert!(grpc_resp.data.is_some());
        let data = grpc_resp.data.unwrap();
        assert!(data.contains("Hello, gRPC E2E!"));
    }

    #[tokio::test]
    async fn test_e2e_mutation() {
        // Start server
        let server_url = start_test_server().await;

        // Create client
        let mut client = create_test_client(&server_url).await;

        // Execute mutation
        let request = tonic::Request::new(GraphQlRequest {
            query: r#"
                mutation {
                    createUser(input: {
                        name: "E2E Test User",
                        email: "e2e@test.com"
                    }) {
                        id
                        name
                        email
                        active
                    }
                }
            "#
            .to_string(),
            variables: None,
            operation_name: None,
        });

        let response = client
            .execute_mutation(request)
            .await
            .expect("Failed to execute mutation");

        let grpc_resp = response.into_inner();

        // Verify response
        assert!(grpc_resp.data.is_some());
        let data = grpc_resp.data.unwrap();
        assert!(data.contains("E2E Test User"));
        assert!(data.contains("e2e@test.com"));
        assert!(data.contains("true")); // active: true
        assert!(grpc_resp.errors.is_empty());
    }

    #[tokio::test]
    async fn test_e2e_query_created_user() {
        // Start server
        let server_url = start_test_server().await;

        // Create client
        let mut client = create_test_client(&server_url).await;

        // First, create a user
        let create_request = tonic::Request::new(GraphQlRequest {
            query: r#"
                mutation {
                    createUser(input: {
                        name: "Query Test",
                        email: "query@test.com"
                    }) {
                        id
                    }
                }
            "#
            .to_string(),
            variables: None,
            operation_name: None,
        });

        let create_response = client
            .execute_mutation(create_request)
            .await
            .expect("Failed to create user");

        let create_resp = create_response.into_inner();
        assert!(create_resp.data.is_some());

        // Extract user ID from response
        let data = create_resp.data.unwrap();
        let user_id = data
            .split('"')
            .find(|s| s.contains('-'))
            .expect("Failed to extract user ID");

        // Query for the created user
        let query_request = tonic::Request::new(GraphQlRequest {
            query: format!(r#"{{ user(id: "{}") {{ name email }} }}"#, user_id),
            variables: None,
            operation_name: None,
        });

        let query_response = client
            .execute_query(query_request)
            .await
            .expect("Failed to query user");

        let query_resp = query_response.into_inner();

        // Verify user data
        assert!(query_resp.data.is_some());
        let query_data = query_resp.data.unwrap();
        assert!(query_data.contains("Query Test"));
        assert!(query_data.contains("query@test.com"));
    }

    #[tokio::test]
    async fn test_e2e_multiple_clients() {
        // Start server
        let server_url = start_test_server().await;

        // Create multiple clients
        let mut client1 = create_test_client(&server_url).await;
        let mut client2 = create_test_client(&server_url).await;
        let mut client3 = create_test_client(&server_url).await;

        // Execute queries concurrently
        let query = GraphQlRequest {
            query: r#"{ hello(name: "Concurrent") }"#.to_string(),
            variables: None,
            operation_name: None,
        };

        let (resp1, resp2, resp3) = tokio::join!(
            client1.execute_query(tonic::Request::new(query.clone())),
            client2.execute_query(tonic::Request::new(query.clone())),
            client3.execute_query(tonic::Request::new(query.clone())),
        );

        // Verify all responses
        assert!(resp1.is_ok());
        assert!(resp2.is_ok());
        assert!(resp3.is_ok());

        let data1 = resp1.unwrap().into_inner().data.unwrap();
        let data2 = resp2.unwrap().into_inner().data.unwrap();
        let data3 = resp3.unwrap().into_inner().data.unwrap();

        assert!(data1.contains("Hello, Concurrent!"));
        assert!(data2.contains("Hello, Concurrent!"));
        assert!(data3.contains("Hello, Concurrent!"));
    }

    #[tokio::test]
    async fn test_e2e_query_with_variables() {
        // Start server
        let server_url = start_test_server().await;

        // Create client
        let mut client = create_test_client(&server_url).await;

        // Execute query with variables
        let request = tonic::Request::new(GraphQlRequest {
            query: r#"
                mutation CreateUserWithVariables($name: String!, $email: String!) {
                    createUser(input: { name: $name, email: $email }) {
                        name
                        email
                    }
                }
            "#
            .to_string(),
            variables: Some(r#"{"name": "Variable User", "email": "var@test.com"}"#.to_string()),
            operation_name: Some("CreateUserWithVariables".to_string()),
        });

        let response = client
            .execute_mutation(request)
            .await
            .expect("Failed to execute mutation with variables");

        let grpc_resp = response.into_inner();

        // Verify response
        assert!(grpc_resp.data.is_some());
        let data = grpc_resp.data.unwrap();
        assert!(data.contains("Variable User"));
        assert!(data.contains("var@test.com"));
    }

    #[tokio::test]
    async fn test_e2e_error_handling() {
        // Start server
        let server_url = start_test_server().await;

        // Create client
        let mut client = create_test_client(&server_url).await;

        // Execute query with invalid syntax
        let request = tonic::Request::new(GraphQlRequest {
            query: r#"{ invalid syntax }"#.to_string(),
            variables: None,
            operation_name: None,
        });

        let response = client
            .execute_query(request)
            .await
            .expect("Server should respond even with error");

        let grpc_resp = response.into_inner();

        // Verify error response
        assert!(!grpc_resp.errors.is_empty());
    }

    #[tokio::test]
    async fn test_e2e_list_users() {
        // Start server
        let server_url = start_test_server().await;

        // Create client
        let mut client = create_test_client(&server_url).await;

        // Create multiple users
        for i in 1..=3 {
            let request = tonic::Request::new(GraphQlRequest {
                query: format!(
                    r#"mutation {{ createUser(input: {{ name: "User {}", email: "user{}@test.com" }}) {{ id }} }}"#,
                    i, i
                ),
                variables: None,
                operation_name: None,
            });

            client
                .execute_mutation(request)
                .await
                .expect("Failed to create user");
        }

        // Query all users
        let request = tonic::Request::new(GraphQlRequest {
            query: r#"{ users { id name email } }"#.to_string(),
            variables: None,
            operation_name: None,
        });

        let response = client
            .execute_query(request)
            .await
            .expect("Failed to query users");

        let grpc_resp = response.into_inner();

        // Verify response contains all users
        assert!(grpc_resp.data.is_some());
        let data = grpc_resp.data.unwrap();
        assert!(data.contains("User 1"));
        assert!(data.contains("User 2"));
        assert!(data.contains("User 3"));
    }
}
