//! Owned metadata for named server function sets.

/// Metadata describing a named server function set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerFnSetMetadata {
	/// The set name supplied through `ServerFnSetChainExt::named`.
	pub name: &'static str,
	/// Metadata for each action in builder order.
	pub actions: Vec<ServerFnSetActionMetadata>,
}

/// Metadata describing one action in a server function set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerFnSetActionMetadata {
	/// The generated server function name.
	pub name: &'static str,
	/// The registered endpoint path.
	pub path: &'static str,
	/// The request codec name.
	pub codec: &'static str,
	/// Names of parameters supplied by dependency injection.
	pub injected_params: &'static [&'static str],
	/// Whether the action operates on one resource.
	pub detail: bool,
	/// Whether the action runs inside a framework-owned transaction.
	pub transactional: bool,
}
