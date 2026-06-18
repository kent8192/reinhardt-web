# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1](https://github.com/kent8192/reinhardt-web/compare/reinhardt-storages@v0.2.0...reinhardt-storages@v0.2.1) - 2026-06-18

### Maintenance

- updated the following local packages: reinhardt-core, reinhardt-conf, reinhardt-providers

## [0.2.0](https://github.com/kent8192/reinhardt-web/releases/tag/reinhardt-storages@v0.2.0) - 2026-06-11

Stable release of `reinhardt-storages` for the Reinhardt 0.2.0 line. This
entry consolidates the 0.2.0 release-candidate series; the original
RC entries remain below as detailed history.

### Migration Notes

- Treat `StorageError` as non-exhaustive and keep wildcard match arms in
  downstream code.
- Move storage configuration to the settings-first `StorageSettings` surface.
- Use the wiremock-backed S3 test setup instead of LocalStack-only assumptions.
- See [`instructions/MIGRATION_0.2.md`](../../instructions/MIGRATION_0.2.md) for the workspace migration checklist.

### Breaking Changes

- *(storages)* [**breaking**] add #[non_exhaustive] to StorageError

### Added

- *(storages)* add reinhardt-storages crate for cloud storage backends
- *(storages)* add Display trait implementation for BackendType
- *(storages)* add cloud storage settings and backends
- local, S3-compatible, GCS, and Azure backend feature surfaces for storage
  integrations.

- *(providers)* add minimal S3 provider client

### Changed

- *(storages)* consolidate test fixture submodules into single file
- *(storages)* consolidate test utility submodules into single file
- storage configuration now follows the explicit nested settings-node model.

### Fixed

- *(settings)* require explicit nested settings nodes
- *(storages)* fix config test compilation and environment access safety
- *(storages)* fix type annotations and content dereference in factory tests
- *(storages)* fix local storage test module imports and async fixture usage
- *(storages)* fix S3 storage test module imports and async fixture usage
- *(storages)* set AWS credentials in S3 test fixture backend creation
- *(storages)* enforce NotFound contract in S3 delete and url methods
- *(storages)* reject path traversal in LocalStorage
- *(storages)* reject Windows drive-letter absolute paths in validate_path
- *(storages)* replace LocalStack with wiremock mock S3 server
- *(storages)* finalize cloud backend verification
- *(storages)* make default_backend feature-aware
- *(storages)* make StorageSettings::default() convertible in non-local builds
- *(storages)* escape #[settings] in deprecation notes for rustdoc
- *(storages)* gate gcs/azure integration tests behind their features

- *(providers)* preserve AWS credential chain
- *(providers)* address CodeRabbit review

### Security

- *(storages)* reject path traversal in LocalStorage
- *(storages)* reject Windows drive-letter absolute paths in validate_path

### Documentation

- *(storages)* update test documentation to reflect wiremock replacement
- *(storages)* document settings-first cloud storage
- *(release)* enforce public API doc coverage

### Maintenance

- updated the following local packages: reinhardt-core, reinhardt-conf

## [0.2.0-rc.5](https://github.com/kent8192/reinhardt-web/compare/reinhardt-storages@v0.2.0-rc.4...reinhardt-storages@v0.2.0-rc.5) - 2026-06-11

### Documentation

- *(release)* enforce public API doc coverage

## [0.2.0-rc.4](https://github.com/kent8192/reinhardt-web/compare/reinhardt-storages@v0.2.0-rc.3...reinhardt-storages@v0.2.0-rc.4) - 2026-06-06

### Fixed

- *(settings)* require explicit nested settings nodes

## [0.2.0-rc.3](https://github.com/kent8192/reinhardt-web/compare/reinhardt-storages@v0.2.0-rc.2...reinhardt-storages@v0.2.0-rc.3) - 2026-06-05

### Maintenance

- updated the following local packages: reinhardt-core, reinhardt-conf

## [0.2.0-rc.2](https://github.com/kent8192/reinhardt-web/releases/tag/reinhardt-storages@v0.2.0-rc.2) - 2026-06-03

### Added

- *(storages)* add reinhardt-storages crate for cloud storage backends
- *(storages)* add Display trait implementation for BackendType
- *(storages)* [**breaking**] add #[non_exhaustive] to StorageError
- *(storages)* add cloud storage settings and backends

### Changed

- *(storages)* consolidate test fixture submodules into single file
- *(storages)* consolidate test utility submodules into single file
- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Documentation

- *(storages)* update test documentation to reflect wiremock replacement
- *(storages)* document settings-first cloud storage

### Fixed

- *(storages)* fix config test compilation and environment access safety
- *(storages)* fix type annotations and content dereference in factory tests
- *(storages)* fix local storage test module imports and async fixture usage
- *(storages)* fix S3 storage test module imports and async fixture usage
- *(storages)* set AWS credentials in S3 test fixture backend creation
- *(storages)* enforce NotFound contract in S3 delete and url methods
- remove develop/0.2.0 content accidentally merged via PR [[#1918](https://github.com/kent8192/reinhardt-web/issues/1918)](https://github.com/kent8192/reinhardt-web/issues/1918)
- resolve CI failures for unconditional reactive auto-wrap PR
- *(ci)* recover develop release-plz prerelease
- *(storages)* reject path traversal in LocalStorage
- *(storages)* reject Windows drive-letter absolute paths in validate_path
- *(ci)* resolve all pre-existing compilation failures on develop/0.2.0
- *(storages)* replace LocalStack with wiremock mock S3 server
- *(storages)* address CodeRabbit review feedback
- *(storages)* finalize cloud backend verification
- apply CodeRabbit auto-fixes
- *(storages)* make default_backend feature-aware
- *(storages)* make StorageSettings::default() convertible in non-local builds
- *(storages)* escape #[settings] in deprecation notes for rustdoc
- *(storages)* gate gcs/azure integration tests behind their features

### Maintenance

- *(storages)* add test dependencies for comprehensive test suite
- *(storages)* add serial_test dev-dependency
- merge develop/0.2.0 and resolve CHANGELOG conflict

### Security

- *(storages)* add symlink containment check and expand test coverage

### Styling

- *(storages)* apply rustfmt to test files
- format files from merge resolution
- *(storages)* wrap long config-error assertion to satisfy rustfmt

### Testing

- *(storages)* add test suite entry point and module organization
- *(storages)* add comprehensive configuration and environment parsing tests
- *(storages)* add factory pattern and backend creation tests
- *(storages)* add test fixtures module organization
- *(storages)* add test utilities module organization
- *(storages)* add comprehensive local storage backend tests
- *(storages)* add comprehensive S3 storage backend tests with LocalStack
- *(storages)* align dangerous path test coverage across all methods
- *(storages)* split Err/Ok arms to avoid Debug on dyn backend
- *(storages)* wait for Azurite readiness log instead of fixed delay

### Added
- Implemented Google Cloud Storage and Azure Blob Storage backends.
- Added `StorageSettings` as the primary `#[settings]` fragment for storage configuration.
- Added `create_storage_from_settings(&StorageSettings)` for settings-first backend construction.
- Added fake-gcs-server and Azurite integration coverage for cloud backend behavior.

### Deprecated
- Deprecated `StorageConfig` and provider-specific `XxxConfig` structs in favor of `StorageSettings`.
- Deprecated `StorageConfig::from_env()` in favor of composed settings loading.

## [0.1.0] - 2026-01-24

### Added
- Initial release of `reinhardt-storages`
- `StorageBackend` trait for unified storage API
- Local file system backend implementation (`LocalStorage`)
- Amazon S3 backend implementation (`S3Storage`)
- Configuration system with environment variable support
- Error types for storage operations
- Factory function for creating storage backends
- Presigned URL generation for S3
- Feature flags for optional backends (`s3`, `local`, `gcs`, `azure`)
- Integration tests for Local storage backend
- Comprehensive documentation and examples

### Features
- Async I/O using Tokio
- Type-safe configuration
- Support for path prefixes
- File metadata operations (size, modified time)

### Notes
- Google Cloud Storage and Azure Blob Storage backends were introduced after this release
