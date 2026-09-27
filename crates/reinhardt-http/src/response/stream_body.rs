//! Single-use stream ownership shared by cloned response metadata.

use super::StreamBody;
use std::fmt;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(super) struct StreamResponseBody(Arc<Mutex<Option<StreamBody>>>);

impl StreamResponseBody {
	pub(super) fn new(stream: StreamBody) -> Self {
		Self(Arc::new(Mutex::new(Some(stream))))
	}

	pub(super) fn take(self) -> StreamBody {
		// No producer code runs under this lock; it only transfers ownership.
		self.0
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner)
			.take()
			.unwrap_or_else(|| {
				Box::pin(futures::stream::once(async {
					Err(std::io::Error::other("response stream has already been taken").into())
				}))
			})
	}
}

impl fmt::Debug for StreamResponseBody {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("StreamResponseBody")
			.finish_non_exhaustive()
	}
}

impl PartialEq for StreamResponseBody {
	fn eq(&self, other: &Self) -> bool {
		Arc::ptr_eq(&self.0, &other.0)
	}
}

impl Eq for StreamResponseBody {}
