//! Dockerfile generation for Reinhardt applications

use super::provider::DeployResult;

/// Generate a Dockerfile for a Reinhardt application
pub fn generate_dockerfile() -> DeployResult<String> {
	Ok(r#"# Reinhardt Application Dockerfile
# Multi-stage build for minimal image size

# Build stage
FROM rust:1.83-bookworm AS builder

WORKDIR /app

# Install dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary
COPY --from=builder /app/target/release/reinhardt /app/reinhardt

# Create non-root user
RUN useradd -r -s /bin/false appuser && \
    chown -R appuser:appuser /app

USER appuser

# Expose port
EXPOSE 8000

# Run the application
CMD ["./reinhardt", "runserver", "0.0.0.0:8000"]
"#
	.to_string())
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_generate_dockerfile_has_rust_base() {
		// Act
		let dockerfile = generate_dockerfile().unwrap();

		// Assert
		assert!(dockerfile.contains("FROM rust:"));
	}

	#[rstest]
	fn test_generate_dockerfile_has_cargo_build() {
		// Act
		let dockerfile = generate_dockerfile().unwrap();

		// Assert
		assert!(dockerfile.contains("cargo build"));
	}

	#[rstest]
	fn test_generate_dockerfile_exposes_port() {
		// Act
		let dockerfile = generate_dockerfile().unwrap();

		// Assert
		assert!(dockerfile.contains("EXPOSE 8000"));
	}

	#[rstest]
	fn test_generate_dockerfile_has_non_root_user() {
		// Act
		let dockerfile = generate_dockerfile().unwrap();

		// Assert
		assert!(dockerfile.contains("non-root"));
		assert!(dockerfile.contains("useradd"));
	}

	#[rstest]
	fn test_generate_dockerfile_has_multi_stage_build() {
		// Act
		let dockerfile = generate_dockerfile().unwrap();

		// Assert
		assert!(dockerfile.contains("AS builder"));
		assert!(dockerfile.contains("FROM debian:"));
	}

	#[rstest]
	fn test_generate_dockerfile_has_runserver_command() {
		// Act
		let dockerfile = generate_dockerfile().unwrap();

		// Assert
		assert!(dockerfile.contains("runserver"));
		assert!(dockerfile.contains("0.0.0.0:8000"));
	}

	#[rstest]
	fn test_generate_dockerfile_has_security_practices() {
		// Act
		let dockerfile = generate_dockerfile().unwrap();

		// Assert
		assert!(dockerfile.contains("USER appuser"));
		assert!(dockerfile.contains("chown -R appuser:appuser"));
	}
}
