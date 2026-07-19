use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::server_fn::ServerFnError;

/// A stable client-visible validation error for one field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldError {
	/// Stable machine-readable error code.
	pub code: String,
	/// Human-readable validation message.
	pub message: String,
}

/// Field-keyed validation errors returned by model server functions.
pub type FieldErrors = BTreeMap<String, Vec<FieldError>>;

/// Client-visible failures returned by model server function sets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerFnSetError {
	/// A transport or legacy server-function failure.
	Transport(ServerFnError),
	/// Structured field validation failures.
	Validation(FieldErrors),
	/// The request has no authenticated principal.
	Unauthenticated,
	/// The requested resource does not exist.
	NotFound {
		/// Stable resource name safe to expose to clients.
		resource: String,
	},
	/// The principal lacks permission for the requested operation.
	Forbidden,
	/// The operation conflicts with current resource state.
	Conflict {
		/// Stable machine-readable conflict code.
		code: String,
		/// Human-readable conflict message.
		message: String,
	},
	/// An application-defined client-visible failure.
	Application {
		/// Stable machine-readable application code.
		code: String,
		/// Human-readable application message.
		message: String,
		/// Structured application-specific details.
		details: serde_json::Value,
	},
	/// A sanitized internal failure.
	Internal,
}

impl ServerFnSetError {
	/// Return the deterministic HTTP status for this client-visible error.
	#[doc(hidden)]
	pub fn http_status(&self) -> u16 {
		match self {
			Self::Validation(_) | Self::Application { .. } => 400,
			Self::Unauthenticated => 401,
			Self::Forbidden => 403,
			Self::NotFound { .. } => 404,
			Self::Conflict { .. } => 409,
			Self::Transport(_) | Self::Internal => 500,
		}
	}

	/// Decode a model error status, retaining legacy extractor and DI envelopes.
	#[doc(hidden)]
	pub fn http_status_from_body(error_body: &[u8]) -> u16 {
		if let Ok(error) = serde_json::from_slice::<Self>(error_body) {
			return error.http_status();
		}
		serde_json::from_slice::<ServerFnError>(error_body)
			.ok()
			.and_then(|error| match error {
				ServerFnError::Server { status, .. } if (100..=599).contains(&status) => {
					Some(status)
				}
				_ => None,
			})
			.unwrap_or(500)
	}

	/// Decode a failed model client response using the stable fallback order.
	#[doc(hidden)]
	pub fn from_http_error(status: u16, body: &str) -> Self {
		if let Ok(error) = serde_json::from_str::<Self>(body) {
			return error;
		}
		if let Ok(error) = serde_json::from_str::<ServerFnError>(body) {
			return Self::Transport(error);
		}
		Self::Transport(ServerFnError::server(status, body))
	}

	/// Sanitize a generated model handler error before the router returns it.
	#[doc(hidden)]
	#[cfg(native)]
	pub fn sanitize_server_error_body(body: bytes::Bytes) -> bytes::Bytes {
		if let Ok(error) = serde_json::from_slice::<Self>(&body) {
			return match error {
				Self::Transport(error) => {
					tracing::error!(error = %error, "model server function returned a transport error");
					bytes::Bytes::from_static(b"\"Internal\"")
				}
				_ => body,
			};
		}
		if matches!(
			serde_json::from_slice::<ServerFnError>(&body),
			Ok(ServerFnError::Server { status, .. }) if (100..=599).contains(&status)
		) {
			return body;
		}
		bytes::Bytes::from_static(b"\"Internal\"")
	}

	/// Remove server-internal transport details before wire serialization.
	#[doc(hidden)]
	#[cfg(native)]
	pub fn into_server_wire_error(self) -> Self {
		match self {
			Self::Transport(error) => {
				tracing::error!(error = %error, "model server function returned a transport error");
				Self::Internal
			}
			other => other,
		}
	}
}

impl std::fmt::Display for ServerFnSetError {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Transport(error) => write!(formatter, "{error}"),
			Self::Validation(_) => formatter.write_str("Validation failed"),
			Self::Unauthenticated => formatter.write_str("Authentication required"),
			Self::NotFound { resource } => write!(formatter, "{resource} not found"),
			Self::Forbidden => formatter.write_str("Permission denied"),
			Self::Conflict { message, .. } | Self::Application { message, .. } => {
				formatter.write_str(message)
			}
			Self::Internal => formatter.write_str("Internal server error"),
		}
	}
}

impl std::error::Error for ServerFnSetError {}

impl From<ServerFnError> for ServerFnSetError {
	fn from(error: ServerFnError) -> Self {
		Self::Transport(error)
	}
}

impl From<reinhardt_core::exception::Error> for ServerFnSetError {
	fn from(error: reinhardt_core::exception::Error) -> Self {
		tracing::error!(%error, "model server function transaction failed");
		Self::Internal
	}
}
