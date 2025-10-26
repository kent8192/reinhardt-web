//! Prelude module for convenient imports
//!
//! Import this module to get access to the most commonly used types and traits.

pub use crate::advanced::{
    AdvancedSettings, CacheSettings, CorsSettings, DatabaseSettings as AdvancedDatabaseSettings,
    EmailSettings, LoggingSettings, MediaSettings, SessionSettings, SettingsError, StaticSettings,
};
pub use crate::builder::{BuildError, GetError, MergedSettings, SettingsBuilder};
pub use crate::env::{Env, EnvError};
pub use crate::env_loader::{EnvLoader, load_env, load_env_auto, load_env_optional};
pub use crate::env_parser::{
    CacheUrl, DatabaseUrl, parse_bool, parse_cache_url, parse_database_url, parse_dict, parse_list,
};
pub use crate::profile::Profile;
pub use crate::sources::{
    ConfigSource, DefaultSource, DotEnvSource, EnvSource, JsonFileSource, SourceError,
    TomlFileSource, auto_source,
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
    SecretError, SecretManager, SecretMetadata, SecretProvider, SecretResult, SecretString,
    SecretValue, SecretVersion,
    providers::{env::EnvSecretProvider, memory::MemorySecretProvider},
};

#[cfg(feature = "vault")]
pub use crate::secrets::providers::hashicorp::{VaultConfig, VaultSecretProvider};
