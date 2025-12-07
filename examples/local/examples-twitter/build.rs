// build.rs
fn main() {
	// Always enable tests for local development
	println!("cargo:rustc-cfg=with_reinhardt");
}
