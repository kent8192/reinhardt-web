use std::env;
use std::path::PathBuf;

fn main() {
	// CARGO_MANIFEST_DIRから4階層遡ってワークスペースルートを取得
	// crates/reinhardt-rest/crates/openapi -> crates/reinhardt-rest/crates
	//                                       -> crates/reinhardt-rest
	//                                       -> crates
	//                                       -> (workspace root)
	let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
	let workspace_root = PathBuf::from(manifest_dir)
		.parent() // crates/reinhardt-rest/crates
		.and_then(|p| p.parent()) // crates/reinhardt-rest
		.and_then(|p| p.parent()) // crates
		.and_then(|p| p.parent()) // workspace root
		.expect("Failed to determine workspace root")
		.to_path_buf();

	// ワークスペースルートを環境変数として設定
	println!(
		"cargo:rustc-env=WORKSPACE_ROOT={}",
		workspace_root.display()
	);

	// branding/thirdparty ディレクトリの変更を検知して再ビルド
	let branding_dir = workspace_root.join("branding/thirdparty");
	println!("cargo:rerun-if-changed={}", branding_dir.display());
}
