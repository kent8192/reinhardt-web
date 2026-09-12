//! Enforce the transport constraint on the document produced by schema extensions.

use async_graphql::extensions::{
	Extension, ExtensionContext, ExtensionFactory, NextParseQuery, NextPrepareRequest,
};
use async_graphql::parser::types::{DocumentOperations, ExecutableDocument, OperationType};
use async_graphql::{Request, ServerError, ServerResult, Variables};
use std::any::TypeId;
use std::sync::{Arc, OnceLock};
use tonic::Status;

pub(super) struct GuardedSchema;

pub(super) struct OperationCheck {
	expected: OperationType,
	error: OnceLock<Status>,
}

impl OperationCheck {
	pub(super) fn new(expected: OperationType) -> Arc<Self> {
		Arc::new(Self {
			expected,
			error: OnceLock::new(),
		})
	}

	pub(super) fn error(&self) -> Option<Status> {
		self.error.get().cloned()
	}

	fn reject(&self, status: Status) -> ServerError {
		let error = ServerError::new(status.message(), None);
		let _ = self.error.set(status);
		error
	}
}

pub(super) struct OperationGuard;

impl ExtensionFactory for OperationGuard {
	fn create(&self) -> Arc<dyn Extension> {
		Arc::new(OperationGuardExtension::default())
	}
}

struct PreparedOperation {
	check: Arc<OperationCheck>,
	operation_name: Option<String>,
}

#[derive(Default)]
struct OperationGuardExtension {
	prepared: OnceLock<PreparedOperation>,
}

#[async_trait::async_trait]
impl Extension for OperationGuardExtension {
	async fn prepare_request(
		&self,
		ctx: &ExtensionContext<'_>,
		request: Request,
		next: NextPrepareRequest<'_>,
	) -> ServerResult<Request> {
		// Keep the RPC constraint even when an extension replaces the entire request.
		let check = request
			.data
			.get(&TypeId::of::<Arc<OperationCheck>>())
			.and_then(|data| data.downcast_ref::<Arc<OperationCheck>>())
			.cloned();
		let request = next.run(ctx, request).await?;
		if let Some(check) = check {
			let _ = self.prepared.set(PreparedOperation {
				check,
				operation_name: request.operation_name.clone(),
			});
		}
		Ok(request)
	}

	async fn parse_query(
		&self,
		ctx: &ExtensionContext<'_>,
		query: &str,
		variables: &Variables,
		next: NextParseQuery<'_>,
	) -> ServerResult<ExecutableDocument> {
		// This is the first registered extension, so all request and document
		// transformations finish before the transport constraint is checked.
		let result = next.run(ctx, query, variables).await;
		let Some(prepared) = self.prepared.get() else {
			// The same schema can also serve ordinary GraphQL requests.
			return result;
		};
		let document = result.map_err(|error| {
			prepared.check.reject(Status::invalid_argument(format!(
				"Invalid GraphQL document: {}",
				error.message
			)))
		})?;
		validate_operation(
			&document,
			prepared.operation_name.as_deref(),
			prepared.check.expected,
		)
		.map_err(|status| prepared.check.reject(status))?;
		Ok(document)
	}
}

fn validate_operation(
	document: &ExecutableDocument,
	operation_name: Option<&str>,
	expected: OperationType,
) -> Result<(), Status> {
	// Match async-graphql's operation selection, including a lone named operation.
	let operation = match (&document.operations, operation_name) {
		(DocumentOperations::Single(operation), None) => operation,
		(DocumentOperations::Single(_), Some(_)) => {
			return Err(Status::invalid_argument(
				"operation_name cannot select an anonymous operation",
			));
		}
		(DocumentOperations::Multiple(operations), Some(name)) => operations
			.get(name)
			.ok_or_else(|| Status::invalid_argument("operation_name was not found"))?,
		(DocumentOperations::Multiple(operations), None) if operations.len() == 1 => operations
			.values()
			.next()
			.expect("single operation was checked"),
		(DocumentOperations::Multiple(_), None) => {
			return Err(Status::invalid_argument(
				"operation_name is required for documents with multiple operations",
			));
		}
	};
	if operation.node.ty != expected {
		return Err(Status::invalid_argument(format!(
			"Expected a {expected} operation, received {}",
			operation.node.ty
		)));
	}
	Ok(())
}
