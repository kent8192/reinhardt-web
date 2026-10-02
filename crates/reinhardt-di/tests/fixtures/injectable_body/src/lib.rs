#![deny(unused_braces)]

use reinhardt_di::{
	DiResult, FactoryOutput, Injectable, InjectionContext, injectable, injectable_key,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Service {
	pub value: u32,
}

#[derive(Clone)]
pub struct Dependency {
	value: u32,
}

#[reinhardt_di::async_trait::async_trait]
impl Injectable for Dependency {
	async fn inject(_ctx: &InjectionContext) -> DiResult<Self> {
		Ok(Self { value: 7 })
	}
}

#[injectable_key]
#[derive(Clone)]
pub struct PlainKey;

#[injectable(scope = "request")]
// Keep the body on one line to exercise rustc's unused_braces suggestions.
#[rustfmt::skip]
pub async fn plain_provider() -> FactoryOutput<PlainKey, Service> { FactoryOutput::new(Service { value: 3 }) }

#[injectable_key]
#[derive(Clone)]
pub struct InjectedKey;

#[injectable(scope = "request")]
#[rustfmt::skip]
pub async fn injected_provider(#[inject] dependency: Dependency) -> FactoryOutput<InjectedKey, Service> { FactoryOutput::new(Service { value: dependency.value }) }

#[injectable_key]
#[derive(Clone)]
pub struct AwaitedKey;

#[injectable(scope = "transient")]
#[rustfmt::skip]
pub async fn awaited_provider() -> FactoryOutput<AwaitedKey, Service> { FactoryOutput::new(async { Service { value: 11 } }.await) }

#[injectable_key]
#[derive(Clone)]
pub struct StatementsKey;

#[injectable(scope = "singleton")]
pub async fn statements_provider(
	#[inject] dependency: Dependency,
) -> FactoryOutput<StatementsKey, Service> {
	let value = dependency.value * 2;
	if value == 14 {
		return FactoryOutput::new(Service { value });
	}
	FactoryOutput::new(Service { value: 0 })
}

// Exercise the deprecated compatibility entrypoint, which shares the expansion.
#[allow(deprecated)]
pub mod legacy {
	use super::*;

	#[injectable_key]
	#[derive(Clone)]
	pub struct LegacyKey;

	#[reinhardt_di::injectable_factory(scope = "request")]
	#[rustfmt::skip]
	pub async fn legacy_provider() -> FactoryOutput<LegacyKey, Service> { FactoryOutput::new(Service { value: 17 }) }
}
