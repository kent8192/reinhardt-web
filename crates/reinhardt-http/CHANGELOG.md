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


<!-- release-plz-separator -->
<!-- Entries below this line were created before release-plz adoption -->

## [0.1.0-alpha.3] - 2026-01-30

### Changed

- Version bump for publish workflow correction (no functional changes)

## [0.1.0-alpha.2] - 2026-01-29

### Added

- `extract_bearer_token()` - Extract Bearer token from Authorization header
- `get_header()` - Get specific header value
- `get_client_ip()` - Get client IP from X-Forwarded-For/X-Real-IP/remote_addr
- `validate_content_type()` - Validate Content-Type header
- `query_as<T>()` - Type-safe query parameter deserialization

### Notes

- Methods migrated from reinhardt-micro crate for better API ergonomics

## [0.1.0-alpha.1] - 2026-01-23

### Added

- Initial crates.io release

