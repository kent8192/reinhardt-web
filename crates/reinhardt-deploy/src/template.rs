use tera::Tera;

use crate::DeployError;
use crate::config::FrontendConfig;

/// Generates Dockerfiles from Tera templates and deployment configuration
pub struct DockerfileGenerator {
	tera: Tera,
}

impl DockerfileGenerator {
	/// Create a new Dockerfile generator with built-in templates
	pub fn new() -> Result<Self, DeployError> {
		let mut tera = Tera::default();

		#[cfg(feature = "wasm-deploy")]
		tera.add_raw_template(
			"Dockerfile.wasm",
			include_str!("../templates/Dockerfile.wasm.tera"),
		)?;

		Ok(Self { tera })
	}

	/// Generate a Dockerfile from the given frontend configuration
	///
	/// Selects the appropriate template based on the configuration:
	/// - WASM-enabled configurations use the multi-stage trunk + nginx template
	///
	/// # Errors
	///
	/// Returns [`DeployError::InvalidConfig`] if the configuration is invalid.
	/// Returns [`DeployError::TemplateRender`] if template rendering fails.
	pub fn generate(&self, config: &FrontendConfig) -> Result<String, DeployError> {
		#[cfg(feature = "wasm-deploy")]
		if config.wasm {
			return self.generate_wasm_dockerfile(config);
		}

		Err(DeployError::InvalidConfig(
			"no suitable Dockerfile template found for the given configuration".to_owned(),
		))
	}

	/// Generate a WASM-specific Dockerfile using the trunk + nginx multi-stage template
	#[cfg(feature = "wasm-deploy")]
	fn generate_wasm_dockerfile(&self, config: &FrontendConfig) -> Result<String, DeployError> {
		use crate::wasm::WasmBuildOptions;

		let wasm_opts = WasmBuildOptions::from_config(config);
		let mut context = tera::Context::new();
		context.insert("app_name", &config.app_name);
		context.insert("source_dir", &config.source_dir);
		context.insert("output_dir", &config.output_dir);
		context.insert("wasm_target", &wasm_opts.target);
		context.insert("rust_version", &wasm_opts.rust_version);
		context.insert("trunk_version", &wasm_opts.trunk_version);
		context.insert("wasm_bindgen_version", &wasm_opts.wasm_bindgen_version);
		context.insert("optimization_level", &wasm_opts.optimization_level);
		context.insert("nginx_port", &wasm_opts.nginx_port);

		Ok(self.tera.render("Dockerfile.wasm", &context)?)
	}
}

impl Default for DockerfileGenerator {
	fn default() -> Self {
		Self::new().expect("built-in templates should always parse successfully")
	}
}
