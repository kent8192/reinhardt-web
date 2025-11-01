use thiserror::Error;

#[derive(Debug, Error)]
pub enum GrpcError {
	#[error("Connection error: {0}")]
	Connection(String),

	#[error("Service error: {0}")]
	Service(String),

	#[error("Not found: {0}")]
	NotFound(String),

	#[error("Invalid argument: {0}")]
	InvalidArgument(String),

	#[error("Internal error: {0}")]
	Internal(String),
}

pub type GrpcResult<T> = Result<T, GrpcError>;

impl From<tonic::Status> for GrpcError {
	fn from(status: tonic::Status) -> Self {
		match status.code() {
			tonic::Code::NotFound => GrpcError::NotFound(status.message().to_string()),
			tonic::Code::InvalidArgument => {
				GrpcError::InvalidArgument(status.message().to_string())
			}
			tonic::Code::Unavailable => GrpcError::Connection(status.message().to_string()),
			_ => GrpcError::Internal(status.message().to_string()),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_error_display() {
		let err = GrpcError::Connection("test error".to_string());
		assert!(err.to_string().contains("Connection error"));

		let err = GrpcError::NotFound("item".to_string());
		assert!(err.to_string().contains("Not found"));
	}

	#[test]
	fn test_from_tonic_status() {
		let status = tonic::Status::not_found("User not found");
		let error = GrpcError::from(status);
		assert!(matches!(error, GrpcError::NotFound(_)));

		let status = tonic::Status::invalid_argument("Invalid ID");
		let error = GrpcError::from(status);
		assert!(matches!(error, GrpcError::InvalidArgument(_)));

		let status = tonic::Status::unavailable("Service unavailable");
		let error = GrpcError::from(status);
		assert!(matches!(error, GrpcError::Connection(_)));
	}
}
