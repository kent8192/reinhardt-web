#![deny(unexpected_cfgs)]

use reinhardt_di::{injectable, injectable_key};

#[injectable_key]
pub struct NativeConfigKey;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
#[derive(Clone)]
pub struct NativeDependency {
	value: &'static str,
}

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
#[derive(Clone)]
pub struct NativeConfig {
	value: &'static str,
}

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
#[reinhardt_di::async_trait::async_trait]
impl reinhardt_di::Injectable for NativeDependency {
	async fn inject(_ctx: &reinhardt_di::InjectionContext) -> reinhardt_di::DiResult<Self> {
		Ok(Self { value: "native" })
	}
}

#[injectable(scope = "transient")]
pub async fn native_config_provider(
	#[inject] dependency: NativeDependency,
) -> reinhardt_di::FactoryOutput<NativeConfigKey, NativeConfig> {
	reinhardt_di::FactoryOutput::new(NativeConfig {
		value: dependency.value,
	})
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub async fn references_generated_stub() {
	native_config_provider().await;
}

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
pub fn native_value(config: NativeConfig) -> &'static str {
	config.value
}
