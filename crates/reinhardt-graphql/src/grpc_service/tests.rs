use super::*;
use async_graphql::extensions::{
	Extension, ExtensionContext, ExtensionFactory, NextParseQuery, NextPrepareRequest,
};
use async_graphql::parser::types::ExecutableDocument;
use async_graphql::parser::types::OperationType;
use rstest::{fixture, rstest};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Default)]
struct RequestExtension {
	replacement: Option<String>,
	operation_name: Option<String>,
	max_bytes: Option<usize>,
	replace_entire: bool,
	prepared: Arc<AtomicUsize>,
	parsed: Arc<AtomicUsize>,
}

impl ExtensionFactory for RequestExtension {
	fn create(&self) -> Arc<dyn Extension> {
		Arc::new(self.clone())
	}
}

#[async_trait::async_trait]
impl Extension for RequestExtension {
	async fn prepare_request(
		&self,
		ctx: &ExtensionContext<'_>,
		mut request: async_graphql::Request,
		next: NextPrepareRequest<'_>,
	) -> async_graphql::ServerResult<async_graphql::Request> {
		self.prepared.fetch_add(1, Ordering::SeqCst);
		if self
			.max_bytes
			.is_some_and(|limit| request.query.len() > limit)
		{
			return Err(async_graphql::ServerError::new(
				"Document size limit exceeded",
				None,
			));
		}
		if let Some(query) = &self.replacement {
			// Replacing the whole request must not discard the transport's constraint.
			if self.replace_entire {
				let variables = request.variables;
				request = async_graphql::Request::new(query).variables(variables);
			} else {
				request.query.clone_from(query);
			}
			request.operation_name = self.operation_name.clone();
		}
		next.run(ctx, request).await
	}

	async fn parse_query(
		&self,
		ctx: &ExtensionContext<'_>,
		query: &str,
		variables: &async_graphql::Variables,
		next: NextParseQuery<'_>,
	) -> async_graphql::ServerResult<ExecutableDocument> {
		self.parsed.fetch_add(1, Ordering::SeqCst);
		next.run(ctx, query, variables).await
	}
}

fn service_with_extension(extension: RequestExtension) -> (TestService, Arc<Calls>) {
	let calls = Arc::new(Calls::default());
	let schema = TestService::schema_builder(
		Query(Arc::clone(&calls)),
		Mutation(Arc::clone(&calls)),
		Subscription(Arc::clone(&calls)),
	)
	.extension(extension)
	.finish();
	(TestService::new(schema), calls)
}

#[rstest]
#[tokio::test]
async fn rpc_executes_prepared_document(
	#[values(
		OperationType::Query,
		OperationType::Mutation,
		OperationType::Subscription
	)]
	rpc: OperationType,
	#[values(false, true)] invalid_original: bool,
	#[values(false, true)] replace_entire: bool,
) {
	// Arrange: preparation replaces the document, selected name, and request data.
	let prepared = Arc::new(AtomicUsize::new(0));
	let parsed = Arc::new(AtomicUsize::new(0));
	let (service, calls) = service_with_extension(RequestExtension {
		replacement: Some(format!(
			"{rpc} Prepared($amount: Int!) {{ value(amount: $amount) }}"
		)),
		operation_name: Some("Prepared".into()),
		max_bytes: None,
		replace_entire,
		prepared: Arc::clone(&prepared),
		parsed: Arc::clone(&parsed),
	});
	let original = if invalid_original {
		"not a GraphQL document".into()
	} else {
		format!("{rpc} Original {{ value(amount: 1) }}")
	};

	// Act
	let response = dispatch(&service, rpc, request(&original, Some("Original")))
		.await
		.unwrap();

	// Assert: only the prepared document executes, and hooks run once.
	assert_eq!(response.errors, []);
	assert_eq!(response.data.as_deref(), Some("{value: 7}"));
	assert_eq!(calls.counts().iter().sum::<usize>(), 1);
	assert_eq!(prepared.load(Ordering::SeqCst), 1);
	assert_eq!(parsed.load(Ordering::SeqCst), 1);
}

#[rstest]
#[tokio::test]
async fn rpc_runs_preparation_limits_before_parsing(
	#[values(
		OperationType::Query,
		OperationType::Mutation,
		OperationType::Subscription
	)]
	rpc: OperationType,
) {
	// Arrange: this invalid document must be rejected by preparation, before parsing.
	let prepared = Arc::new(AtomicUsize::new(0));
	let parsed = Arc::new(AtomicUsize::new(0));
	let (service, calls) = service_with_extension(RequestExtension {
		replacement: None,
		operation_name: None,
		max_bytes: Some(8),
		replace_entire: false,
		prepared: Arc::clone(&prepared),
		parsed: Arc::clone(&parsed),
	});

	// Act
	let response = dispatch(&service, rpc, request("not a GraphQL document", None))
		.await
		.unwrap();

	// Assert: preserve the extension's rejection and never run a resolver.
	assert_eq!(response.data, None);
	assert_eq!(response.errors.len(), 1);
	assert_eq!(response.errors[0].message, "Document size limit exceeded");
	assert_eq!(calls.counts(), [0, 0, 0]);
	assert_eq!(prepared.load(Ordering::SeqCst), 1);
	assert_eq!(parsed.load(Ordering::SeqCst), 0);
}

#[derive(Clone)]
struct ReplaceDocument(String);

impl ExtensionFactory for ReplaceDocument {
	fn create(&self) -> Arc<dyn Extension> {
		Arc::new(self.clone())
	}
}

#[async_trait::async_trait]
impl Extension for ReplaceDocument {
	async fn parse_query(
		&self,
		ctx: &ExtensionContext<'_>,
		query: &str,
		variables: &async_graphql::Variables,
		next: NextParseQuery<'_>,
	) -> async_graphql::ServerResult<ExecutableDocument> {
		next.run(ctx, query, variables).await?;
		// A user extension can return a different AST after the parser finishes.
		async_graphql::parser::parse_query(&self.0).map_err(Into::into)
	}
}

#[rstest]
#[case(OperationType::Query, OperationType::Mutation)]
#[case(OperationType::Query, OperationType::Subscription)]
#[case(OperationType::Mutation, OperationType::Query)]
#[case(OperationType::Mutation, OperationType::Subscription)]
#[case(OperationType::Subscription, OperationType::Query)]
#[case(OperationType::Subscription, OperationType::Mutation)]
#[tokio::test]
async fn rpc_rejects_class_changes_from_extensions(
	#[case] rpc: OperationType,
	#[case] replacement: OperationType,
	#[values(false, true)] replace_at_parse: bool,
) {
	// Arrange: the original document matches the RPC, but the final one does not.
	let calls = Arc::new(Calls::default());
	let builder = TestService::schema_builder(
		Query(Arc::clone(&calls)),
		Mutation(Arc::clone(&calls)),
		Subscription(Arc::clone(&calls)),
	);
	let query = format!("{replacement} {{ value(amount: 7) }}");
	let schema = if replace_at_parse {
		builder.extension(ReplaceDocument(query)).finish()
	} else {
		builder
			.extension(RequestExtension {
				replacement: Some(query),
				replace_entire: true,
				..Default::default()
			})
			.finish()
	};
	let service = TestService::new(schema);
	let original = format!("{rpc} {{ value(amount: 1) }}");

	// Act
	let error = dispatch(&service, rpc, request(&original, None))
		.await
		.unwrap_err();

	// Assert: every transformed mismatch is rejected before side effects.
	assert_eq!(error.code(), tonic::Code::InvalidArgument);
	assert_eq!(
		error.message(),
		format!("Expected a {rpc} operation, received {replacement}")
	);
	assert_eq!(calls.counts(), [0, 0, 0]);
}

#[rstest]
#[tokio::test]
async fn rpc_executes_document_returned_by_parse_extension(
	#[values(
		OperationType::Query,
		OperationType::Mutation,
		OperationType::Subscription
	)]
	rpc: OperationType,
) {
	// Arrange
	let calls = Arc::new(Calls::default());
	let schema = TestService::schema_builder(
		Query(Arc::clone(&calls)),
		Mutation(Arc::clone(&calls)),
		Subscription(Arc::clone(&calls)),
	)
	.extension(ReplaceDocument(format!("{rpc} {{ value(amount: 7) }}")))
	.finish();
	let service = TestService::new(schema);

	// Act
	let response = dispatch(
		&service,
		rpc,
		request(&format!("{rpc} {{ value(amount: 1) }}"), None),
	)
	.await
	.unwrap();

	// Assert
	assert_eq!(response.errors, []);
	assert_eq!(response.data.as_deref(), Some("{value: 7}"));
	assert_eq!(calls.counts().iter().sum::<usize>(), 1);
}

#[rstest]
#[tokio::test]
async fn schema_remains_shareable_with_ordinary_graphql_requests(
	service: (TestService, Arc<Calls>),
) {
	// Arrange
	let (service, calls) = service;

	// Act: ordinary requests do not carry an RPC operation constraint.
	let response = service
		.schema
		.execute("mutation { value(amount: 7) }")
		.await;

	// Assert
	assert_eq!(response.errors, []);
	assert_eq!(response.data, async_graphql::value!({"value": 7}));
	assert_eq!(calls.counts(), [0, 1, 0]);
}

#[rstest]
#[tokio::test]
async fn concurrent_rpcs_keep_independent_constraints(service: (TestService, Arc<Calls>)) {
	// Arrange
	let (service, calls) = service;

	// Act
	let (query, mutation, subscription) = tokio::join!(
		dispatch(
			&service,
			OperationType::Query,
			request("mutation { value(amount: 1) }", None)
		),
		dispatch(
			&service,
			OperationType::Mutation,
			request("mutation { value(amount: 7) }", None)
		),
		dispatch(
			&service,
			OperationType::Subscription,
			request("subscription { value(amount: 7) }", None)
		),
	);

	// Assert
	assert_eq!(query.unwrap_err().code(), tonic::Code::InvalidArgument);
	assert_eq!(mutation.unwrap().data.as_deref(), Some("{value: 7}"));
	assert_eq!(subscription.unwrap().data.as_deref(), Some("{value: 7}"));
	assert_eq!(calls.counts(), [0, 1, 1]);
}

#[rstest]
#[should_panic(
	expected = "GraphQLGrpcService requires a schema built with GraphQLGrpcService::schema_builder"
)]
fn service_rejects_schema_without_operation_guard() {
	// Arrange
	let calls = Arc::new(Calls::default());
	let schema = Schema::new(
		Query(Arc::clone(&calls)),
		Mutation(Arc::clone(&calls)),
		Subscription(calls),
	);

	// Act / Assert: an unguarded schema cannot become an executable gRPC service.
	TestService::new(schema);
}

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
	let schema = TestService::schema_builder(
		Query(Arc::clone(&calls)),
		Mutation(Arc::clone(&calls)),
		Subscription(Arc::clone(&calls)),
	)
	.finish();
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

	// Act: drive subscription preparation without allowing a resolver to run.
	let result = dispatch(&service, rpc, request).await;

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
	let result = dispatch(&service, rpc, request).await;

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
