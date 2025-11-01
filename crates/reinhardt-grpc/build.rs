fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Compile .proto files using protox (pure Rust implementation)
	// Common types and GraphQL types provided by the framework
	let file_descriptors =
		protox::compile(["proto/common.proto", "proto/graphql.proto"], ["proto"])?;

	tonic_build::configure()
		.build_server(true)
		.build_client(true)
		.compile_fds(file_descriptors)?;

	Ok(())
}
