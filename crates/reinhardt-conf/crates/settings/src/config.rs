use reinhardt_exception::Result;

/// Common configuration trait
///
/// Various `*Config` structures implement this trait to provide
/// a common interface for validation and merging.
pub trait Config: Clone + Send + Sync + 'static {
	/// Validates configuration values. Returns `Error::Validation` if there are problems.
	fn validate(&self) -> Result<()> {
		Ok(())
	}

	/// Merges another configuration with override rules.
	/// Default implementation uses last-wins semantics (`other` takes precedence).
	fn merge(self, other: Self) -> Self {
		other
	}
}
