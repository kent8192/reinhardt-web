use bytes::Bytes;
use futures::stream::Stream;
use hyper::{HeaderMap, StatusCode};
use serde::Serialize;
use std::pin::Pin;

/// HTTP Response representation
pub struct Response {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
    /// Indicates whether the middleware chain should stop processing
    /// When true, no further middleware or handlers will be executed
    stop_chain: bool,
}

/// Streaming HTTP Response
pub struct StreamingResponse<S> {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub stream: S,
}

/// Type alias for streaming body
pub type StreamBody =
    Pin<Box<dyn Stream<Item = Result<Bytes, Box<dyn std::error::Error + Send + Sync>>> + Send>>;

impl Response {
    /// Create a new Response with the given status code
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Response;
    /// use hyper::StatusCode;
    ///
    /// let response = Response::new(StatusCode::OK);
    /// assert_eq!(response.status, StatusCode::OK);
    /// assert!(response.body.is_empty());
    /// ```
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            headers: HeaderMap::new(),
            body: Bytes::new(),
            stop_chain: false,
        }
    }
    /// Create a Response with HTTP 200 OK status
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Response;
    /// use hyper::StatusCode;
    ///
    /// let response = Response::ok();
    /// assert_eq!(response.status, StatusCode::OK);
    /// ```
    pub fn ok() -> Self {
        Self::new(StatusCode::OK)
    }
    /// Create a Response with HTTP 201 Created status
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Response;
    /// use hyper::StatusCode;
    ///
    /// let response = Response::created();
    /// assert_eq!(response.status, StatusCode::CREATED);
    /// ```
    pub fn created() -> Self {
        Self::new(StatusCode::CREATED)
    }
    /// Create a Response with HTTP 204 No Content status
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Response;
    /// use hyper::StatusCode;
    ///
    /// let response = Response::no_content();
    /// assert_eq!(response.status, StatusCode::NO_CONTENT);
    /// ```
    pub fn no_content() -> Self {
        Self::new(StatusCode::NO_CONTENT)
    }
    /// Create a Response with HTTP 400 Bad Request status
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Response;
    /// use hyper::StatusCode;
    ///
    /// let response = Response::bad_request();
    /// assert_eq!(response.status, StatusCode::BAD_REQUEST);
    /// ```
    pub fn bad_request() -> Self {
        Self::new(StatusCode::BAD_REQUEST)
    }
    /// Create a Response with HTTP 401 Unauthorized status
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Response;
    /// use hyper::StatusCode;
    ///
    /// let response = Response::unauthorized();
    /// assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    /// ```
    pub fn unauthorized() -> Self {
        Self::new(StatusCode::UNAUTHORIZED)
    }
    /// Create a Response with HTTP 403 Forbidden status
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Response;
    /// use hyper::StatusCode;
    ///
    /// let response = Response::forbidden();
    /// assert_eq!(response.status, StatusCode::FORBIDDEN);
    /// ```
    pub fn forbidden() -> Self {
        Self::new(StatusCode::FORBIDDEN)
    }
    /// Create a Response with HTTP 404 Not Found status
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Response;
    /// use hyper::StatusCode;
    ///
    /// let response = Response::not_found();
    /// assert_eq!(response.status, StatusCode::NOT_FOUND);
    /// ```
    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND)
    }
    /// Create a Response with HTTP 500 Internal Server Error status
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Response;
    /// use hyper::StatusCode;
    ///
    /// let response = Response::internal_server_error();
    /// assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    /// ```
    pub fn internal_server_error() -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR)
    }
    /// Create a Response with HTTP 410 Gone status
    ///
    /// Used when a resource has been permanently removed.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Response;
    /// use hyper::StatusCode;
    ///
    /// let response = Response::gone();
    /// assert_eq!(response.status, StatusCode::GONE);
    /// ```
    pub fn gone() -> Self {
        Self::new(StatusCode::GONE)
    }
    /// Create a Response with HTTP 301 Moved Permanently (permanent redirect)
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Response;
    /// use hyper::StatusCode;
    ///
    /// let response = Response::permanent_redirect("/new-location");
    /// assert_eq!(response.status, StatusCode::MOVED_PERMANENTLY);
    /// assert_eq!(
    ///     response.headers.get("location").unwrap().to_str().unwrap(),
    ///     "/new-location"
    /// );
    /// ```
    pub fn permanent_redirect(location: impl AsRef<str>) -> Self {
        Self::new(StatusCode::MOVED_PERMANENTLY).with_location(location.as_ref())
    }
    /// Create a Response with HTTP 302 Found (temporary redirect)
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Response;
    /// use hyper::StatusCode;
    ///
    /// let response = Response::temporary_redirect("/temp-location");
    /// assert_eq!(response.status, StatusCode::FOUND);
    /// assert_eq!(
    ///     response.headers.get("location").unwrap().to_str().unwrap(),
    ///     "/temp-location"
    /// );
    /// ```
    pub fn temporary_redirect(location: impl AsRef<str>) -> Self {
        Self::new(StatusCode::FOUND).with_location(location.as_ref())
    }
    /// Create a Response with HTTP 307 Temporary Redirect (preserves HTTP method)
    ///
    /// Unlike 302, this guarantees the request method is preserved during redirect.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Response;
    /// use hyper::StatusCode;
    ///
    /// let response = Response::temporary_redirect_preserve_method("/temp-location");
    /// assert_eq!(response.status, StatusCode::TEMPORARY_REDIRECT);
    /// assert_eq!(
    ///     response.headers.get("location").unwrap().to_str().unwrap(),
    ///     "/temp-location"
    /// );
    /// ```
    pub fn temporary_redirect_preserve_method(location: impl AsRef<str>) -> Self {
        Self::new(StatusCode::TEMPORARY_REDIRECT).with_location(location.as_ref())
    }
    /// Set the response body
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Response;
    /// use bytes::Bytes;
    ///
    /// let response = Response::ok().with_body("Hello, World!");
    /// assert_eq!(response.body, Bytes::from("Hello, World!"));
    /// ```
    pub fn with_body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = body.into();
        self
    }
    /// Add a custom header to the response
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Response;
    ///
    /// let response = Response::ok().with_header("X-Custom-Header", "custom-value");
    /// assert_eq!(
    ///     response.headers.get("X-Custom-Header").unwrap().to_str().unwrap(),
    ///     "custom-value"
    /// );
    /// ```
    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        if let Ok(header_name) = hyper::header::HeaderName::from_bytes(name.as_bytes()) {
            if let Ok(header_value) = hyper::header::HeaderValue::from_str(value) {
                self.headers.insert(header_name, header_value);
            }
        }
        self
    }
    /// Add a Location header to the response (typically used for redirects)
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Response;
    /// use hyper::StatusCode;
    ///
    /// let response = Response::new(StatusCode::FOUND).with_location("/redirect-target");
    /// assert_eq!(
    ///     response.headers.get("location").unwrap().to_str().unwrap(),
    ///     "/redirect-target"
    /// );
    /// ```
    pub fn with_location(mut self, location: &str) -> Self {
        if let Ok(value) = hyper::header::HeaderValue::from_str(location) {
            self.headers.insert(hyper::header::LOCATION, value);
        }
        self
    }
    /// Set the response body to JSON and add appropriate Content-Type header
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Response;
    /// use serde_json::json;
    ///
    /// let data = json!({"message": "Hello, World!"});
    /// let response = Response::ok().with_json(&data).unwrap();
    ///
    /// assert_eq!(
    ///     response.headers.get("content-type").unwrap().to_str().unwrap(),
    ///     "application/json"
    /// );
    /// ```
    pub fn with_json<T: Serialize>(mut self, data: &T) -> crate::Result<Self> {
        use crate::Error;
        let json = serde_json::to_vec(data).map_err(|e| Error::Serialization(e.to_string()))?;
        self.body = Bytes::from(json);
        self.headers.insert(
            hyper::header::CONTENT_TYPE,
            hyper::header::HeaderValue::from_static("application/json"),
        );
        Ok(self)
    }
    /// Add a custom header using typed HeaderName and HeaderValue
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Response;
    /// use hyper::header::{HeaderName, HeaderValue};
    ///
    /// let header_name = HeaderName::from_static("x-custom-header");
    /// let header_value = HeaderValue::from_static("custom-value");
    /// let response = Response::ok().with_typed_header(header_name, header_value);
    ///
    /// assert_eq!(
    ///     response.headers.get("x-custom-header").unwrap().to_str().unwrap(),
    ///     "custom-value"
    /// );
    /// ```
    pub fn with_typed_header(
        mut self,
        key: hyper::header::HeaderName,
        value: hyper::header::HeaderValue,
    ) -> Self {
        self.headers.insert(key, value);
        self
    }

    /// Check if this response should stop the middleware chain
    ///
    /// When true, no further middleware or handlers will be executed.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Response;
    ///
    /// let response = Response::ok();
    /// assert!(!response.should_stop_chain());
    ///
    /// let stopping_response = Response::ok().with_stop_chain(true);
    /// assert!(stopping_response.should_stop_chain());
    /// ```
    pub fn should_stop_chain(&self) -> bool {
        self.stop_chain
    }

    /// Set whether this response should stop the middleware chain
    ///
    /// When set to true, the middleware chain will stop processing and return
    /// this response immediately, skipping any remaining middleware and handlers.
    ///
    /// This is useful for early returns in middleware, such as:
    /// - Authentication failures (401 Unauthorized)
    /// - CORS preflight responses (204 No Content)
    /// - Rate limiting rejections (429 Too Many Requests)
    /// - Cache hits (304 Not Modified)
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::Response;
    /// use hyper::StatusCode;
    ///
    /// // Early return for authentication failure
    /// let auth_failure = Response::unauthorized()
    ///     .with_body("Authentication required")
    ///     .with_stop_chain(true);
    /// assert!(auth_failure.should_stop_chain());
    ///
    /// // CORS preflight response
    /// let preflight = Response::no_content()
    ///     .with_header("Access-Control-Allow-Origin", "*")
    ///     .with_stop_chain(true);
    /// assert!(preflight.should_stop_chain());
    /// ```
    pub fn with_stop_chain(mut self, stop: bool) -> Self {
        self.stop_chain = stop;
        self
    }
}

impl From<crate::Error> for Response {
    fn from(error: crate::Error) -> Self {
        let status =
            StatusCode::from_u16(error.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = serde_json::json!({
            "error": error.to_string(),
        });

        Response::new(status)
            .with_json(&body)
            .unwrap_or_else(|_| Response::internal_server_error())
    }
}

impl<S> StreamingResponse<S>
where
    S: Stream<Item = Result<Bytes, Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
{
    /// Create a new streaming response with OK status
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::StreamingResponse;
    /// use hyper::StatusCode;
    /// use futures::stream;
    /// use bytes::Bytes;
    ///
    /// let data = vec![Ok(Bytes::from("chunk1")), Ok(Bytes::from("chunk2"))];
    /// let stream = stream::iter(data);
    /// let response = StreamingResponse::new(stream);
    ///
    /// assert_eq!(response.status, StatusCode::OK);
    /// ```
    pub fn new(stream: S) -> Self {
        Self {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            stream,
        }
    }
    /// Create a streaming response with a specific status code
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::StreamingResponse;
    /// use hyper::StatusCode;
    /// use futures::stream;
    /// use bytes::Bytes;
    ///
    /// let data = vec![Ok(Bytes::from("data"))];
    /// let stream = stream::iter(data);
    /// let response = StreamingResponse::with_status(stream, StatusCode::PARTIAL_CONTENT);
    ///
    /// assert_eq!(response.status, StatusCode::PARTIAL_CONTENT);
    /// ```
    pub fn with_status(stream: S, status: StatusCode) -> Self {
        Self {
            status,
            headers: HeaderMap::new(),
            stream,
        }
    }
    /// Set the status code
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::StreamingResponse;
    /// use hyper::StatusCode;
    /// use futures::stream;
    /// use bytes::Bytes;
    ///
    /// let data = vec![Ok(Bytes::from("data"))];
    /// let stream = stream::iter(data);
    /// let response = StreamingResponse::new(stream).status(StatusCode::ACCEPTED);
    ///
    /// assert_eq!(response.status, StatusCode::ACCEPTED);
    /// ```
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }
    /// Add a header to the streaming response
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::StreamingResponse;
    /// use hyper::header::{HeaderName, HeaderValue, CACHE_CONTROL};
    /// use futures::stream;
    /// use bytes::Bytes;
    ///
    /// let data = vec![Ok(Bytes::from("data"))];
    /// let stream = stream::iter(data);
    /// let response = StreamingResponse::new(stream)
    ///     .header(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    ///
    /// assert_eq!(
    ///     response.headers.get(CACHE_CONTROL).unwrap().to_str().unwrap(),
    ///     "no-cache"
    /// );
    /// ```
    pub fn header(
        mut self,
        key: hyper::header::HeaderName,
        value: hyper::header::HeaderValue,
    ) -> Self {
        self.headers.insert(key, value);
        self
    }
    /// Set the Content-Type header (media type)
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_http::StreamingResponse;
    /// use hyper::header::CONTENT_TYPE;
    /// use futures::stream;
    /// use bytes::Bytes;
    ///
    /// let data = vec![Ok(Bytes::from("data"))];
    /// let stream = stream::iter(data);
    /// let response = StreamingResponse::new(stream).media_type("video/mp4");
    ///
    /// assert_eq!(
    ///     response.headers.get(CONTENT_TYPE).unwrap().to_str().unwrap(),
    ///     "video/mp4"
    /// );
    /// ```
    pub fn media_type(self, media_type: &str) -> Self {
        self.header(
            hyper::header::CONTENT_TYPE,
            hyper::header::HeaderValue::from_str(media_type).unwrap_or_else(|_| {
                hyper::header::HeaderValue::from_static("application/octet-stream")
            }),
        )
    }
}

impl<S> StreamingResponse<S> {
    /// Consume the response and return the underlying stream
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_http::StreamingResponse;
    /// use futures::stream::{self, StreamExt};
    /// use bytes::Bytes;
    ///
    /// async fn example() {
    ///     let data = vec![Ok(Bytes::from("chunk1")), Ok(Bytes::from("chunk2"))];
    ///     let stream = stream::iter(data);
    ///     let response = StreamingResponse::new(stream);
    ///
    ///     let mut extracted_stream = response.into_stream();
    ///     let first_chunk = extracted_stream.next().await.unwrap().unwrap();
    ///     assert_eq!(first_chunk, Bytes::from("chunk1"));
    /// }
    /// ```
    pub fn into_stream(self) -> S {
        self.stream
    }
}
