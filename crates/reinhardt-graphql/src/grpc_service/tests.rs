use super::*;
use async_graphql::parser::types::OperationType;
use rstest::{fixture, rstest};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
struct Calls([AtomicUsize; 3]);

impl Calls {
	fn counts(&self) -> [usize; 3] {
		self.0.each_ref().map(|count| count.load(Ordering::SeqCst))
	}
}

struct Query(Arc<Calls>);
struct Mutation(Arc<Calls>);
struct Subscription(Arc<Calls>);

#[async_graphql::Object]
impl Query {
	async fn value(&self, amount: i32) -> i32 {
		self.0.0[0].fetch_add(1, Ordering::SeqCst);
		amount
	}
}

#[async_graphql::Object]
impl Mutation {
	async fn value(&self, amount: i32) -> i32 {
		self.0.0[1].fetch_add(1, Ordering::SeqCst);
		amount
	}
}

#[async_graphql::Subscription]
impl Subscription {
	async fn value(&self, amount: i32) -> impl Stream<Item = i32> {
		self.0.0[2].fetch_add(1, Ordering::SeqCst);
		tokio_stream::iter([amount])
	}
}

type TestService = GraphQLGrpcService<Query, Mutation, Subscription>;

#[fixture]
fn service() -> (TestService, Arc<Calls>) {
	let calls = Arc::new(Calls::default());
	let schema = Schema::new(
		Query(Arc::clone(&calls)),
		Mutation(Arc::clone(&calls)),
		Subscription(Arc::clone(&calls)),
	);
	(TestService::new(schema), calls)
}

fn request(query: &str, operation_name: Option<&str>) -> Request<GraphQlRequest> {
	Request::new(GraphQlRequest {
		query: query.into(),
		operation_name: operation_name.map(str::to_owned),
		variables: Some(r#"{"amount":7}"#.into()),
	})
}

async fn dispatch(
	service: &TestService,
	rpc: OperationType,
	request: Request<GraphQlRequest>,
) -> Result<GraphQlResponse, Status> {
	match rpc {
		OperationType::Query => service
			.execute_query(request)
			.await
			.map(Response::into_inner),
		OperationType::Mutation => service
			.execute_mutation(request)
			.await
			.map(Response::into_inner),
		OperationType::Subscription => {
			let mut stream = service.execute_subscription(request).await?.into_inner();
			let event = stream.next().await.expect("one subscription event")?;
			assert_eq!(event.event_type, "data");
			assert_eq!(event.id, "1");
			assert_eq!(stream.next().await.map(|_| ()), None);
			Ok(event.payload.expect("subscription payload"))
		}
	}
}

#[rstest]
#[case(OperationType::Query, "mutation", None)]
#[case(OperationType::Query, "subscription", None)]
#[case(OperationType::Mutation, "query", None)]
#[case(OperationType::Mutation, "subscription", None)]
#[case(OperationType::Subscription, "query", None)]
#[case(OperationType::Subscription, "mutation", None)]
#[case(OperationType::Query, "mutation", Some("Selected"))]
#[case(OperationType::Query, "subscription", Some("Selected"))]
#[case(OperationType::Mutation, "query", Some("Selected"))]
#[case(OperationType::Mutation, "subscription", Some("Selected"))]
#[case(OperationType::Subscription, "query", Some("Selected"))]
#[case(OperationType::Subscription, "mutation", Some("Selected"))]
#[tokio::test]
async fn rpc_rejects_mismatched_operation_before_resolver_execution(
	service: (TestService, Arc<Calls>),
	#[case] rpc: OperationType,
	#[case] submitted: &str,
	#[case] operation_name: Option<&str>,
) {
	// Arrange: include a matching decoy before the selected mismatched operation.
	let (service, calls) = service;
	let query = if operation_name.is_some() {
		format!("{rpc} Decoy {{ value(amount: 1) }} {submitted} Selected {{ value(amount: 7) }}")
	} else {
		format!("{submitted} {{ value(amount: 7) }}")
	};
	let request = request(&query, operation_name);

	// Act: subscription mismatches must fail before a stream is returned.
	let result = match rpc {
		OperationType::Query => service.execute_query(request).await.map(|_| ()),
		OperationType::Mutation => service.execute_mutation(request).await.map(|_| ()),
		OperationType::Subscription => service.execute_subscription(request).await.map(|_| ()),
	};

	// Assert: no query, mutation, or subscription resolver ran.
	assert_eq!(calls.counts(), [0, 0, 0]);
	let error = result.expect_err("RPC must reject the selected operation class");
	assert_eq!(error.code(), tonic::Code::InvalidArgument);
	assert_eq!(
		error.message(),
		format!("Expected a {rpc} operation, received {submitted}")
	);
}

#[rstest]
#[case("", None)]
#[case("query {", None)]
#[case("fragment Fields on Query { value(amount: 7) }", None)]
#[case(
	"query Same { value(amount: 7) } mutation Same { value(amount: 7) }",
	Some("Same")
)]
#[case("{ value(amount: 7) } query Read { value(amount: 7) }", None)]
#[case(
	"query Read { value(amount: 7) } mutation Change { value(amount: 7) }",
	None
)]
#[case(
	"query Read { value(amount: 7) } mutation Change { value(amount: 7) }",
	Some("")
)]
#[case(
	"query Read { value(amount: 7) } mutation Change { value(amount: 7) }",
	Some("Missing")
)]
#[case("query Read { value(amount: 7) }", Some("read"))]
#[case("query Read { value(amount: 7) }", Some(" Read "))]
#[case("{ value(amount: 7) }", Some("Read"))]
#[tokio::test]
async fn rpc_rejects_invalid_or_ambiguous_selection(
	service: (TestService, Arc<Calls>),
	#[values(
		OperationType::Query,
		OperationType::Mutation,
		OperationType::Subscription
	)]
	rpc: OperationType,
	#[case] query: &str,
	#[case] operation_name: Option<&str>,
) {
	// Arrange
	let (service, calls) = service;
	let request = request(query, operation_name);

	// Act
	let result = match rpc {
		OperationType::Query => service.execute_query(request).await.map(|_| ()),
		OperationType::Mutation => service.execute_mutation(request).await.map(|_| ()),
		OperationType::Subscription => service.execute_subscription(request).await.map(|_| ()),
	};

	// Assert
	assert_eq!(
		result.expect_err("invalid operation selection").code(),
		tonic::Code::InvalidArgument
	);
	assert_eq!(calls.counts(), [0, 0, 0]);
}

#[rstest]
#[case(false, None)]
#[case(false, Some(""))]
#[case(true, None)]
#[case(true, Some(""))]
#[case(true, Some("Selected"))]
#[tokio::test]
async fn rpc_accepts_single_operation(
	service: (TestService, Arc<Calls>),
	#[values(
		OperationType::Query,
		OperationType::Mutation,
		OperationType::Subscription
	)]
	rpc: OperationType,
	#[case] named: bool,
	#[case] operation_name: Option<&str>,
) {
	// Arrange
	let (service, calls) = service;
	let name = if named { "Selected" } else { "" };
	let query = format!("{rpc} {name}($amount: Int!) {{ value(amount: $amount) }}");

	// Act
	let response = dispatch(&service, rpc, request(&query, operation_name))
		.await
		.unwrap();

	// Assert
	assert_eq!(response.errors, []);
	assert_eq!(response.data.as_deref(), Some("{value: 7}"));
	assert_eq!(
		calls.counts(),
		match rpc {
			OperationType::Query => [1, 0, 0],
			OperationType::Mutation => [0, 1, 0],
			OperationType::Subscription => [0, 0, 1],
		}
	);
}

#[rstest]
#[case(OperationType::Query, "Read", [1, 0, 0])]
#[case(OperationType::Mutation, "Change", [0, 1, 0])]
#[case(OperationType::Subscription, "Watch", [0, 0, 1])]
#[tokio::test]
async fn rpc_selects_matching_operation_with_variables_aliases_and_fragments(
	service: (TestService, Arc<Calls>),
	#[case] rpc: OperationType,
	#[case] operation_name: &str,
	#[case] expected_calls: [usize; 3],
) {
	// Arrange: only the selected operation may execute, regardless of document order.
	let (service, calls) = service;
	let query = "mutation Change($amount: Int!) { ...MutationFields }
		subscription Watch($amount: Int!) { result: value(amount: $amount) }
		query Read($amount: Int!) { ...QueryFields }
		fragment QueryFields on Query { result: value(amount: $amount) }
		fragment MutationFields on Mutation { result: value(amount: $amount) }";

	// Act
	let response = dispatch(&service, rpc, request(query, Some(operation_name)))
		.await
		.unwrap();

	// Assert
	assert_eq!(response.errors, []);
	assert_eq!(response.data.as_deref(), Some("{result: 7}"));
	assert_eq!(calls.counts(), expected_calls);
}

#[rstest]
#[tokio::test]
async fn rpc_accepts_shorthand_query(service: (TestService, Arc<Calls>)) {
	// Arrange
	let (service, calls) = service;

	// Act
	let response = service
		.execute_query(request("{ value(amount: 7) }", None))
		.await
		.unwrap()
		.into_inner();

	// Assert
	assert_eq!(response.errors, []);
	assert_eq!(response.data.as_deref(), Some("{value: 7}"));
	assert_eq!(calls.counts(), [1, 0, 0]);
}

#[rstest]
#[case(OperationType::Query, "Query")]
#[case(OperationType::Mutation, "Mutation")]
#[case(OperationType::Subscription, "Subscription")]
#[tokio::test]
async fn rpc_preserves_schema_validation_errors(
	service: (TestService, Arc<Calls>),
	#[case] rpc: OperationType,
	#[case] root_type: &str,
) {
	// Arrange
	let (service, calls) = service;
	let query = format!("{rpc} {{ missing }}");

	// Act: valid operation selection still reaches schema validation.
	let response = dispatch(&service, rpc, request(&query, None))
		.await
		.unwrap();

	// Assert: schema errors retain their GraphQL response shape.
	assert_eq!(response.data, None);
	assert_eq!(response.errors.len(), 1);
	assert_eq!(
		response.errors[0].message,
		format!("Unknown field \"missing\" on type \"{root_type}\".")
	);
	assert_eq!(calls.counts(), [0, 0, 0]);
}
