# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-alpha.13](https://github.com/kent8192/reinhardt-web/compare/reinhardt-conf@v0.1.0-alpha.12...reinhardt-conf@v0.1.0-alpha.13) - 2026-02-18

### Fixed

- *(conf)* prevent secret exposure in serialization

### Maintenance

- *(reinhardt-conf)* add SAFETY comments to unsafe blocks in env.rs
- *(reinhardt-conf)* add SAFETY comments to unsafe blocks in testing.rs
- *(reinhardt-conf)* add SAFETY comments to unsafe blocks in env_loader.rs
- *(reinhardt-conf)* add SAFETY comments to unsafe blocks in profile.rs
- *(reinhardt-conf)* add SAFETY comments to unsafe blocks in sources.rs
- *(reinhardt-conf)* add SAFETY comments to unsafe blocks in secrets/providers/env.rs

## [0.1.0-alpha.12](https://github.com/kent8192/reinhardt-web/compare/reinhardt-conf@v0.1.0-alpha.11...reinhardt-conf@v0.1.0-alpha.12) - 2026-02-15

### Maintenance

- updated the following local packages: reinhardt-query

## [0.1.0-alpha.11](https://github.com/kent8192/reinhardt-web/compare/reinhardt-conf@v0.1.0-alpha.10...reinhardt-conf@v0.1.0-alpha.11) - 2026-02-14

### Maintenance

- updated the following local packages: reinhardt-query

## [0.1.0-alpha.10](https://github.com/kent8192/reinhardt-web/compare/reinhardt-conf@v0.1.0-alpha.9...reinhardt-conf@v0.1.0-alpha.10) - 2026-02-14

### Maintenance

- updated the following local packages: reinhardt-query

## [0.1.0-alpha.8](https://github.com/kent8192/reinhardt-web/compare/reinhardt-conf@v0.1.0-alpha.7...reinhardt-conf@v0.1.0-alpha.8) - 2026-02-12

### Changed

- convert relative paths to absolute paths
- restore single-level super:: paths preserved by convention

## [0.1.0-alpha.7](https://github.com/kent8192/reinhardt-web/compare/reinhardt-conf@v0.1.0-alpha.6...reinhardt-conf@v0.1.0-alpha.7) - 2026-02-06

### Other

- updated the following local packages: reinhardt-utils

## [0.1.0-alpha.6](https://github.com/kent8192/reinhardt-web/compare/reinhardt-conf@v0.1.0-alpha.5...reinhardt-conf@v0.1.0-alpha.6) - 2026-02-03

### Other

- updated the following local packages: reinhardt-core, reinhardt-utils

## [0.1.0-alpha.5](https://github.com/kent8192/reinhardt-web/compare/reinhardt-conf@v0.1.0-alpha.4...reinhardt-conf@v0.1.0-alpha.5) - 2026-02-03

### Other

- merge main into chore/release-plz-migration
- add release-plz migration markers to CHANGELOGs

### Breaking Changes
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

## [0.1.0-alpha.4] - 2026-01-30

### Changed

- Re-release of 0.1.0-alpha.3 content after version correction
- Update imports for `reinhardt_utils::staticfiles` module rename (#114)


## [0.1.0-alpha.3] - 2026-01-29 [YANKED]

**Note:** This version was yanked due to version skipping in the main crate (`reinhardt-web`). Use the latest available version instead.

### Changed

- Update imports for `reinhardt_utils::staticfiles` module rename (#114)

## [0.1.0-alpha.1] - 2026-01-23

### Added

- Initial crates.io release

