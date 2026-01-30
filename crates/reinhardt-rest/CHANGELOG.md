# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-alpha.3] - 2026-01-30

### Changed

- Moved `OpenApiRouter` to `reinhardt-openapi` crate to resolve circular dependency
- Re-exported `generate_openapi_schema` from `endpoints` module for backward compatibility

### Removed

- Removed `openapi/router_wrapper.rs` (moved to `reinhardt-openapi` crate)

### Notes

- See [Issue #23](https://github.com/kent8192/reinhardt-web/issues/23) for circular dependency resolution details

## [0.1.0-alpha.2] - 2026-01-23

### Fixed

- Embed branding assets within crate for crates.io compatibility

## [0.1.0-alpha.1] - 2026-01-23

### Added

- Initial release with RESTful API framework with serializers, viewsets, and browsable API interface
