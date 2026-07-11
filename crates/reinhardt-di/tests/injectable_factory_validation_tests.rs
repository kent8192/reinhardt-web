#![cfg(feature = "macros")]
// Exercises the deprecated `FactoryOutput` alias to verify it still resolves
// to the same registry entry as `KeyedFactoryOutput`.
#![allow(deprecated)]

use reinhardt_di::{
	FactoryOutput, InjectableKey, KeyedFactoryOutput, RegistryValidator, SelfKey, global_registry,
	injectable, injectable_key,
};
use serial_test::serial;
use std::any::{TypeId, type_name};
use std::sync::Arc;

#[derive(Clone)]
struct MacroSelfConfig;

#[injectable(scope = "singleton")]
async fn macro_self_config() -> MacroSelfConfig {
	MacroSelfConfig
}

#[derive(Clone)]
struct MacroKeyedConfig;

#[injectable_key]
struct MacroKeyedConfigKey;

#[injectable(scope = "singleton")]
async fn macro_keyed_config() -> KeyedFactoryOutput<MacroKeyedConfigKey, MacroKeyedConfig> {
	KeyedFactoryOutput::new(MacroKeyedConfig)
}

#[derive(Clone)]
struct MacroAliasConfig;

struct MacroAliasConfigKey;

impl InjectableKey for MacroAliasConfigKey {}

#[injectable(scope = "singleton")]
async fn macro_alias_config() -> FactoryOutput<MacroAliasConfigKey, MacroAliasConfig> {
	FactoryOutput::new(MacroAliasConfig)
}

#[derive(Clone)]
struct MacroAliasedOutputConfig;

#[injectable_key]
struct MacroAliasedOutputConfigKey;

type MacroAliasedOutput = KeyedFactoryOutput<MacroAliasedOutputConfigKey, MacroAliasedOutputConfig>;

#[injectable(scope = "singleton")]
async fn macro_aliased_output_config() -> MacroAliasedOutput {
	KeyedFactoryOutput::new(MacroAliasedOutputConfig)
}

#[derive(Clone)]
struct MacroAliasedFactoryOutputConfig;

#[injectable_key]
struct MacroAliasedFactoryOutputConfigKey;

type MacroAliasedFactoryOutput =
	FactoryOutput<MacroAliasedFactoryOutputConfigKey, MacroAliasedFactoryOutputConfig>;

#[injectable(scope = "singleton")]
async fn macro_aliased_factory_output_config() -> MacroAliasedFactoryOutput {
	FactoryOutput::new(MacroAliasedFactoryOutputConfig)
}

#[serial(di_registry)]
#[test]
fn injectable_providers_register_value_qualified_names_for_validation() {
	let registry = Arc::clone(global_registry());

	let self_keyed_type =
		TypeId::of::<KeyedFactoryOutput<SelfKey<MacroSelfConfig>, MacroSelfConfig>>();
	assert!(
		registry.is_registered::<KeyedFactoryOutput<SelfKey<MacroSelfConfig>, MacroSelfConfig>>()
	);
	assert_eq!(
		registry.get_qualified_type_name(&self_keyed_type),
		Some(type_name::<MacroSelfConfig>())
	);

	let keyed_type = TypeId::of::<KeyedFactoryOutput<MacroKeyedConfigKey, MacroKeyedConfig>>();
	assert!(registry.is_registered::<KeyedFactoryOutput<MacroKeyedConfigKey, MacroKeyedConfig>>());
	assert_eq!(
		registry.get_qualified_type_name(&keyed_type),
		Some(type_name::<MacroKeyedConfig>())
	);

	let alias_type = TypeId::of::<KeyedFactoryOutput<MacroAliasConfigKey, MacroAliasConfig>>();
	assert!(registry.is_registered::<KeyedFactoryOutput<MacroAliasConfigKey, MacroAliasConfig>>());
	assert_eq!(
		registry.get_qualified_type_name(&alias_type),
		Some(type_name::<MacroAliasConfig>())
	);

	let aliased_output_type =
		TypeId::of::<KeyedFactoryOutput<SelfKey<MacroAliasedOutput>, MacroAliasedOutput>>();
	assert!(
		registry
			.is_registered::<KeyedFactoryOutput<SelfKey<MacroAliasedOutput>, MacroAliasedOutput>>()
	);
	assert_eq!(
		registry.get_qualified_type_name(&aliased_output_type),
		Some(type_name::<MacroAliasedOutput>())
	);

	let aliased_factory_output_type = TypeId::of::<
		KeyedFactoryOutput<SelfKey<MacroAliasedFactoryOutput>, MacroAliasedFactoryOutput>,
	>();
	assert!(registry.is_registered::<
		KeyedFactoryOutput<SelfKey<MacroAliasedFactoryOutput>, MacroAliasedFactoryOutput>,
	>());
	assert_eq!(
		registry.get_qualified_type_name(&aliased_factory_output_type),
		Some(type_name::<MacroAliasedFactoryOutput>())
	);

	let result = RegistryValidator::new(registry).validate();
	assert!(result.is_ok(), "validation errors: {result:#?}");
}
