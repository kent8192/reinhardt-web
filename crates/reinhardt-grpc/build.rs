fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Compile protobuf definitions
	tonic_prost_build::configure()
		.build_server(true)
		.build_client(true)
		.compile_protos(
			&["proto/common.proto", "proto/graphql.proto"],
			&["proto"],
		)?;

	Ok(())
}
