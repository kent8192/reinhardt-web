# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- release-plz-separator -->
<!-- Entries below this line were created before release-plz adoption -->

## [0.1.0-alpha.12](https://github.com/kent8192/reinhardt-web/compare/reinhardt-admin@v0.1.0-alpha.11...reinhardt-admin@v0.1.0-alpha.12) - 2026-02-15

### Maintenance

- updated the following local packages: reinhardt-urls, reinhardt-pages

## [0.1.0-alpha.11](https://github.com/kent8192/reinhardt-web/compare/reinhardt-admin@v0.1.0-alpha.10...reinhardt-admin@v0.1.0-alpha.11) - 2026-02-14

### Maintenance

- updated the following local packages: reinhardt-auth, reinhardt-urls, reinhardt-pages

## [0.1.0-alpha.10](https://github.com/kent8192/reinhardt-web/compare/reinhardt-admin@v0.1.0-alpha.9...reinhardt-admin@v0.1.0-alpha.10) - 2026-02-14

### Maintenance

- updated the following local packages: reinhardt-query, reinhardt-apps, reinhardt-db, reinhardt-db, reinhardt-auth, reinhardt-urls, reinhardt-pages

## [0.1.0-alpha.9](https://github.com/kent8192/reinhardt-web/compare/reinhardt-admin@v0.1.0-alpha.8...reinhardt-admin@v0.1.0-alpha.9) - 2026-02-14

### Maintenance

- updated the following local packages: reinhardt-query, reinhardt-apps, reinhardt-db, reinhardt-db, reinhardt-auth, reinhardt-urls, reinhardt-pages

## [0.1.0-alpha.7](https://github.com/kent8192/reinhardt-web/compare/reinhardt-admin@v0.1.0-alpha.6...reinhardt-admin@v0.1.0-alpha.7) - 2026-02-12

### Changed

- convert relative paths to absolute paths

## [0.1.0-alpha.6](https://github.com/kent8192/reinhardt-web/compare/reinhardt-admin@v0.1.0-alpha.5...reinhardt-admin@v0.1.0-alpha.6) - 2026-02-10

### Maintenance

- *(clippy)* add deny lints for todo/unimplemented/dbg_macro

## [0.1.0-alpha.5](https://github.com/kent8192/reinhardt-web/compare/reinhardt-admin@v0.1.0-alpha.4...reinhardt-admin@v0.1.0-alpha.5) - 2026-02-10

### Fixed

- *(admin)* move database tests to integration crate to break circular publish chain
- *(release)* revert unpublished crate versions to pre-release state

### Reverted

- undo PR [[#219](https://github.com/kent8192/reinhardt-web/issues/219)](https://github.com/kent8192/reinhardt-web/issues/219) version bumps for unpublished crates

## [0.1.0-alpha.4](https://github.com/kent8192/reinhardt-web/compare/reinhardt-admin@v0.1.0-alpha.3...reinhardt-admin@v0.1.0-alpha.4) - 2026-02-06

### Other

- updated the following local packages: reinhardt-utils, reinhardt-di, reinhardt-apps, reinhardt-db, reinhardt-db, reinhardt-auth, reinhardt-urls, reinhardt-pages

## [0.1.0-alpha.3](https://github.com/kent8192/reinhardt-web/compare/reinhardt-admin@v0.1.0-alpha.2...reinhardt-admin@v0.1.0-alpha.3) - 2026-02-03

### Other

- updated the following local packages: reinhardt-core, reinhardt-pages, reinhardt-http, reinhardt-utils, reinhardt-di, reinhardt-apps, reinhardt-db, reinhardt-db, reinhardt-auth, reinhardt-urls

## [0.1.0-alpha.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-admin@v0.1.0-alpha.1...reinhardt-admin@v0.1.0-alpha.2) - 2026-02-03

### Other

- add release-plz migration markers to CHANGELOGs
- *(changelog)* add missing 0.1.0-alpha.1 release entries
- *(package)* replace version.workspace with explicit versions

## [0.1.0-alpha.1] - 2026-01-23

### Added
- Initial release
- Admin panel functionality (via `reinhardt-panel`)
- CLI tool functionality (via `reinhardt-cli`)
