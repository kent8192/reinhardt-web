#![cfg(feature = "macros")]

use reinhardt_di::{
	InjectionContext, KeyedFactoryOutput, RegistryValidator, ValidationErrorKind, global_registry,
	injectable, injectable_key,
};
use std::sync::Arc;

#[injectable_key]
struct FrameworkContextKey;

type AliasedFrameworkContextOutput = KeyedFactoryOutput<FrameworkContextKey, InjectionContext>;

#[injectable(scope = "singleton")]
async fn aliased_framework_context_provider() -> AliasedFrameworkContextOutput {
	panic!("the provider is only registered for validation")
}

#[test]
fn aliased_framework_output_remains_a_validation_error() {
	let registry = Arc::clone(global_registry());
	let errors = RegistryValidator::new(registry)
		.validate()
		.expect_err("framework-owned alias output must be rejected");

	assert!(
		errors
			.iter()
			.any(|error| error.kind == ValidationErrorKind::FrameworkTypeOverride)
	);
}
