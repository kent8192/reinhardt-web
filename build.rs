fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Skip gRPC proto compilation when building for WASM
	// CARGO_CFG_TARGET_ARCH is set by Cargo during cross-compilation
	let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
	if target_arch == "wasm32" {
		return Ok(());
	}

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
