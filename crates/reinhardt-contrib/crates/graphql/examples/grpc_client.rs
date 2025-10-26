//! Example: GraphQL over gRPC Client
//!
//! This example demonstrates how to connect to a GraphQL gRPC server and execute queries/mutations.
//!
//! Make sure the server is running first:
//! ```bash
//! cargo run --package reinhardt-graphql --features graphql-grpc --example grpc_server
//! ```
//!
//! Then run this client:
//! ```bash
//! cargo run --package reinhardt-graphql --features graphql-grpc --example grpc_client
//! ```

#[cfg(feature = "graphql-grpc")]
use reinhardt_grpc::proto::graphql::{
    GraphQlRequest, graph_ql_service_client::GraphQlServiceClient,
};

#[cfg(feature = "graphql-grpc")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to GraphQL gRPC Server...");

    // Connect to server
    let mut client = GraphQlServiceClient::connect("http://127.0.0.1:50051").await?;
    println!("Connected!\n");

    // Example 1: Simple query
    println!("=== Example 1: Simple Query ===");
    let request = tonic::Request::new(GraphQlRequest {
        query: r#"{ hello }"#.to_string(),
        variables: None,
        operation_name: None,
    });

    let response = client.execute_query(request).await?;
    let data = response.into_inner();
    println!("Query: {{ hello }}");
    println!("Response: {}\n", data.data.unwrap_or_default());

    // Example 2: Query with arguments
    println!("=== Example 2: Query with Arguments ===");
    let request = tonic::Request::new(GraphQlRequest {
        query: r#"{ hello(name: "gRPC Client") }"#.to_string(),
        variables: None,
        operation_name: None,
    });

    let response = client.execute_query(request).await?;
    let data = response.into_inner();
    println!("Query: {{ hello(name: \"gRPC Client\") }}");
    println!("Response: {}\n", data.data.unwrap_or_default());

    // Example 3: Create user mutation
    println!("=== Example 3: Create User Mutation ===");
    let request = tonic::Request::new(GraphQlRequest {
        query: r#"
            mutation {
                createUser(input: {
                    name: "Alice Johnson",
                    email: "alice@example.com"
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

    let response = client.execute_mutation(request).await?;
    let data = response.into_inner();
    println!("Mutation: createUser");
    println!("Response: {}\n", data.data.clone().unwrap_or_default());

    // Extract user ID for next example (simple parsing)
    let user_id = if let Some(ref data_str) = data.data {
        data_str
            .split('"')
            .find(|s| s.contains('-'))
            .unwrap_or("unknown")
            .to_string()
    } else {
        "unknown".to_string()
    };

    // Example 4: Query created user
    println!("=== Example 4: Query Created User ===");
    let request = tonic::Request::new(GraphQlRequest {
        query: format!(
            r#"{{ user(id: "{}") {{ id name email active }} }}"#,
            user_id
        ),
        variables: None,
        operation_name: None,
    });

    let response = client.execute_query(request).await?;
    let data = response.into_inner();
    println!("Query: user(id: \"{}\")", user_id);
    println!("Response: {}\n", data.data.unwrap_or_default());

    // Example 5: Create multiple users
    println!("=== Example 5: Create Multiple Users ===");
    for i in 1..=3 {
        let request = tonic::Request::new(GraphQlRequest {
            query: format!(
                r#"mutation {{ createUser(input: {{ name: "User {}", email: "user{}@example.com" }}) {{ id name }} }}"#,
                i, i
            ),
            variables: None,
            operation_name: None,
        });

        let response = client.execute_mutation(request).await?;
        let data = response.into_inner();
        println!("Created: {}", data.data.unwrap_or_default());
    }
    println!();

    // Example 6: List all users
    println!("=== Example 6: List All Users ===");
    let request = tonic::Request::new(GraphQlRequest {
        query: r#"{ users { id name email active } }"#.to_string(),
        variables: None,
        operation_name: None,
    });

    let response = client.execute_query(request).await?;
    let data = response.into_inner();
    println!("Query: {{ users }}");
    println!("Response: {}\n", data.data.unwrap_or_default());

    // Example 7: Mutation with variables
    println!("=== Example 7: Mutation with Variables ===");
    let request = tonic::Request::new(GraphQlRequest {
        query: r#"
            mutation CreateUser($name: String!, $email: String!) {
                createUser(input: { name: $name, email: $email }) {
                    id
                    name
                    email
                }
            }
        "#
        .to_string(),
        variables: Some(r#"{"name": "Bob Smith", "email": "bob@example.com"}"#.to_string()),
        operation_name: Some("CreateUser".to_string()),
    });

    let response = client.execute_mutation(request).await?;
    let data = response.into_inner();
    println!("Mutation: CreateUser with variables");
    println!("Variables: {{\"name\": \"Bob Smith\", \"email\": \"bob@example.com\"}}");
    println!("Response: {}\n", data.data.unwrap_or_default());

    // Example 8: Update user status
    println!("=== Example 8: Update User Status ===");
    let request = tonic::Request::new(GraphQlRequest {
        query: format!(
            r#"mutation {{ updateUserStatus(id: "{}", active: false) {{ id name active }} }}"#,
            user_id
        ),
        variables: None,
        operation_name: None,
    });

    let response = client.execute_mutation(request).await?;
    let data = response.into_inner();
    println!("Mutation: updateUserStatus");
    println!("Response: {}\n", data.data.unwrap_or_default());

    println!("All examples completed successfully!");

    Ok(())
}

#[cfg(not(feature = "graphql-grpc"))]
fn main() {
    eprintln!("This example requires the 'graphql-grpc' feature.");
    eprintln!(
        "Run with: cargo run --package reinhardt-graphql --features graphql-grpc --example grpc_client"
    );
    std::process::exit(1);
}
