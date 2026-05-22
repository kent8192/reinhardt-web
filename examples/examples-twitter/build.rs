use cfg_aliases::cfg_aliases;
fn main() {
	let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let examples_dir = std::path::Path::new(&manifest_dir).parent().unwrap();
	let parent_dir = examples_dir.parent().unwrap();
	let parent_cargo = parent_dir.join("Cargo.toml");
	let is_local_dev = parent_cargo.exists()
		&& std::fs::read_to_string(&parent_cargo)
			.map(|c| c.contains("name = \"reinhardt-web\""))
			.unwrap_or(false);
	if is_local_dev {
		println!("cargo:rustc-cfg=with_reinhardt");
		let config_path = examples_dir.join(".cargo/config.toml");
		if !config_path.exists() {
			println!(
				"cargo:warning=Local reinhardt workspace detected but .cargo/config.toml is missing. \
				 Copy the template: cp .cargo/config.local.toml .cargo/config.toml"
			);
		}
	} else {
		println!("cargo:rustc-cfg=with_reinhardt");
	}
	println!("cargo:rerun-if-changed=build.rs");
	println!("cargo:rerun-if-changed=../.cargo/config.toml");
	println!("cargo::rustc-check-cfg=cfg(with_reinhardt)");
	println!("cargo::rustc-check-cfg=cfg(wasm)");
	println!("cargo::rustc-check-cfg=cfg(native)");
	cfg_aliases! {
		wasm : { all(target_family = "wasm", target_os = "unknown") }, native : {
		not(all(target_family = "wasm", target_os = "unknown")) },
	}
}
