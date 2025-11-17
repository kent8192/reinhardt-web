//! HTTP request and response handling for Reinhardt framework.
//!
//! This crate provides core HTTP abstractions including request and response types,
//! header handling, and content negotiation.
//!
//! ## Request Construction
//!
//! Requests are constructed using the builder pattern for type-safe configuration:
//!
//! ```rust
//! use reinhardt_http::Request;
//! use hyper::{Method, HeaderMap};
//!
//! let request = Request::builder()
//!     .method(Method::POST)
//!     .uri("/api/users")
//!     .version(hyper::Version::HTTP_11)
//!     .headers(HeaderMap::new())
//!     .body(bytes::Bytes::from("request body"))
//!     .build()
//!     .unwrap();
//! ```
//!
//! ## Response Creation
//!
//! Responses use helper methods and builder pattern:
//!
//! ```rust
//! use reinhardt_http::Response;
//!
//! // Using helpers
//! let response = Response::ok().with_body("Hello, World!");
//!
//! // With JSON
//! let json_response = Response::ok()
//!     .with_json(&serde_json::json!({"status": "success"}))
//!     .unwrap();
//! ```

pub mod extensions;
pub mod request;
pub mod response;

pub use extensions::Extensions;
pub use request::{Request, RequestBuilder};
pub use response::{Response, StreamBody, StreamingResponse};

// Re-export error types from reinhardt-exception for consistency across the framework
pub use reinhardt_exception::{Error, Result};
