fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Set up path to import common.proto from reinhardt-grpc
	let grpc_proto_path = "../crates/reinhardt-grpc/proto";

	// Compile .proto files under tests/proto/ using tonic-prost-build
	tonic_prost_build::configure()
		.build_server(true)
		.build_client(true)
		.compile_protos(
			&["proto/user.proto", "proto/user_events.proto"],
			&["proto", grpc_proto_path],
		)?;

	Ok(())
}
