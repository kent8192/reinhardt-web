# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **BREAKING**: `PagesAuthenticator::authenticate_from_cookies` now returns proper error instead of panicking with `todo!()`
  - Session store integration with reinhardt-pages is fully implemented
  - Method now returns `AuthenticationFailed` error with descriptive message when authentication fails
  - See [#22](https://github.com/kent8192/reinhardt-web/issues/22) for implementation tracking

### Added

- `PagesAuthenticator` now supports generic `SessionBackend` for flexible session management
- Error case tests for `PagesAuthenticator::authenticate_from_cookies`
- Builder pattern methods: `with_cookie_name()`, `with_timeout()`
- Documentation about session key specifications

### Fixed

- Removed runtime panic risk from `todo!()` macro in production code

## [0.1.0-alpha.3](https://github.com/kent8192/reinhardt-web/compare/reinhardt-websockets@v0.1.0-alpha.2...reinhardt-websockets@v0.1.0-alpha.3) - 2026-02-03

### Other

- updated the following local packages: reinhardt-pages, reinhardt-di

## [0.1.0-alpha.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-websockets@v0.1.0-alpha.1...reinhardt-websockets@v0.1.0-alpha.2) - 2026-02-03

### Other

- add release-plz migration markers to CHANGELOGs
- *(changelog)* remove obsolete [0.1.0] sections
- *(changelog)* add missing 0.1.0-alpha.1 release entries
- *(package)* replace version.workspace with explicit versions
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

## [0.1.0-alpha.1] - 2026-01-23

### Added

- Initial crates.io release
