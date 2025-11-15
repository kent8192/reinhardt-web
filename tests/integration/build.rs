fn main() {
	// Declare custom cfg features to avoid "unexpected cfg" warnings
	println!("cargo:rustc-check-cfg=cfg(feature, values(\"hot-reload\", \"caching\", \"source-maps\", \"image-optimization\", \"graphql\"))");
}
