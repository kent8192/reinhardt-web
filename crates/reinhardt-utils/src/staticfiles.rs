//! Static files and production utilities for Reinhardt

pub mod caching;
pub mod cdn;
pub mod checks;
pub mod dependency_resolver;
pub mod handler;
pub mod health;
pub mod media;
pub mod metrics;
pub mod middleware;
pub mod path_resolver;
pub mod processing;
pub mod storage;
pub mod template_integration;

#[cfg(feature = "dev-server")]
pub mod dev_server;

pub use caching::{CacheControlConfig, CacheControlMiddleware, CacheDirective, CachePolicy};
pub use cdn::{CdnConfig, CdnInvalidationRequest, CdnProvider, CdnPurgeHelper, CdnUrlGenerator};
pub use checks::{CheckLevel, CheckMessage, check_static_files_config};
pub use dependency_resolver::DependencyGraph;
pub use handler::{StaticError, StaticFile, StaticFileHandler, StaticResult};
pub use health::{
	CacheHealthCheck, DatabaseHealthCheck, HealthCheck, HealthCheckManager, HealthCheckResult,
	HealthReport, HealthStatus,
};
pub use media::{HasMedia, Media};
pub use metrics::{Metric, MetricsCollector, RequestMetrics, RequestTimer};
pub use middleware::{StaticFilesConfig as StaticMiddlewareConfig, StaticFilesMiddleware};
pub use path_resolver::PathResolver;
pub use storage::{
	FileSystemStorage, HashedFileStorage, Manifest, ManifestStaticFilesStorage, ManifestVersion,
	MemoryStorage, StaticFilesConfig, StaticFilesFinder, Storage, StorageRegistry,
};

#[cfg(feature = "s3")]
pub use storage::{S3Config, S3Storage};

#[cfg(feature = "azure")]
pub use storage::{AzureBlobConfig, AzureBlobStorage};

#[cfg(feature = "gcs")]
pub use storage::{GcsConfig, GcsStorage};

pub use template_integration::TemplateStaticConfig;

pub use processing::{ProcessingConfig, ProcessingPipeline, ProcessingResult, Processor};

#[cfg(feature = "dev-server")]
pub use dev_server::{
	AutoReload, AutoReloadBuilder, DevServerConfig, DevelopmentErrorHandler, FileWatcher,
	FileWatcherBuilder, ReloadEvent, WatchEvent,
};
