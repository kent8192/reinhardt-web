fn main() {
	println!("cargo::rustc-check-cfg=cfg(server)");
	if std::env::var_os("CARGO_FEATURE_SERVER").is_some() {
		println!("cargo::rustc-cfg=server");
	}
}
