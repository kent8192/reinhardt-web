# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- release-plz-separator -->
<!-- Entries below this line were created before release-plz adoption -->

## [0.1.0-alpha.1] - 2025-01-26

### Added

- Initial release of `reinhardt-openapi` crate
- `OpenApiRouter` wrapper for automatic OpenAPI documentation endpoints
- Swagger UI endpoint at `/api/docs`
- Redoc UI endpoint at `/api/redoc`
- OpenAPI JSON endpoint at `/api/openapi.json`
- Handler and Router trait implementations for `OpenApiRouter`

### Notes

- This crate was extracted from `reinhardt-rest` to resolve circular dependency issues
- See [Issue #23](https://github.com/kent8192/reinhardt-web/issues/23) for details
