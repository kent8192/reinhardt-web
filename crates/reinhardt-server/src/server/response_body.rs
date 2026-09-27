//! Stream owned producers with transport backpressure.

use bytes::Bytes;
use http_body_util::Full;
use hyper::body::{Body, Frame, SizeHint};
use reinhardt_http::{Response, StreamBody};
use std::pin::Pin;
use std::task::{Context, Poll};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub(super) enum ServerResponseBody {
	Buffered(Full<Bytes>),
	Stream(Option<StreamBody>),
}

impl ServerResponseBody {
	fn from_response(response: &mut Response) -> Self {
		match response.take_stream_body() {
			Some(stream) => Self::Stream(Some(stream)),
			None => Self::Buffered(Full::new(response.body.clone())),
		}
	}
}

impl Body for ServerResponseBody {
	type Data = Bytes;
	type Error = BoxError;

	fn poll_frame(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Result<Frame<Bytes>, BoxError>>> {
		match self.get_mut() {
			Self::Buffered(body) => Pin::new(body)
				.poll_frame(cx)
				.map(|frame| frame.map(|result| result.map_err(|never| match never {}))),
			Self::Stream(source) => {
				let Some(stream) = source.as_mut() else {
					return Poll::Ready(None);
				};
				match stream.as_mut().poll_next(cx) {
					Poll::Ready(Some(Ok(bytes))) => Poll::Ready(Some(Ok(Frame::data(bytes)))),
					Poll::Ready(result) => {
						// Release the producer at EOF or the first error, even if the
						// transport retains this body until connection teardown.
						*source = None;
						Poll::Ready(result.map(|result| result.map(Frame::data)))
					}
					Poll::Pending => Poll::Pending,
				}
			}
		}
	}

	fn is_end_stream(&self) -> bool {
		match self {
			Self::Buffered(body) => body.is_end_stream(),
			Self::Stream(source) => source.is_none(),
		}
	}

	fn size_hint(&self) -> SizeHint {
		match self {
			Self::Buffered(body) => body.size_hint(),
			Self::Stream(Some(_)) => SizeHint::default(),
			Self::Stream(None) => SizeHint::with_exact(0),
		}
	}
}

pub(super) fn into_hyper_response(mut response: Response) -> hyper::Response<ServerResponseBody> {
	let body = ServerResponseBody::from_response(&mut response);
	let mut output = hyper::Response::new(body);
	*output.status_mut() = response.status;
	*output.headers_mut() = response.headers;
	output
}

pub(super) fn request_body_too_large_response() -> hyper::Response<ServerResponseBody> {
	into_hyper_response(
		Response::new(hyper::StatusCode::PAYLOAD_TOO_LARGE)
			.with_body(Bytes::from_static(b"Request body too large")),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use http_body_util::BodyExt;
	use rstest::rstest;
	use std::sync::Arc;

	#[rstest]
	#[tokio::test]
	async fn stream_transport_polls_on_demand_and_stops_after_errors() {
		// Arrange
		use std::sync::atomic::{AtomicUsize, Ordering};
		let polls = Arc::new(AtomicUsize::new(0));
		let observed = polls.clone();
		let stream = futures_util::stream::poll_fn(move |_| {
			let result = match observed.fetch_add(1, Ordering::SeqCst) {
				0 => Ok(Bytes::from_static(b"first")),
				1 => Err(std::io::Error::other("producer failed").into()),
				_ => panic!("producer must not be polled after its first error"),
			};
			Poll::Ready(Some(result))
		});
		let mut body = into_hyper_response(Response::ok().with_stream(stream)).into_body();
		assert_eq!(polls.load(Ordering::SeqCst), 0);
		assert_eq!(body.size_hint().exact(), None);
		assert!(!body.is_end_stream());
		// Act / Assert: each requested frame polls exactly once; nothing is prefetched.
		assert_eq!(
			body.frame().await.unwrap().unwrap().into_data().unwrap(),
			"first"
		);
		assert_eq!(polls.load(Ordering::SeqCst), 1);
		assert_eq!(
			body.frame().await.unwrap().unwrap_err().to_string(),
			"producer failed"
		);
		assert!(body.is_end_stream());
		assert_eq!(body.size_hint().exact(), Some(0));
		assert!(body.frame().await.is_none());
		assert_eq!(polls.load(Ordering::SeqCst), 2);
	}

	#[rstest]
	#[tokio::test]
	async fn stream_transport_releases_producer_on_completion() {
		// Arrange
		let (sender, receiver) = tokio::sync::oneshot::channel::<()>();
		let stream = futures_util::stream::poll_fn(move |_| {
			let _keep_alive = &sender;
			Poll::Ready(None::<Result<Bytes, BoxError>>)
		});
		let mut response = Response::ok().with_stream(stream);
		let surviving_clone = response.clone();
		let mut body = ServerResponseBody::from_response(&mut response);
		// Act
		assert!(body.frame().await.is_none());
		// Assert
		assert!(body.is_end_stream());
		assert_eq!(body.size_hint().exact(), Some(0));
		assert!(surviving_clone.is_streaming());
		assert!(
			receiver.await.is_err(),
			"EOF must release the producer immediately"
		);
	}
}
