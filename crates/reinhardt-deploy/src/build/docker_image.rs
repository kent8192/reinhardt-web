//! Docker image building utilities for Reinhardt deploy.
//!
//! Provides Dockerfile generation with multi-stage builds optimized for Rust
//! applications, along with helpers for constructing Docker CLI arguments.

/// Options for building a Docker image of a Reinhardt application.
#[derive(Debug, Clone)]
pub struct DockerBuildOptions {
	/// Application name used for labeling and default binary name.
	pub app_name: String,
	/// Port to expose in the container.
	pub port: u16,
	/// Health check endpoint path (e.g., "/health/").
	pub health_check: String,
	/// Override the binary name copied from the build stage.
	/// Falls back to `app_name` when `None`.
	pub binary_name: Option<String>,
}

/// Generate a multi-stage Dockerfile for a Rust application.
///
/// Stage 1 (builder): compiles the project in release mode.
/// Stage 2 (runtime): copies the binary into a minimal Debian image with
/// health check and port exposure configured.
pub fn generate_dockerfile(opts: &DockerBuildOptions) -> String {
	let binary = opts.binary_name.as_deref().unwrap_or(&opts.app_name);

	format!(
		r#"# Stage 1: Build the application
FROM rust:latest AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin {binary}

# Stage 2: Create a minimal runtime image
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/{binary} /usr/local/bin/{binary}
EXPOSE {port}
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:{port}{health_check} || exit 1
CMD ["{binary}"]
"#,
		binary = binary,
		port = opts.port,
		health_check = opts.health_check,
	)
}

/// Construct a Docker image tag from registry, application name, and commit hash.
///
/// Returns a tag in the format `registry/app_name:commit`.
pub fn build_image_tag(registry: &str, app_name: &str, commit: &str) -> String {
	format!("{registry}/{app_name}:{commit}")
}

/// Build the argument list for a `docker build` command.
///
/// Returns args suitable for passing to a process builder:
/// `["build", "-t", tag, context_dir]`.
pub fn build_docker_args(tag: &str, context_dir: &str) -> Vec<String> {
	vec![
		"build".to_string(),
		"-t".to_string(),
		tag.to_string(),
		context_dir.to_string(),
	]
}

/// Build the argument list for a `docker push` command.
///
/// Returns args suitable for passing to a process builder:
/// `["push", tag]`.
pub fn push_docker_args(tag: &str) -> Vec<String> {
	vec!["push".to_string(), tag.to_string()]
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn dockerfile_generation_minimal() {
		// Arrange
		let opts = DockerBuildOptions {
			app_name: "myapp".to_string(),
			port: 8000,
			health_check: "/health/".to_string(),
			binary_name: None,
		};

		// Act
		let dockerfile = generate_dockerfile(&opts);

		// Assert
		assert!(dockerfile.contains("FROM rust:"));
		assert!(dockerfile.contains("EXPOSE 8000"));
		assert!(dockerfile.contains("HEALTHCHECK"));
		assert!(dockerfile.contains("/health/"));
	}

	#[rstest]
	fn dockerfile_generation_with_binary_name() {
		// Arrange
		let opts = DockerBuildOptions {
			app_name: "myapp".to_string(),
			port: 8000,
			health_check: "/health/".to_string(),
			binary_name: Some("myapp-server".to_string()),
		};

		// Act
		let dockerfile = generate_dockerfile(&opts);

		// Assert
		assert!(dockerfile.contains("myapp-server"));
	}

	#[rstest]
	fn image_tag_from_commit() {
		// Arrange
		let registry = "myregistry";
		let app_name = "myapp";
		let commit = "abc1234";

		// Act
		let tag = build_image_tag(registry, app_name, commit);

		// Assert
		assert_eq!(tag, "myregistry/myapp:abc1234");
	}

	#[rstest]
	fn build_args_correct() {
		// Arrange & Act
		let args = build_docker_args("myregistry/myapp:abc1234", ".");

		// Assert
		assert_eq!(args, vec!["build", "-t", "myregistry/myapp:abc1234", "."]);
	}

	#[rstest]
	fn push_args_correct() {
		// Arrange & Act
		let args = push_docker_args("myregistry/myapp:abc1234");

		// Assert
		assert_eq!(args, vec!["push", "myregistry/myapp:abc1234"]);
	}

	#[rstest]
	fn dockerfile_includes_runtime_stage() {
		// Arrange
		let opts = DockerBuildOptions {
			app_name: "testapp".to_string(),
			port: 3000,
			health_check: "/api/health".to_string(),
			binary_name: None,
		};

		// Act
		let dockerfile = generate_dockerfile(&opts);

		// Assert
		assert!(dockerfile.contains("debian:bookworm-slim"));
		assert!(dockerfile.contains("EXPOSE 3000"));
	}
}
