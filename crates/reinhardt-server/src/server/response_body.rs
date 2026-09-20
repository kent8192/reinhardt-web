//! Preserve buffered responses while streaming owned file ranges with backpressure.

use bytes::Bytes;
use http_body_util::Full;
use hyper::body::{Body, Frame, SizeHint};
use reinhardt_http::Response;
use reinhardt_http::response::FileResponseBody;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::task::JoinHandle;

type BoxError = Box<dyn std::error::Error + Send + Sync>;
const CHUNK_SIZE: usize = 64 * 1024;

pub(super) enum ServerResponseBody {
	Buffered(Full<Bytes>),
	File {
		source: FileResponseBody,
		position: u64,
		pending: Option<JoinHandle<std::io::Result<Bytes>>>,
		failed: bool,
	},
}

impl ServerResponseBody {
	fn from_response(response: &Response) -> Self {
		match response.file_body() {
			Some(source) => Self::File {
				source: source.clone(),
				position: 0,
				pending: None,
				failed: false,
			},
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
			Self::File {
				source,
				position,
				pending,
				failed,
			} => {
				if *failed || *position == source.len() {
					return Poll::Ready(None);
				}
				let task = pending.get_or_insert_with(|| {
					let source = source.clone();
					let offset = *position;
					tokio::task::spawn_blocking(move || source.read_chunk(offset, CHUNK_SIZE))
				});
				match Pin::new(task).poll(cx) {
					Poll::Pending => Poll::Pending,
					Poll::Ready(result) => {
						*pending = None;
						let bytes = match result {
							Ok(Ok(bytes)) => bytes,
							Ok(Err(error)) => {
								*failed = true;
								return Poll::Ready(Some(Err(Box::new(error))));
							}
							Err(error) => {
								*failed = true;
								return Poll::Ready(Some(Err(Box::new(error))));
							}
						};
						*position += bytes.len() as u64;
						Poll::Ready(Some(Ok(Frame::data(bytes))))
					}
				}
			}
		}
	}

	fn is_end_stream(&self) -> bool {
		match self {
			Self::Buffered(body) => body.is_end_stream(),
			Self::File {
				source,
				position,
				failed,
				..
			} => *failed || *position == source.len(),
		}
	}

	fn size_hint(&self) -> SizeHint {
		match self {
			Self::Buffered(body) => body.size_hint(),
			Self::File {
				source, position, ..
			} => SizeHint::with_exact(source.len() - position),
		}
	}
}

impl Drop for ServerResponseBody {
	fn drop(&mut self) {
		if let Self::File {
			pending: Some(task),
			..
		} = self
		{
			// Abort a queued read on disconnect. Tokio cannot cancel a running blocking
			// read; that one bounded read owns its file guard until it returns.
			task.abort();
		}
	}
}

pub(super) fn into_hyper_response(response: Response) -> hyper::Response<ServerResponseBody> {
	let body = ServerResponseBody::from_response(&response);
	let mut output = hyper::Response::new(body);
	*output.status_mut() = response.status;
	*output.headers_mut() = response.headers;
	output
}

pub(super) fn request_body_too_large_response() -> hyper::Response<ServerResponseBody> {
	into_hyper_response(
		Response::new(hyper::StatusCode::PAYLOAD_TOO_LARGE)
			.with_static_body(b"Request body too large"),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use http_body_util::BodyExt;
	use rstest::rstest;
	use std::io::Write;
	use std::sync::Arc;

	struct FileHandler(std::fs::File);

	#[async_trait::async_trait]
	impl reinhardt_http::Handler for FileHandler {
		async fn handle(&self, _: reinhardt_http::Request) -> reinhardt_http::Result<Response> {
			let file = self.0.try_clone().unwrap();
			let length = file.metadata().unwrap().len();
			Ok(Response::ok()
				.with_file_body(file, 0, length)
				.unwrap()
				.with_header("content-type", "video/mp4"))
		}
	}

	struct ServerTask(JoinHandle<()>);
	impl Drop for ServerTask {
		fn drop(&mut self) {
			self.0.abort();
		}
	}

	#[rstest]
	#[case(false)]
	#[case(true)]
	#[tokio::test]
	async fn native_http_transports_send_complete_file_bodies(#[case] http2: bool) {
		// Arrange
		let mut file = tempfile::tempfile().unwrap();
		let expected = vec![23; CHUNK_SIZE * 3 + 11];
		file.write_all(&expected).unwrap();
		let handler = Arc::new(FileHandler(file));
		let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
		let address = listener.local_addr().unwrap();
		let _server = ServerTask(tokio::spawn(async move {
			let (stream, peer) = listener.accept().await.unwrap();
			if http2 {
				super::super::http2::Http2Server::handle_connection(stream, handler)
					.await
					.unwrap();
			} else {
				super::super::http::HttpServer::handle_connection(stream, peer, handler, None)
					.await
					.unwrap();
			}
		}));
		let mut builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(10));
		if http2 {
			builder = builder.http2_prior_knowledge();
		}
		let client = builder.build().unwrap();
		// Act
		let response = client
			.get(format!("http://{address}/movie.mp4"))
			.send()
			.await
			.unwrap();
		// Assert
		assert_eq!(response.status(), hyper::StatusCode::OK);
		assert_eq!(response.headers()[hyper::header::CONTENT_TYPE], "video/mp4");
		assert_eq!(response.content_length(), Some(expected.len() as u64));
		assert_eq!(response.bytes().await.unwrap().as_ref(), expected);
	}

	#[rstest]
	#[tokio::test]
	async fn file_transport_preserves_ranges_and_never_emits_an_oversized_frame() {
		// Arrange
		let mut file = tempfile::tempfile().unwrap();
		let content = vec![17; CHUNK_SIZE * 3 + 11];
		file.write_all(&content).unwrap();
		let response = Response::ok()
			.with_file_body(file, 7, (content.len() - 9) as u64)
			.unwrap();
		let mut body = into_hyper_response(response).into_body();
		let mut received = Vec::new();
		// Act
		while let Some(frame) = body.frame().await {
			let data = frame.unwrap().into_data().unwrap();
			assert!(data.len() <= CHUNK_SIZE);
			received.extend_from_slice(&data);
		}
		// Assert
		assert_eq!(received, content[7..content.len() - 2]);
		assert!(body.is_end_stream());
		assert_eq!(body.size_hint().exact(), Some(0));
	}

	#[rstest]
	#[tokio::test]
	async fn truncated_files_fail_the_stream_instead_of_sending_a_successful_short_body() {
		// Arrange
		let file = tempfile::tempfile().unwrap();
		file.set_len(20).unwrap();
		let mutation = file.try_clone().unwrap();
		let response = Response::ok().with_file_body(file, 0, 20).unwrap();
		mutation.set_len(0).unwrap();
		let mut body = into_hyper_response(response).into_body();
		// Act
		let frame = body.frame().await.unwrap();
		// Assert
		assert!(frame.is_err());
		assert!(body.is_end_stream());
	}
}
