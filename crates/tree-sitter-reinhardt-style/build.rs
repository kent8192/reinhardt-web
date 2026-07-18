fn main() {
	println!("cargo:rerun-if-changed=src/parser.c");
	println!("cargo:rerun-if-changed=src/tree_sitter/parser.h");

	cc::Build::new()
		.file("src/parser.c")
		.include("src")
		.compile("tree-sitter-reinhardt-style");
}
