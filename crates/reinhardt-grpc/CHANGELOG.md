# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.4](https://github.com/kent8192/reinhardt-web/compare/reinhardt-grpc@v0.2.0-rc.3...reinhardt-grpc@v0.2.0-rc.4) - 2026-06-07

### Documentation

- update version references to v0.2.0-rc.4

## [0.2.0-rc.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-grpc@v0.1.3...reinhardt-grpc@v0.2.0-rc.2) - 2026-06-03

### Added

- *(grpc)* add GrpcServerSettings fragment for the grpc_server section

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Deprecated

- *(grpc)* deprecate GrpcServerConfig in favor of GrpcServerSettings

### Fixed

- *(ci)* recover develop release-plz prerelease

### Maintenance

- *(grpc)* add reinhardt-conf, reinhardt-core, serde deps for settings fragment

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-grpc@v0.1.0-rc.30...reinhardt-grpc@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-grpc` as part of the
reinhardt-web 0.1.0 release. Embeds a tonic-based gRPC server in the
framework, sharing DI, middleware, and observability with the HTTP
stack.

For the workspace-wide release narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is in the [Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **Per-request DI in gRPC handlers** — handler macros fork the
  `InjectionContext` per request, giving each gRPC call a clean
  request scope and removing implementation-name collisions in
  generated code.
- **Hardened transport limits** — request timeouts, connection
  limits, default message-size limits, and protobuf depth limits
  ship out of the box; error messages are sanitized so server-side
  details never leak to clients.
- **Strict protobuf input validation** — string fields are validated
  by Unicode scalar count (not byte length) with early-exit
  counting, and macro-generated code performs the same checks
  user-written validators would.
- **Cow-based test helpers** — `Cow<str>` is used in test messaging
  to reduce allocations without changing the public API.
- **Tower integration documented** — out-of-the-box compatibility
  with the tower middleware stack is covered in the crate docs,
  alongside the engine-name and feature-flag corrections that
  landed during the rc cycle.

### Notable Breaking Changes

- **`Injected<T>` deprecated** ([#3631](https://github.com/kent8192/reinhardt-web/discussions/3631))
  — gRPC handler injection sites move to `Depends<T>`.

### Migration Notes

See the [root CHANGELOG migration guide](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#migration-guide).
Existing tonic services keep working; only the DI-attribute syntax on
generated handler wrappers changes.
