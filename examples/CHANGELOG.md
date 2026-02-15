# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2026-01-24

### Added
- Initial release with examples extracted from main repository
- 7 local development examples:
  - `examples-hello-world`: Minimal Reinhardt application
  - `examples-rest-api`: RESTful API with Django-style structure
  - `examples-database-integration`: Database integration with migrations
  - `examples-tutorial-basis`: Basic tutorial example
  - `examples-tutorial-rest`: REST API tutorial
  - `examples-github-issues`: GitHub issues integration example
  - `examples-twitter`: Twitter integration example
- Shared common utilities (`common/`)
- Shared test macros (`test-macros/`)
- CI/CD workflows with matrix testing
- Daily compatibility checks with main repository
- Helper scripts for synchronization:
  - `scripts/sync-from-main.sh`: Pull updates from main repository
  - `scripts/test-all.sh`: Test all examples
- Comprehensive documentation:
  - README.md: Overview and usage guide
  - CONTRIBUTING.md: Contribution guidelines
  - SUBTREE_OPERATIONS.md: Git subtree operation guide
  - COMPATIBILITY.json: Version compatibility matrix
- Dual MIT OR Apache-2.0 license
- Git subtree integration with `reinhardt-web` repository

### Changed
- (None for initial release)

### Deprecated
- (None)

### Removed
- (None)

### Fixed
- (None)

### Security
- (None)

[Unreleased]: https://github.com/kent8192/reinhardt-web/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/kent8192/reinhardt-web/releases/tag/v1.0.0
