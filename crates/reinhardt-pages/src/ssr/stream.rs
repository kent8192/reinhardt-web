//! Stream output types for server-side rendering.

use bytes::Bytes;
use futures_util::stream::{self, LocalBoxStream};
use futures_util::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};

/// A chunk emitted by server-side rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SsrChunk {
	/// HTML bytes represented as UTF-8 text.
	Html(String),
}

impl SsrChunk {
	/// Converts this chunk into bytes for HTTP body adapters.
	pub fn into_bytes(self) -> Bytes {
		match self {
			Self::Html(html) => Bytes::from(html),
		}
	}

	/// Converts this chunk into a string.
	pub fn into_string(self) -> String {
		match self {
			Self::Html(html) => html,
		}
	}
}

/// Stream of server-rendered HTML chunks.
pub struct SsrStream {
	inner: LocalBoxStream<'static, SsrChunk>,
}

impl SsrStream {
	/// Creates an SSR stream from eager chunks.
	pub fn from_chunks<I>(chunks: I) -> Self
	where
		I: IntoIterator<Item = SsrChunk> + 'static,
		I::IntoIter: 'static,
	{
		Self {
			inner: stream::iter(chunks).boxed_local(),
		}
	}

	/// Creates an SSR stream from another stream.
	pub fn from_stream(source: impl Stream<Item = SsrChunk> + 'static) -> Self {
		Self {
			inner: source.boxed_local(),
		}
	}

	/// Collects all chunks into a single string.
	pub async fn collect_string(mut self) -> String {
		let mut html = String::new();
		while let Some(chunk) = self.inner.next().await {
			html.push_str(&chunk.into_string());
		}
		html
	}
}

impl Stream for SsrStream {
	type Item = SsrChunk;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		self.inner.as_mut().poll_next(cx)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn collect_string_from_chunks() {
		let stream = SsrStream::from_chunks([
			SsrChunk::Html("a".to_string()),
			SsrChunk::Html("b".to_string()),
		]);

		assert_eq!(stream.collect_string().await, "ab");
	}

	#[tokio::test]
	async fn collect_string_from_stream() {
		let stream = SsrStream::from_stream(stream::iter([
			SsrChunk::Html("a".to_string()),
			SsrChunk::Html("b".to_string()),
		]));

		assert_eq!(stream.collect_string().await, "ab");
	}
}
