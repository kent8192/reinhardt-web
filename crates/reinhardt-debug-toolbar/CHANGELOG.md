# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.1](https://github.com/kent8192/reinhardt-web/releases/tag/reinhardt-debug-toolbar@v0.2.0-rc.1) - 2026-05-24

### Added

- *(debug)* add reinhardt-debug-toolbar crate for development tools

### Changed

- *(debug-toolbar)* simplify nested conditionals using let-chains
- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Fixed

- resolve test module resolution errors in debug-toolbar
- add missing #[cfg(feature = "sql-panel")] gates to test imports and fixtures
- remove develop/0.2.0 content accidentally merged via PR [[#1918](https://github.com/kent8192/reinhardt-web/issues/1918)](https://github.com/kent8192/reinhardt-web/issues/1918)

### Testing

- *(debug-toolbar)* add test infrastructure with builders, fixtures, and mock panel
- *(debug-toolbar)* add HTTP response helper functions for integration tests
- *(debug-toolbar)* add 55 unit tests covering context, panels, config, errors, and sanitization
- *(debug-toolbar)* add 18 integration tests for toolbar injection, middleware, and use cases
- *(debug-toolbar)* add 19 feature-gated tests for SQL panel, normalization, and N+1 detection

### Added
- Initial implementation of debug toolbar framework
- SQL query panel with duplicate detection
- Request/Response information panel
- Tower/Axum middleware integration
- Task-local context storage
- HTML toolbar injection

## [0.1.0-alpha.1] - TBD

### Added
- Project initialization
- Core architecture design
- Feature flag system
