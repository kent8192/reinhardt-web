//! Native `MockServiceWorker` runtime.

/// Policy for requests that do not match a registered handler.
#[derive(Debug, Clone)]
pub enum UnhandledPolicy {
	/// Return a deterministic diagnostic error response.
	Error,
	/// Native passthrough is not supported in the first native runtime.
	Passthrough,
	/// Log a warning and return the same deterministic response as `Error`.
	Warn,
}

/// Native MSW-style mock server for HTTP clients that accept explicit endpoints.
pub struct MockServiceWorker;
