#![cfg(feature = "tls")]

use reinhardt_grpc::GrpcClientTlsConfig;

// A separate integration-test process ensures a server or another test cannot
// initialize the process-wide provider before this client-first configuration.
#[test]
fn client_first_configuration_selects_provider_with_both_backends_enabled() {
	assert!(rustls::crypto::CryptoProvider::get_default().is_none());
	let rcgen::CertifiedKey { cert, signing_key } =
		rcgen::generate_simple_self_signed(vec!["auth.internal".into()]).unwrap();
	let cert = cert.pem();
	let tls = GrpcClientTlsConfig::new(&cert, &cert, signing_key.serialize_pem(), "auth.internal");
	assert!(tls.endpoint("https://127.0.0.1:50051").is_ok());
	assert!(rustls::crypto::CryptoProvider::get_default().is_some());
}
