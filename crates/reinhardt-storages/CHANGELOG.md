# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.1](https://github.com/kent8192/reinhardt-web/releases/tag/reinhardt-storages@v0.2.0-rc.1) - 2026-05-22

### Added

- *(storages)* add reinhardt-storages crate for cloud storage backends
- *(storages)* add Display trait implementation for BackendType

### Changed

- *(storages)* consolidate test fixture submodules into single file
- *(storages)* consolidate test utility submodules into single file
- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Fixed

- *(storages)* fix config test compilation and environment access safety
- *(storages)* fix type annotations and content dereference in factory tests
- *(storages)* fix local storage test module imports and async fixture usage
- *(storages)* fix S3 storage test module imports and async fixture usage
- *(storages)* set AWS credentials in S3 test fixture backend creation
- *(storages)* enforce NotFound contract in S3 delete and url methods
- remove develop/0.2.0 content accidentally merged via PR [[#1918](https://github.com/kent8192/reinhardt-web/issues/1918)](https://github.com/kent8192/reinhardt-web/issues/1918)
- *(storages)* add #[allow(clippy::todo)] to unimplemented Phase 2 backends

### Maintenance

- *(storages)* add test dependencies for comprehensive test suite
- *(storages)* add serial_test dev-dependency

### Styling

- *(storages)* apply rustfmt to test files

### Testing

- *(storages)* add test suite entry point and module organization
- *(storages)* add comprehensive configuration and environment parsing tests
- *(storages)* add factory pattern and backend creation tests
- *(storages)* add test fixtures module organization
- *(storages)* add test utilities module organization
- *(storages)* add comprehensive local storage backend tests
- *(storages)* add comprehensive S3 storage backend tests with LocalStack

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
- Google Cloud Storage and Azure Blob Storage backends are planned for future releases
