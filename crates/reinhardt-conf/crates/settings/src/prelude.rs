//! Prelude module for convenient imports
//!
//! Import this module to get access to the most commonly used types and traits.

pub use crate::advanced::{
    AdvancedSettings, CacheSettings, CorsSettings, DatabaseSettings as AdvancedDatabaseSettings,
    EmailSettings, LoggingSettings, MediaSettings, SessionSettings, SettingsError, StaticSettings,
};
pub use crate::builder::{BuildError, GetError, MergedSettings, SettingsBuilder};
pub use crate::env::{Env, EnvError};
pub use crate::env_loader::{load_env, load_env_auto, load_env_optional, EnvLoader};
pub use crate::env_parser::{
    parse_bool, parse_cache_url, parse_database_url, parse_dict, parse_list, CacheUrl, DatabaseUrl,
};
pub use crate::profile::Profile;
pub use crate::sources::{
    auto_source, ConfigSource, DefaultSource, DotEnvSource, EnvSource, JsonFileSource, SourceError,
    TomlFileSource,
};
pub use crate::validation::{
    ChoiceValidator, PatternValidator, RangeValidator, RequiredValidator, SecurityValidator,
    SettingsValidator, ValidationError, ValidationResult, Validator,
};
pub use crate::{DatabaseConfig, MiddlewareConfig, Settings, TemplateConfig};

// Dynamic settings (async feature)
#[cfg(feature = "async")]
pub use crate::backends::{memory::MemoryBackend, *};

#[cfg(feature = "async")]
pub use crate::dynamic::{DynamicBackend, DynamicError, DynamicResult, DynamicSettings};

#[cfg(feature = "async")]
pub use crate::secrets::{
    providers::{env::EnvSecretProvider, memory::MemorySecretProvider},
    SecretError, SecretManager, SecretMetadata, SecretProvider, SecretResult, SecretString,
    SecretValue, SecretVersion,
};

#[cfg(feature = "vault")]
pub use crate::secrets::providers::hashicorp::{VaultConfig, VaultSecretProvider};
