# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-alpha.12](https://github.com/kent8192/reinhardt-web/compare/reinhardt-rest@v0.1.0-alpha.11...reinhardt-rest@v0.1.0-alpha.12) - 2026-02-14

### Maintenance

- updated the following local packages: reinhardt-query, reinhardt-db, reinhardt-db, reinhardt-auth, reinhardt-auth

## [0.1.0-alpha.11](https://github.com/kent8192/reinhardt-web/compare/reinhardt-rest@v0.1.0-alpha.10...reinhardt-rest@v0.1.0-alpha.11) - 2026-02-14

### Maintenance

- updated the following local packages: reinhardt-query, reinhardt-db, reinhardt-db, reinhardt-auth, reinhardt-auth

## [0.1.0-alpha.10](https://github.com/kent8192/reinhardt-web/compare/reinhardt-rest@v0.1.0-alpha.9...reinhardt-rest@v0.1.0-alpha.10) - 2026-02-14

### Changed

- *(rest)* remove unused sea-orm dependency

## [0.1.0-alpha.9](https://github.com/kent8192/reinhardt-web/compare/reinhardt-rest@v0.1.0-alpha.8...reinhardt-rest@v0.1.0-alpha.9) - 2026-02-12

### Maintenance

- updated the following local packages: reinhardt-core, reinhardt-core, reinhardt-utils, reinhardt-db, reinhardt-db, reinhardt-auth, reinhardt-auth, reinhardt-http

## [0.1.0-alpha.8](https://github.com/kent8192/reinhardt-web/compare/reinhardt-rest@v0.1.0-alpha.7...reinhardt-rest@v0.1.0-alpha.8) - 2026-02-10

### Fixed

- *(rest)* move tests to integration crate to break circular publish chain
- *(release)* revert unpublished crate versions to pre-release state

### Reverted

- undo PR [[#219](https://github.com/kent8192/reinhardt-web/issues/219)](https://github.com/kent8192/reinhardt-web/issues/219) version bumps for unpublished crates

### Styling

- apply formatting to migrated test files and modified source files

## [0.1.0-alpha.7](https://github.com/kent8192/reinhardt-web/compare/reinhardt-rest@v0.1.0-alpha.6...reinhardt-rest@v0.1.0-alpha.7) - 2026-02-06

### Fixed

- remove reinhardt-urls from doc example to avoid circular dependency
- break circular dependency between reinhardt-openapi-macros and reinhardt-rest
- remove unused dev-dependencies from reinhardt-rest

### Other

- Revert "Merge pull request #202 from kent8192/release-plz-2026-02-06T13-32-57Z"
- release

## [0.1.0-alpha.6](https://github.com/kent8192/reinhardt-web/compare/reinhardt-rest@v0.1.0-alpha.5...reinhardt-rest@v0.1.0-alpha.6) - 2026-02-03

### Other

- updated the following local packages: reinhardt-core, reinhardt-core, reinhardt-pages, reinhardt-http, reinhardt-utils, reinhardt-server, reinhardt-db, reinhardt-db, reinhardt-auth, reinhardt-auth

## [0.1.0-alpha.5](https://github.com/kent8192/reinhardt-web/compare/reinhardt-rest@v0.1.0-alpha.4...reinhardt-rest@v0.1.0-alpha.5) - 2026-02-03

### Other

- merge main into chore/release-plz-migration
- add release-plz migration markers to CHANGELOGs

<!-- release-plz-separator -->
<!-- Entries below this line were created before release-plz adoption -->

## [0.1.0-alpha.4] - 2026-01-30

### Changed

- Version bump for publish workflow correction (no functional changes)

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

