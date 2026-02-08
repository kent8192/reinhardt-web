# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-alpha.6](https://github.com/kent8192/reinhardt-web/compare/reinhardt-web@v0.1.0-alpha.5...reinhardt-web@v0.1.0-alpha.6) - 2026-02-07

### Other

- Merge pull request #129 from kent8192/fix/issue-128-bug-runserver-uses-settingsdefault-instead-of-loading-from-settings-directory

## [0.1.0-alpha.5](https://github.com/kent8192/reinhardt-web/compare/reinhardt-web@v0.1.0-alpha.4...reinhardt-web@v0.1.0-alpha.5) - 2026-02-07

### Fixed

- add version to reinhardt-test workspace dependency for crates.io publishing
- *(utils)* remove unused dev-dependencies to break circular publish chain
- *(ci)* improve publish-check filter for non-publishable crates
- remove reinhardt-urls from doc example to avoid circular dependency
- break circular dependency between reinhardt-openapi-macros and reinhardt-rest
- remove unused dev-dependencies from reinhardt-rest
- remove reinhardt-di self-reference dev-dependency

### Other

- undo unpublished reinhardt-web v0.1.0-alpha.5 version bump and CHANGELOG entry
- release
- Revert "Merge pull request #202 from kent8192/release-plz-2026-02-06T13-32-57Z"
- release
- skip publish-check for release-plz branches
- add secrets inherit to reusable workflows
- install protoc for reinhardt-grpc build
- add publish dry-run check to detect circular dev-dependencies

## [0.1.0-alpha.4](https://github.com/kent8192/reinhardt-web/compare/reinhardt-web@v0.1.0-alpha.3...reinhardt-web@v0.1.0-alpha.4) - 2026-02-03

### Other

- Merge pull request #111 from kent8192/fix/issue-81-bug-reinhardt-pages-wasm-build-fails-due-to-tokiomio-server-side-dependencies

## [0.1.0-alpha.3](https://github.com/kent8192/reinhardt-web/compare/reinhardt-web@v0.1.0-alpha.2...reinhardt-web@v0.1.0-alpha.3) - 2026-02-03

### Fixed

- *(ci)* use GitHub App token for release-plz to trigger CI workflows
- add publish = false to example packages
- *(ci)* add missing example packages to release-plz exclusion list
- *(ci)* use registry-manifest-path to avoid workspace member errors
- *(ci)* run release-plz only on push to main
- *(ci)* use release-plz update for PR validation
- *(ci)* use release --dry-run for PR validation
- *(ci)* remove WASM build artifacts from git tracking
- *(examples)* standardize settings file pattern with .example.toml
- *(ci)* remove proptest regression files from git tracking
- *(ci)* use jlumbroso/free-disk-space for ui-test workflow
- *(ci)* use jlumbroso/free-disk-space for examples tests
- *(ci)* use jlumbroso/free-disk-space for integration tests
- *(ci)* add disk cleanup step to integration test workflows
- *(ci)* increase root-reserve-mb from 4GB to 8GB
- *(ci)* move docker pull after rust setup to avoid disk space issues
- *(ci)* replace docker save/load with pull-only approach
- *(ci)* delete tar files after docker load to save disk space
- *(ci)* split docker image lists by workflow to avoid disk space issues

### Other

- Merge pull request #167 from kent8192/fix/release-plz-cargo-metadata-warn
- *(release-plz)* use prebuilt binary for dry-run testing
- *(release-plz)* add --dry-run flag for debugging
- *(openapi)* bump version to 0.1.0-alpha.2 for release-plz fix
- add docker image caching to avoid rate limits
- Merge pull request #110 from kent8192/fix/issue-83-docs-improve-getting-started-experience-and-ecosystem-documentation
- change release PR branch prefix to release/
- merge main into chore/release-plz-migration
- update release label description for release-plz
- add release-plz migration markers to CHANGELOGs
- remove cargo-workspaces configuration from Cargo.toml
- update CLAUDE.md for release-plz migration
- simplify release commits section for release-plz
- rewrite release process documentation for release-plz
- remove Version Cascade Policy
- remove cargo-workspaces publish workflows
- add release-plz GitHub Actions workflow
- add release-plz configuration

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
