//! Value types describing route-level version information.

/// A single (namespace, path-prefix) pair discovered on a router's route.
///
/// Implementations of [`crate::VersionedRouter`] construct these from
/// their internal route storage. `reinhardt-rest::versioning` then
/// applies its configured pattern (e.g. `"/v{version}/"`) to
/// `path_prefix` to extract the version string.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RouteVersionInfo {
	/// The namespace label attached to the route, if any
	/// (e.g. `"v1"` from `Route::with_namespace("v1")`).
	pub namespace: Option<String>,
	/// The registered route path/prefix as stored by the router
	/// (e.g. `"/v1/users/"`).
	///
	/// "Prefix" here means the full leading path segment that the
	/// router associates with this route — it is **not** trimmed of
	/// the version segment. Callers typically feed this whole string
	/// to a version-extracting regex so pattern strategies remain free
	/// to anchor wherever they need (e.g. `"/v{version}/"` matches at
	/// the start of the path).
	pub path_prefix: String,
}

impl RouteVersionInfo {
	/// Construct a [`RouteVersionInfo`] from a namespace label and a
	/// path prefix.
	pub fn new(namespace: Option<String>, path_prefix: impl Into<String>) -> Self {
		Self {
			namespace,
			path_prefix: path_prefix.into(),
		}
	}
}
