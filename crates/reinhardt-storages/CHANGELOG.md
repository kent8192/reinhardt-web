# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.7](https://github.com/kent8192/reinhardt-web/compare/reinhardt-storages@v0.2.6...reinhardt-storages@v0.2.7) - 2026-06-28

### Maintenance

- updated the following local packages: reinhardt-core, reinhardt-conf, reinhardt-providers

## [0.2.6](https://github.com/kent8192/reinhardt-web/compare/reinhardt-storages@v0.2.5...reinhardt-storages@v0.2.6) - 2026-06-27

### Maintenance

- merge main into develop/0.3.0

## [0.2.5](https://github.com/kent8192/reinhardt-web/compare/reinhardt-storages@v0.2.4...reinhardt-storages@v0.2.5) - 2026-06-26

### Maintenance

- updated the following local packages: reinhardt-core, reinhardt-conf, reinhardt-providers

## [0.2.4](https://github.com/kent8192/reinhardt-web/compare/reinhardt-storages@v0.2.3...reinhardt-storages@v0.2.4) - 2026-06-24

### Maintenance

- updated the following local packages: reinhardt-core, reinhardt-conf, reinhardt-providers

## [0.2.3](https://github.com/kent8192/reinhardt-web/compare/reinhardt-storages@v0.2.2...reinhardt-storages@v0.2.3) - 2026-06-23

### Maintenance

- updated the following local packages: reinhardt-core, reinhardt-conf, reinhardt-providers

## [0.2.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-storages@v0.2.1...reinhardt-storages@v0.2.2) - 2026-06-19

### Maintenance

- updated the following local packages: reinhardt-core, reinhardt-conf, reinhardt-providers

## [0.2.1](https://github.com/kent8192/reinhardt-web/compare/reinhardt-storages@v0.2.0...reinhardt-storages@v0.2.1) - 2026-06-18

### Maintenance

- updated the following local packages: reinhardt-core, reinhardt-conf, reinhardt-providers

## [0.2.0](https://github.com/kent8192/reinhardt-web/releases/tag/reinhardt-storages@v0.2.0) - 2026-06-11

Stable release of `reinhardt-storages` for the Reinhardt 0.2.0 line. This
entry consolidates the 0.2.0 release-candidate series into one
stable release section.

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
