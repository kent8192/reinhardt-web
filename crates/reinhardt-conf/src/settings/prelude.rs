//! Prelude module for convenient imports
//!
//! Import this module to get access to the most commonly used types and traits.

#[allow(deprecated)]
pub use super::advanced::{
	AdvancedSettings, CacheSettings, CorsSettings, DatabaseSettings as AdvancedDatabaseSettings,
	EmailSettings, LoggingSettings, MediaSettings, SessionSettings, SettingsError, StaticSettings,
};
pub use super::builder::{BuildError, GetError, MergedSettings, SettingsBuilder};
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
// `JsonFileSource` and `auto_source` are deprecated alongside *.json
// configuration support and will be removed in 0.2.0 (issue #4087). The prelude
// continues to surface them during the deprecation window so existing user code
// keeps compiling unchanged. The `#[allow(deprecated)]` is scoped to this
// re-export only so deprecations added to other `sources` items remain visible.
#[allow(deprecated)]
pub use super::sources::{JsonFileSource, auto_source};
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
