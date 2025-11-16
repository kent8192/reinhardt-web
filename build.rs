fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Compile proto files for gRPC integration tests
	tonic_prost_build::configure()
		.build_server(true)
		.build_client(true)
		.compile_protos(
			&[
				"tests/proto/common.proto",
				"tests/proto/user.proto",
				"tests/proto/user_events.proto",
			],
			&["tests/proto"],
		)?;

	Ok(())
}
