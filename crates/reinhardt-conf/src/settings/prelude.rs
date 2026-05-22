//! Prelude module for convenient imports
//!
//! Import this module to get access to the most commonly used types and traits.

// `AdvancedSettings` was removed in 0.2.0 (Issue #4520); the individual
// fragment types remain.
pub use super::advanced::{
	CacheSettings, CorsSettings, DatabaseSettings as AdvancedDatabaseSettings, EmailSettings,
	LoggingSettings, MediaSettings, SessionSettings, SettingsError, StaticSettings,
};
pub use super::builder::{BuildError, GetError, MergeStrategy, MergedSettings, SettingsBuilder};
pub use super::env::{Env, EnvError};
pub use super::env_loader::{EnvLoader, load_env, load_env_auto, load_env_optional};
pub use super::env_parser::{
	CacheUrl, DatabaseUrl, parse_bool, parse_cache_url, parse_database_url, parse_dict, parse_list,
};
pub use super::interpolation::InterpolationError;
pub use super::profile::Profile;
pub use super::sources::{
	ConfigSource, DefaultSource, DotEnvSource, EnvSource, HighPriorityEnvSource,
	LowPriorityEnvSource, SourceError, TomlFileSource,
};
pub use super::typed_deserializer::CoercionError;
// `JsonFileSource` and `auto_source` were removed in 0.2.0 alongside *.json
// configuration support (Issue #4087). TOML is the canonical format —
// construct `TomlFileSource` directly.
pub use super::testing::{SettingsOverride, SettingsOverrideGuard};
pub use super::validation::{
	ChoiceValidator, PatternValidator, RangeValidator, RequiredValidator, SecurityValidator,
	SettingsValidator, ValidationError, ValidationResult, Validator,
};
#[allow(deprecated)]
pub use super::{DatabaseConfig, MiddlewareConfig, Settings, TemplateConfig};

// Dynamic settings (async feature)
#[cfg(feature = "async")]
pub use super::backends::{memory::MemoryBackend, *};

#[cfg(feature = "async")]
pub use super::dynamic::{DynamicBackend, DynamicError, DynamicResult, DynamicSettings};

#[cfg(feature = "async")]
pub use super::secrets::{
	SecretError, SecretManager, SecretMetadata, SecretProvider, SecretResult, SecretString,
	SecretValue, SecretVersion,
	providers::{env::EnvSecretProvider, memory::MemorySecretProvider},
};

#[cfg(feature = "vault")]
pub use super::secrets::providers::hashicorp::{VaultConfig, VaultSecretProvider};
