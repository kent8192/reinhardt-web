fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Set up path to import common.proto from reinhardt-grpc
	let grpc_proto_path = "../crates/reinhardt-grpc/proto";

	// Compile .proto files under tests/proto/
	let file_descriptors = protox::compile(
		&["proto/user.proto", "proto/user_events.proto"],
		&["proto", grpc_proto_path],
	)?;

	tonic_build::configure()
		.build_server(true)
		.build_client(true)
		.compile_fds(file_descriptors)?;

	Ok(())
}
