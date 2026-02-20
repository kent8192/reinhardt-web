//! gRPC server reflection support
//!
//! This module provides server reflection for gRPC services,
//! enabling tools like `grpcurl` and `grpcui` to work without proto files.

/// File descriptor set for Reinhardt gRPC services
pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("reinhardt_descriptor");

/// Builder for creating reflection service
pub struct ReflectionServiceBuilder {
	services: Vec<&'static [u8]>,
}

impl ReflectionServiceBuilder {
	/// Create a new builder
	pub fn new() -> Self {
		Self {
			services: vec![FILE_DESCRIPTOR_SET],
		}
	}

	/// Add additional file descriptor set
	pub fn add_descriptor(mut self, descriptor: &'static [u8]) -> Self {
		self.services.push(descriptor);
		self
	}

	/// Build the reflection service (v1alpha)
	///
	/// Returns a `ServerReflectionServer` that can be added to a tonic server.
	pub fn build_v1alpha(
		self,
	) -> Result<
		tonic_reflection::server::v1alpha::ServerReflectionServer<
			impl tonic_reflection::server::v1alpha::ServerReflection,
		>,
		tonic_reflection::server::Error,
	> {
		let mut builder = tonic_reflection::server::Builder::configure();
		for descriptor in self.services {
			builder = builder.register_encoded_file_descriptor_set(descriptor);
		}
		builder.build_v1alpha()
	}

	/// Build the reflection service (v1)
	///
	/// Returns a `ServerReflectionServer` that can be added to a tonic server.
	pub fn build_v1(
		self,
	) -> Result<
		tonic_reflection::server::v1::ServerReflectionServer<
			impl tonic_reflection::server::v1::ServerReflection,
		>,
		tonic_reflection::server::Error,
	> {
		let mut builder = tonic_reflection::server::Builder::configure();
		for descriptor in self.services {
			builder = builder.register_encoded_file_descriptor_set(descriptor);
		}
		builder.build_v1()
	}
}

impl Default for ReflectionServiceBuilder {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_file_descriptor_set_not_empty() {
		// Arrange
		// Act & Assert
		assert!(!FILE_DESCRIPTOR_SET.is_empty());
	}

	#[rstest]
	fn test_reflection_service_builder_default() {
		// Arrange
		// Act
		let builder = ReflectionServiceBuilder::default();
		// Assert
		assert_eq!(builder.services.len(), 1);
	}

	#[rstest]
	fn test_reflection_service_builder_add_descriptor() {
		// Arrange
		// Act
		let builder = ReflectionServiceBuilder::new().add_descriptor(FILE_DESCRIPTOR_SET);
		// Assert
		assert_eq!(builder.services.len(), 2);
	}

	#[rstest]
	fn test_reflection_service_builder_build_v1alpha() {
		// Arrange
		let builder = ReflectionServiceBuilder::new();
		// Act
		let result = builder.build_v1alpha();
		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_reflection_service_builder_build_v1() {
		// Arrange
		let builder = ReflectionServiceBuilder::new();
		// Act
		let result = builder.build_v1();
		// Assert
		assert!(result.is_ok());
	}
}
