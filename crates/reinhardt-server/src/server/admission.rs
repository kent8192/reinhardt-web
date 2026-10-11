//! Optional admission before native HTTP request-body collection.
use async_trait::async_trait;
use hyper::{HeaderMap, Method, Uri};
use reinhardt_http::Response;
use std::{net::SocketAddr, time::Duration};

/// Immutable request metadata available before the body is read.
pub struct RequestHead<'a> {
	/// HTTP method.
	pub method: &'a Method,
	/// Request target.
	pub uri: &'a Uri,
	/// Headers received before the body.
	pub headers: &'a HeaderMap,
	/// Connected peer address, when available.
	pub remote_addr: Option<SocketAddr>,
}

/// A body budget and an owned resource held until handler completion or cancellation.
pub struct AdmittedRequest {
	/// Maximum number of body bytes accepted for this request.
	pub max_body_size: u64,
	/// Maximum time spent receiving the entire body.
	pub receive_timeout: Duration,
	_guard: Box<dyn Send>,
}

impl AdmittedRequest {
	/// Retain `guard` until the request finishes or is cancelled.
	pub fn new<G: Send + 'static>(max_body_size: u64, receive_timeout: Duration, guard: G) -> Self {
		Self {
			max_body_size,
			receive_timeout,
			_guard: Box::new(guard),
		}
	}
}

/// An opt-in hook for bounded, cancellation-safe async request admission.
#[async_trait]
pub trait RequestAdmission: Send + Sync {
	/// Admit a request using metadata available before body collection.
	async fn admit(&self, head: RequestHead<'_>) -> Result<AdmittedRequest, Response>;
}

/// Marks requests whose transport already owns application admission.
#[derive(Clone, Copy)]
pub struct PreBodyAdmitted;
