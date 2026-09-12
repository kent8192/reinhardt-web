//! Downstream compilation with renamed dependencies and a separate DTO producer crate.
#![cfg(not(target_arch = "wasm32"))]

use std::{fs, path::Path, process::Command};
use tempfile::TempDir;

#[test]
fn renamed_pages_and_facade_support_external_dtos_on_native_and_wasm() {
	run_fixture_checks(false);
}

#[test]
#[ignore = "Requires Chrome and a configured wasm-bindgen-test-runner"]
fn renamed_facade_auth_pages_submit_in_browser() {
	run_fixture_checks(true);
}

fn run_fixture_checks(browser: bool) {
	let pages_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
	let root = pages_dir
		.parent()
		.unwrap()
		.parent()
		.unwrap()
		.canonicalize()
		.unwrap();
	let fixture = pages_dir.join("tests/fixtures/named_client_form_auth/src");
	let workspace = TempDir::new().unwrap();
	let target = TempDir::new().unwrap();
	fs::create_dir_all(workspace.path().join("dto/src")).unwrap();
	fs::create_dir_all(workspace.path().join("app/src")).unwrap();
	fs::write(
		workspace.path().join("Cargo.toml"),
		"[workspace]\nmembers = [\"dto\", \"app\"]\nresolver = \"3\"\n",
	)
	.unwrap();
	fs::copy(
		fixture.join("dto.rs"),
		workspace.path().join("dto/src/dto.rs"),
	)
	.unwrap();
	fs::copy(
		fixture.join("pages.rs"),
		workspace.path().join("app/src/pages.rs"),
	)
	.unwrap();
	fs::copy(
		fixture.join("browser.rs"),
		workspace.path().join("app/src/browser.rs"),
	)
	.unwrap();
	for (source, destination) in [
		("support/client_form_view_browser.rs", "browser_support.rs"),
		("fixtures/form_scope.rs", "form_scope.rs"),
	] {
		let support = fs::read_to_string(pages_dir.join("tests").join(source))
			.unwrap()
			.replace("reinhardt_pages::", "crate::pages_api::");
		fs::write(workspace.path().join("app/src").join(destination), support).unwrap();
	}
	for facade in [false, true] {
		if browser && !facade {
			continue;
		}
		let (dependency, alias) = if facade {
			(
				format!(
					"rh = {{ package = \"reinhardt-web\", path = {:?}, default-features = false, features = [\"pages\"] }}",
					root
				),
				"rh::pages",
			)
		} else {
			(
				format!(
					"pages_dep = {{ package = \"reinhardt-pages\", path = {:?} }}\nreinhardt-core = {{ path = {:?}, default-features = false, features = [\"validators\"] }}",
					pages_dir,
					root.join("crates/reinhardt-core")
				),
				"pages_dep",
			)
		};
		for (directory, name, extra, module) in [
			("dto", "auth-dto", String::new(), "dto"),
			(
				"app",
				"auth-app",
				format!("auth-dto = {{ path = {:?} }}", workspace.path().join("dto")),
				"pages",
			),
		] {
			let browser_dependencies = if directory == "app" && browser {
				"\n[target.'cfg(target_arch = \"wasm32\")'.dev-dependencies]\nwasm-bindgen = \"0.2\"\nwasm-bindgen-test = \"0.3\"\njs-sys = \"0.3\"\ngloo-timers = { version = \"0.3\", features = [\"futures\"] }\nweb-sys = { version = \"0.3\", features = [\"Window\", \"Document\", \"Element\", \"HtmlInputElement\", \"HtmlFormElement\", \"HtmlSelectElement\", \"EventInit\"] }\n"
			} else {
				""
			};
			let manifest = format!(
				"[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n[features]\ntesting = []\n[dependencies]\n{dependency}\nserde = {{ version = \"1\", features = [\"derive\"] }}\nserde_json = \"1\"\n{extra}\n{browser_dependencies}"
			);
			if facade {
				assert!(!manifest.contains("reinhardt-pages"));
				assert!(!manifest.contains("reinhardt-core"));
			}
			fs::write(
				workspace.path().join(directory).join("Cargo.toml"),
				manifest,
			)
			.unwrap();
			let browser_module = if directory == "app" && browser {
				"\n#[cfg(all(test, target_arch = \"wasm32\"))]\nmod browser;\n"
			} else {
				""
			};
			fs::write(
				workspace.path().join(directory).join("src/lib.rs"),
				format!("pub use {alias} as pages_api;\npub mod {module};\n{browser_module}"),
			)
			.unwrap();
		}
		for wasm in [false, true] {
			if browser && !wasm {
				continue;
			}
			let mut command =
				Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
			command
				.arg(if wasm && !browser { "check" } else { "test" })
				.args(["--manifest-path"])
				.arg(workspace.path().join("Cargo.toml"))
				.args(["-p", "auth-app", "--target-dir"])
				.arg(target.path())
				.env_remove("CARGO_TARGET_DIR")
				// Keep intermediate artifacts under the same RAII directory even
				// when global Cargo configuration redirects build output or wrappers.
				.env("CARGO_BUILD_BUILD_DIR", target.path().join("build"))
				.env("RUSTC_WRAPPER", "");
			if wasm {
				command.args(["--target", "wasm32-unknown-unknown"]);
			}
			let output = command.arg("--offline").output().unwrap();
			let output = if output.status.success() || !offline_resolution_failed(&output) {
				output
			} else {
				let mut online = Command::new(command.get_program());
				online.args(command.get_args().filter(|arg| *arg != "--offline"));
				for (key, value) in command.get_envs() {
					if let Some(value) = value {
						online.env(key, value);
					} else {
						online.env_remove(key);
					}
				}
				online.output().unwrap()
			};
			assert!(
				output.status.success(),
				"facade={facade}, wasm={wasm}, browser={browser}\ncommand: {command:?}\n{}\n{}",
				String::from_utf8_lossy(&output.stdout),
				String::from_utf8_lossy(&output.stderr)
			);
		}
	}
}

fn offline_resolution_failed(output: &std::process::Output) -> bool {
	let stderr = String::from_utf8_lossy(&output.stderr);
	stderr.contains("no matching package named")
		|| stderr.contains("failed to download")
		|| stderr.contains("attempting to make an HTTP request, but --offline was specified")
		|| stderr.contains("candidate versions found which didn't match")
}
