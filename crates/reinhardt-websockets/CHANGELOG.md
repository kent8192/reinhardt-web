# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
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


## [0.1.0-alpha.2] - 2026-01-24

### Changed

- **BREAKING**: `PagesAuthenticator::authenticate_from_cookies` now returns proper error instead of panicking with `todo!()`
  - Session store integration with reinhardt-pages is deferred to future release
  - Method now returns `AuthenticationFailed` error with descriptive message
  - See [#22](https://github.com/kent8192/reinhardt-web/issues/22) for implementation tracking

### Added

- Error case tests for `PagesAuthenticator::authenticate_from_cookies`
- Documentation about current limitations and future plans

### Fixed

- Removed runtime panic risk from `todo!()` macro in production code


## [0.1.0-alpha.1] - 2026-01-23

### Added

- Initial crates.io release

