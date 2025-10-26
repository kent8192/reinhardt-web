//! Performance benchmarks for GraphQL execution
//!
//! Compares performance of:
//! - Direct GraphQL execution (baseline)
//! - GraphQL over gRPC (network protocol overhead)

use async_graphql::{EmptySubscription, Object, Schema};
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

#[cfg(feature = "graphql-grpc")]
use reinhardt_graphql::grpc_service::GraphQLGrpcService;
#[cfg(feature = "graphql-grpc")]
use reinhardt_grpc::proto::graphql::{GraphQlRequest, graph_ql_service_server::GraphQlService};
#[cfg(feature = "graphql-grpc")]
use tonic::Request;

// Test Query type
struct Query;

#[Object]
impl Query {
    async fn hello(&self, name: Option<String>) -> String {
        format!("Hello, {}!", name.unwrap_or_else(|| "World".to_string()))
    }

    async fn compute(&self, n: i32) -> i32 {
        // Simple computation for benchmarking
        (0..n).sum()
    }

    async fn nested_data(&self, depth: i32) -> String {
        // Simulate nested data fetching
        (0..depth)
            .map(|i| format!("Level {}", i))
            .collect::<Vec<_>>()
            .join(" -> ")
    }
}

struct Mutation;

#[Object]
impl Mutation {
    async fn echo(&self, message: String) -> String {
        message
    }
}

// Benchmark: Direct GraphQL execution (baseline)
fn bench_direct_graphql(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let schema = Schema::build(Query, Mutation, EmptySubscription).finish();

    let mut group = c.benchmark_group("direct_graphql");

    // Simple query
    group.bench_function("simple_query", |b| {
        b.to_async(&runtime).iter(|| async {
            let query = r#"{ hello }"#;
            black_box(schema.execute(query).await)
        });
    });

    // Query with arguments
    group.bench_function("query_with_args", |b| {
        b.to_async(&runtime).iter(|| async {
            let query = r#"{ hello(name: "Benchmark") }"#;
            black_box(schema.execute(query).await)
        });
    });

    // Query with computation
    group.bench_function("query_compute", |b| {
        b.to_async(&runtime).iter(|| async {
            let query = r#"{ compute(n: 100) }"#;
            black_box(schema.execute(query).await)
        });
    });

    // Nested data query
    group.bench_function("nested_query", |b| {
        b.to_async(&runtime).iter(|| async {
            let query = r#"{ nested_data: nestedData(depth: 10) }"#;
            black_box(schema.execute(query).await)
        });
    });

    group.finish();
}

// Benchmark: GraphQL over gRPC
#[cfg(feature = "graphql-grpc")]
fn bench_grpc_graphql(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let schema = Schema::build(Query, Mutation, EmptySubscription).finish();
    let service = GraphQLGrpcService::new(schema);

    let mut group = c.benchmark_group("grpc_graphql");

    // Simple query
    group.bench_function("simple_query", |b| {
        b.to_async(&runtime).iter(|| async {
            let request = Request::new(GraphQlRequest {
                query: r#"{ hello }"#.to_string(),
                variables: None,
                operation_name: None,
            });
            black_box(service.execute_query(request).await)
        });
    });

    // Query with arguments
    group.bench_function("query_with_args", |b| {
        b.to_async(&runtime).iter(|| async {
            let request = Request::new(GraphQlRequest {
                query: r#"{ hello(name: "Benchmark") }"#.to_string(),
                variables: None,
                operation_name: None,
            });
            black_box(service.execute_query(request).await)
        });
    });

    // Query with computation
    group.bench_function("query_compute", |b| {
        b.to_async(&runtime).iter(|| async {
            let request = Request::new(GraphQlRequest {
                query: r#"{ compute(n: 100) }"#.to_string(),
                variables: None,
                operation_name: None,
            });
            black_box(service.execute_query(request).await)
        });
    });

    // Nested data query
    group.bench_function("nested_query", |b| {
        b.to_async(&runtime).iter(|| async {
            let request = Request::new(GraphQlRequest {
                query: r#"{ nested_data: nestedData(depth: 10) }"#.to_string(),
                variables: None,
                operation_name: None,
            });
            black_box(service.execute_query(request).await)
        });
    });

    group.finish();
}

// Benchmark: Query complexity comparison
fn bench_query_complexity(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let schema = Schema::build(Query, Mutation, EmptySubscription).finish();

    let mut group = c.benchmark_group("query_complexity");

    for depth in [5, 10, 20, 50].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(depth), depth, |b, &depth| {
            b.to_async(&runtime).iter(|| async {
                let query = format!(r#"{{ nestedData(depth: {}) }}"#, depth);
                black_box(schema.execute(&query).await)
            });
        });
    }

    group.finish();
}

// Benchmark: Mutation performance
fn bench_mutations(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let schema = Schema::build(Query, Mutation, EmptySubscription).finish();

    let mut group = c.benchmark_group("mutations");

    group.bench_function("simple_mutation", |b| {
        b.to_async(&runtime).iter(|| async {
            let query = r#"mutation { echo(message: "test") }"#;
            black_box(schema.execute(query).await)
        });
    });

    group.finish();
}

// Benchmark groups
criterion_group!(
    benches,
    bench_direct_graphql,
    bench_query_complexity,
    bench_mutations,
);

#[cfg(feature = "graphql-grpc")]
criterion_group!(grpc_benches, bench_grpc_graphql);

#[cfg(feature = "graphql-grpc")]
criterion_main!(benches, grpc_benches);

#[cfg(not(feature = "graphql-grpc"))]
criterion_main!(benches);
