#![cfg(feature = "tls")]

use reinhardt_grpc::{GrpcClientTlsConfig, GrpcServerTlsConfig, tonic};

#[test]
fn transport_preserves_application_selected_provider() {
	rustls::crypto::ring::default_provider()
		.install_default()
		.unwrap();
	let selected = rustls::crypto::CryptoProvider::get_default()
		.unwrap()
		.clone();
	let rcgen::CertifiedKey { cert, signing_key } =
		rcgen::generate_simple_self_signed(vec!["auth.internal".into()]).unwrap();
	let cert = cert.pem();
	let key = signing_key.serialize_pem();
	assert!(
		GrpcServerTlsConfig::new(&cert, &key, &cert)
			.configure(tonic::transport::Server::builder())
			.is_ok()
	);
	assert!(
		GrpcClientTlsConfig::new(&cert, &cert, &key, "auth.internal")
			.endpoint("https://127.0.0.1:50051")
			.is_ok()
	);
	assert!(std::sync::Arc::ptr_eq(
		&selected,
		rustls::crypto::CryptoProvider::get_default().unwrap()
	));
}
