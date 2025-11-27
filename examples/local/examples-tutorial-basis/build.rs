fn main() {
	// Declare custom cfg to avoid warnings in Rust 2024 edition
	println!("cargo::rustc-check-cfg=cfg(with_reinhardt)");
	// Local examples always enable with-reinhardt feature
	println!("cargo:rustc-cfg=with_reinhardt");
}
