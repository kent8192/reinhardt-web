# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Sub-Crate Updates

<!-- Add sub-crate updates here following the format:
- `[crate-name]` updated to v[version] ([CHANGELOG](crates/[crate-name]/CHANGELOG.md#[anchor]))
  - Brief summary of key changes
-->

<!-- release-plz-separator -->
<!-- Entries below this line were created before release-plz adoption -->

## [0.1.0-alpha.2] - 2026-01-29

### Changed

- **BREAKING**: Update `static-files` feature to use `reinhardt-utils/staticfiles` (#114)

### Sub-Crate Updates

- `reinhardt-utils` updated to v0.1.0-alpha.4 ([CHANGELOG](crates/reinhardt-utils/CHANGELOG.md#010-alpha4---2026-01-30))
  - Re-release after version correction
- `reinhardt-conf` updated to v0.1.0-alpha.4 ([CHANGELOG](crates/reinhardt-conf/CHANGELOG.md#010-alpha4---2026-01-30))
  - Re-release after version correction
- `reinhardt-pages` updated to v0.1.0-alpha.4 ([CHANGELOG](crates/reinhardt-pages/CHANGELOG.md#010-alpha4---2026-01-30))
  - Re-release after version correction
- `reinhardt-test` updated to v0.1.0-alpha.4 ([CHANGELOG](crates/reinhardt-test/CHANGELOG.md#010-alpha4---2026-01-30))
  - Re-release after version correction
- `reinhardt-commands` updated to v0.1.0-alpha.4 ([CHANGELOG](crates/reinhardt-commands/CHANGELOG.md#010-alpha4---2026-01-30))
  - Version bump for publish workflow correction
- `reinhardt-rest` updated to v0.1.0-alpha.4 ([CHANGELOG](crates/reinhardt-rest/CHANGELOG.md#010-alpha4---2026-01-30))
  - Version bump for publish workflow correction
- `reinhardt-http` updated to v0.1.0-alpha.3 ([CHANGELOG](crates/reinhardt-http/CHANGELOG.md#010-alpha3---2026-01-30))
  - Version bump for publish workflow correction
- `reinhardt-db` updated to v0.1.0-alpha.3 ([CHANGELOG](crates/reinhardt-db/CHANGELOG.md#010-alpha3---2026-01-30))
  - Version bump for publish workflow correction
- `reinhardt-forms` updated to v0.1.0-alpha.3 ([CHANGELOG](crates/reinhardt-forms/CHANGELOG.md#010-alpha3---2026-01-30))
  - Version bump for publish workflow correction
- `reinhardt-pages-macros` updated to v0.1.0-alpha.3 ([CHANGELOG](crates/reinhardt-pages/macros/CHANGELOG.md#010-alpha3---2026-01-30))
  - Version bump for publish workflow correction

## [0.1.0-alpha.1] - 2026-01-23

### Sub-Crate Updates

- `reinhardt-shortcuts` updated to v0.1.0-alpha.2 ([CHANGELOG](crates/reinhardt-shortcuts/CHANGELOG.md#010-alpha2---2026-01-23))
  - Initial release with keyboard shortcut support
- `reinhardt-i18n` updated to v0.1.0-alpha.2 ([CHANGELOG](crates/reinhardt-i18n/CHANGELOG.md#010-alpha2---2026-01-23))
  - Initial release with internationalization support

### Added

- Initial release of the full-stack API framework facade crate
- Feature presets: minimal, standard, full, api-only, graphql-server, websocket-server, cli-tools, test-utils
- Fine-grained feature flags for authentication, database backends, middleware, and more
- WASM target support via conditional compilation
- Re-exports of all Reinhardt sub-crates through a unified API

---

## Sub-Crate CHANGELOGs

For detailed changes in individual sub-crates, refer to their respective CHANGELOG files:

### Core & Foundation
- [reinhardt-core](crates/reinhardt-core/CHANGELOG.md) - Core framework types and traits
- [reinhardt-utils](crates/reinhardt-utils/CHANGELOG.md) - Utility functions and macros
- [reinhardt-conf](crates/reinhardt-conf/CHANGELOG.md) - Configuration management

### Database & ORM
- [reinhardt-db](crates/reinhardt-db/CHANGELOG.md) - Database connection and query building

### Dependency Injection
- [reinhardt-di](crates/reinhardt-di/CHANGELOG.md) - Dependency injection container
- [reinhardt-dentdelion](crates/reinhardt-dentdelion/CHANGELOG.md) - DI macros and utilities

### HTTP & REST
- [reinhardt-http](crates/reinhardt-http/CHANGELOG.md) - HTTP server and request handling
- [reinhardt-rest](crates/reinhardt-rest/CHANGELOG.md) - REST API framework
- [reinhardt-middleware](crates/reinhardt-middleware/CHANGELOG.md) - HTTP middleware
- [reinhardt-server](crates/reinhardt-server/CHANGELOG.md) - Server runtime

### GraphQL & gRPC
- [reinhardt-graphql](crates/reinhardt-graphql/CHANGELOG.md) - GraphQL server implementation
- [reinhardt-graphql-macros](crates/reinhardt-graphql/macros/CHANGELOG.md) - GraphQL procedural macros
- [reinhardt-grpc](crates/reinhardt-grpc/CHANGELOG.md) - gRPC server implementation

### WebSockets & Real-time
- [reinhardt-websockets](crates/reinhardt-websockets/CHANGELOG.md) - WebSocket support

### Authentication & Authorization
- [reinhardt-auth](crates/reinhardt-auth/CHANGELOG.md) - Authentication and authorization

### Views & Forms
- [reinhardt-views](crates/reinhardt-views/CHANGELOG.md) - View rendering and templates
- [reinhardt-forms](crates/reinhardt-forms/CHANGELOG.md) - Form handling and validation

### Routing & Dispatch
- [reinhardt-urls](crates/reinhardt-urls/CHANGELOG.md) - URL routing
- [reinhardt-dispatch](crates/reinhardt-dispatch/CHANGELOG.md) - Request dispatcher
- [reinhardt-commands](crates/reinhardt-commands/CHANGELOG.md) - Command pattern implementation

### Background Tasks & Messaging
- [reinhardt-tasks](crates/reinhardt-tasks/CHANGELOG.md) - Background task queue
- [reinhardt-mail](crates/reinhardt-mail/CHANGELOG.md) - Email sending

### Internationalization & Shortcuts
- [reinhardt-i18n](crates/reinhardt-i18n/CHANGELOG.md) - Internationalization support
- [reinhardt-shortcuts](crates/reinhardt-shortcuts/CHANGELOG.md) - Keyboard shortcuts

### Admin & CLI
- [reinhardt-admin](crates/reinhardt-admin/CHANGELOG.md) - Admin interface
- [reinhardt-admin-cli](crates/reinhardt-admin-cli/CHANGELOG.md) - Admin CLI tools

### Testing
- [reinhardt-test](crates/reinhardt-test/CHANGELOG.md) - Testing utilities and fixtures
