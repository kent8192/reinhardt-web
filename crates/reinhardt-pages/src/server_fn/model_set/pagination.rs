use serde::{Deserialize, Serialize};

use super::{FieldError, FieldErrors, ServerFnSetError};

const DEFAULT_PAGE_LIMIT: u32 = 25;
const MAX_PAGE_LIMIT: u32 = 100;

/// Requested offset-pagination parameters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageRequest {
	/// Optional item limit. Omission selects the framework default.
	pub limit: Option<u32>,
	/// Zero-based row offset.
	pub offset: u64,
}

impl PageRequest {
	/// Validate and normalize the pagination request.
	pub fn validate(self) -> Result<ValidatedPageRequest, ServerFnSetError> {
		let limit = self.limit.unwrap_or(DEFAULT_PAGE_LIMIT);
		if !(1..=MAX_PAGE_LIMIT).contains(&limit) {
			let mut fields = FieldErrors::new();
			fields.insert(
				"limit".to_owned(),
				vec![FieldError {
					code: "out_of_range".to_owned(),
					message: "limit must be between 1 and 100".to_owned(),
				}],
			);
			return Err(ServerFnSetError::Validation(fields));
		}

		Ok(ValidatedPageRequest {
			limit,
			offset: self.offset,
		})
	}
}

/// Validated pagination parameters safe to apply to a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatedPageRequest {
	/// Bounded item limit in `1..=100`.
	pub limit: u32,
	/// Zero-based row offset.
	pub offset: u64,
}

/// One offset-paginated response page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page<T> {
	/// Items in this page.
	pub items: Vec<T>,
	/// Total item count before slicing.
	pub total: u64,
	/// Applied item limit.
	pub limit: u32,
	/// Applied row offset.
	pub offset: u64,
}

/// Cross-target list-query contract for model server functions.
pub trait ServerFnListQuery {
	/// Return the pagination parameters embedded in this typed query.
	fn page_request(&self) -> PageRequest;
}
