#![cfg(feature = "wasm-deploy")]

use reinhardt_deploy::DockerfileGenerator;
use reinhardt_deploy::config::FrontendConfig;
use reinhardt_deploy::wasm::{WasmBuildOptions, detect_wasm_project};
use rstest::rstest;

#[rstest]
fn wasm_build_options_has_sensible_defaults() {
	// Arrange / Act
	let opts = WasmBuildOptions::default();

	// Assert
	assert_eq!(opts.target, "wasm32-unknown-unknown");
	assert_eq!(opts.rust_version, "1.85");
	assert_eq!(opts.trunk_version, "0.21.12");
	assert_eq!(opts.wasm_bindgen_version, "0.2.100");
	assert_eq!(opts.optimization_level, "z");
	assert_eq!(opts.nginx_port, 80);
}

#[rstest]
fn wasm_build_options_builder_methods() {
	// Arrange / Act
	let opts = WasmBuildOptions::default()
		.with_rust_version("1.86")
		.with_trunk_version("0.22.0")
		.with_wasm_bindgen_version("0.2.101")
		.with_optimization_level("s")
		.with_nginx_port(8080);

	// Assert
	assert_eq!(opts.rust_version, "1.86");
	assert_eq!(opts.trunk_version, "0.22.0");
	assert_eq!(opts.wasm_bindgen_version, "0.2.101");
	assert_eq!(opts.optimization_level, "s");
	assert_eq!(opts.nginx_port, 8080);
}

#[rstest]
fn wasm_build_options_from_config_uses_target() {
	// Arrange
	let config = FrontendConfig::new("my-app").with_wasm_target("wasm32-wasip1");

	// Act
	let opts = WasmBuildOptions::from_config(&config);

	// Assert
	assert_eq!(opts.target, "wasm32-wasip1");
}

#[rstest]
fn wasm_build_options_from_config_defaults_target() {
	// Arrange
	let config = FrontendConfig::new("my-app").with_wasm();

	// Act
	let opts = WasmBuildOptions::from_config(&config);

	// Assert
	assert_eq!(opts.target, "wasm32-unknown-unknown");
}

#[rstest]
fn frontend_config_with_wasm_sets_default_target() {
	// Arrange / Act
	let config = FrontendConfig::new("test-app").with_wasm();

	// Assert
	assert!(config.wasm);
	assert_eq!(
		config.wasm_target.as_deref(),
		Some("wasm32-unknown-unknown")
	);
}

#[rstest]
fn frontend_config_defaults_without_wasm() {
	// Arrange / Act
	let config = FrontendConfig::new("test-app");

	// Assert
	assert!(!config.wasm);
	assert!(config.wasm_target.is_none());
	assert_eq!(config.source_dir, "frontend");
	assert_eq!(config.output_dir, "dist");
}

#[rstest]
#[case("wasm-bindgen = \"0.2\"", true)]
#[case("web-sys = \"0.3\"", true)]
#[case("js-sys = \"0.3\"", true)]
#[case("reinhardt-pages", true)]
#[case("target_arch = \"wasm32\"", true)]
#[case("tokio = \"1\"", false)]
#[case("serde = \"1\"", false)]
fn detect_wasm_project_identifies_wasm_dependencies(#[case] content: &str, #[case] expected: bool) {
	// Arrange
	let cargo_toml = format!("[dependencies]\n{content}");

	// Act
	let result = detect_wasm_project(&cargo_toml);

	// Assert
	assert_eq!(result, expected);
}

#[rstest]
fn generate_wasm_dockerfile_contains_expected_stages() {
	// Arrange
	let generator = DockerfileGenerator::new().unwrap();
	let config = FrontendConfig::new("my-wasm-app")
		.with_source_dir("client")
		.with_output_dir("build")
		.with_wasm();

	// Act
	let dockerfile = generator.generate(&config).unwrap();

	// Assert
	assert!(dockerfile.contains("FROM rust:"));
	assert!(dockerfile.contains("AS toolchain"));
	assert!(dockerfile.contains("AS builder"));
	assert!(dockerfile.contains("FROM nginx:alpine AS runtime"));
	assert!(dockerfile.contains("rustup target add wasm32-unknown-unknown"));
	assert!(dockerfile.contains("cargo install trunk"));
	assert!(dockerfile.contains("cargo install wasm-bindgen-cli"));
	assert!(dockerfile.contains("trunk build --release"));
	assert!(dockerfile.contains("/app/client"));
	assert!(dockerfile.contains("/app/build"));
	assert!(dockerfile.contains("my-wasm-app"));
	assert!(dockerfile.contains("try_files"));
}

#[rstest]
fn generate_wasm_dockerfile_respects_custom_options() {
	// Arrange
	let generator = DockerfileGenerator::new().unwrap();
	let config = FrontendConfig::new("custom-app").with_wasm_target("wasm32-wasip1");

	// Act
	let dockerfile = generator.generate(&config).unwrap();

	// Assert
	assert!(dockerfile.contains("wasm32-wasip1"));
	assert!(dockerfile.contains("custom-app"));
}
