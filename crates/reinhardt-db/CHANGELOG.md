# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-alpha.14](https://github.com/kent8192/reinhardt-web/compare/reinhardt-db@v0.1.0-alpha.13...reinhardt-db@v0.1.0-alpha.14) - 2026-02-19

### Fixed

- *(security)* use parameterized queries and escape identifiers to prevent SQL injection

### Styling

- apply code formatting to security fix files

## [0.1.0-alpha.12](https://github.com/kent8192/reinhardt-web/compare/reinhardt-db@v0.1.0-alpha.11...reinhardt-db@v0.1.0-alpha.12) - 2026-02-15

### Maintenance

- updated the following local packages: reinhardt-query, reinhardt-conf

## [0.1.0-alpha.11](https://github.com/kent8192/reinhardt-web/compare/reinhardt-db@v0.1.0-alpha.10...reinhardt-db@v0.1.0-alpha.11) - 2026-02-14

### Maintenance

- updated the following local packages: reinhardt-query, reinhardt-conf

## [0.1.0-alpha.10](https://github.com/kent8192/reinhardt-web/compare/reinhardt-db@v0.1.0-alpha.9...reinhardt-db@v0.1.0-alpha.10) - 2026-02-14

### Maintenance

- updated the following local packages: reinhardt-query, reinhardt-conf

## [0.1.0-alpha.9](https://github.com/kent8192/reinhardt-web/compare/reinhardt-db@v0.1.0-alpha.8...reinhardt-db@v0.1.0-alpha.9) - 2026-02-14

### Changed

- *(db)* replace super::super:: with crate:: absolute paths in migrations
- *(db)* fix unused variable assignments in migration operation tests

### Fixed

- *(db)* bind insert values in many-to-many manager instead of discarding

## [0.1.0-alpha.8](https://github.com/kent8192/reinhardt-web/compare/reinhardt-db@v0.1.0-alpha.7...reinhardt-db@v0.1.0-alpha.8) - 2026-02-12

### Changed

- convert relative paths to absolute paths
- *(db)* convert relative paths to absolute paths in orm execution
- restore single-level super:: paths preserved by convention

### Fixed

- correct incorrect path conversions in test imports

## [0.1.0-alpha.7](https://github.com/kent8192/reinhardt-web/compare/reinhardt-db@v0.1.0-alpha.6...reinhardt-db@v0.1.0-alpha.7) - 2026-02-10

### Fixed

- *(db)* remove unused reinhardt-test dev-dependency
- *(release)* revert unpublished crate versions to pre-release state

### Reverted

- undo PR [[#219](https://github.com/kent8192/reinhardt-web/issues/219)](https://github.com/kent8192/reinhardt-web/issues/219) version bumps for unpublished crates

## [0.1.0-alpha.6](https://github.com/kent8192/reinhardt-web/compare/reinhardt-db@v0.1.0-alpha.5...reinhardt-db@v0.1.0-alpha.6) - 2026-02-06

### Other

- updated the following local packages: reinhardt-di, reinhardt-conf

## [0.1.0-alpha.5](https://github.com/kent8192/reinhardt-web/compare/reinhardt-db@v0.1.0-alpha.4...reinhardt-db@v0.1.0-alpha.5) - 2026-02-03

### Other

- updated the following local packages: reinhardt-core, reinhardt-conf, reinhardt-di

## [0.1.0-alpha.4](https://github.com/kent8192/reinhardt-web/compare/reinhardt-db@v0.1.0-alpha.3...reinhardt-db@v0.1.0-alpha.4) - 2026-02-03

### Other

- merge main into chore/release-plz-migration
- add release-plz migration markers to CHANGELOGs
- N/A

### Added
- Work in progress features (not yet released)

### Changed
- N/A

### Deprecated
- N/A

### Removed
- N/A

### Fixed
- N/A

### Security
- N/A


<!-- release-plz-separator -->
<!-- Entries below this line were created before release-plz adoption -->

## [0.1.0-alpha.3] - 2026-01-30

### Changed

- Version bump for publish workflow correction (no functional changes)

## [0.1.0-alpha.2] - 2026-01-29

### Changed

- Improve CHECK constraints comments in PostgreSQL and MySQL introspectors for clarity
- Update package version from workspace reference to explicit version

## [0.1.0-alpha.1] - 2026-01-23

### Added

- Initial crates.io release

