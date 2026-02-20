fn main() -> Result<(), Box<dyn std::error::Error>> {
	let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);

	// Compile protobuf definitions and generate file descriptor set for reflection
	tonic_prost_build::configure()
		.build_server(true)
		.build_client(true)
		.file_descriptor_set_path(out_dir.join("reinhardt_descriptor.bin"))
		.compile_protos(&["proto/common.proto", "proto/graphql.proto"], &["proto"])?;

	Ok(())
}
