# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0-alpha.1](https://github.com/kent8192/reinhardt-web/compare/reinhardt-deeplink@v0.3.2...reinhardt-deeplink@v0.4.0-alpha.1) - 2026-07-20

### Fixed

- *(routers)* align consumers with scoped Copy signals
- *(deeplink)* preserve client router extension
- *(release)* restore develop prerelease lifecycle

## [0.3.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-deeplink@v0.2.0...reinhardt-deeplink@v0.3.0) - 2026-06-28

Stable release of `reinhardt-deeplink` for the Reinhardt 0.3.0 line. This
entry consolidates the 0.3.0 release-candidate series into one
stable release section.

### Migration Notes

- Review the root CHANGELOG and `instructions/MIGRATION_0.3.md` before upgrading from 0.2.x.

### Added

- *(urls)* [**breaking**] remove raw server route registration APIs

## [0.2.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-deeplink@v0.1.3...reinhardt-deeplink@v0.2.0) - 2026-06-11

Stable release of `reinhardt-deeplink` for the Reinhardt 0.2.0 line. This
entry consolidates the 0.2.0 release-candidate series into one
stable release section.

### Migration Notes

- Review the breaking changes listed below before upgrading from 0.1.x.
- See the root CHANGELOG and `instructions/MIGRATION_0.2.md` for cross-crate migration guidance.

### Added

- *(deeplink)* add DeeplinkSettings fragment

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Deprecated

- *(deeplink)* deprecate DeeplinkConfig in favor of DeeplinkSettings

### Fixed

- *(settings)* require explicit nested settings nodes
- *(deeplink)* derive Default for DeeplinkSettings
- *(build)* port Codex review follow-ups
- *(ci)* recover develop release-plz prerelease

### Documentation

- *(deeplink)* document #![allow(deprecated)] allowances

### Maintenance

- *(deeplink)* add reinhardt-conf dependency for settings fragments

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-deeplink@v0.1.0-rc.30...reinhardt-deeplink@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-deeplink` as part of the
reinhardt-web 0.1.0 release. Provides mobile-app deep-linking primitives
for iOS Universal Links, Android App Links, and custom URL schemes.

For the workspace-wide release narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is in the [Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **Universal Links and App Links** — Helpers for serving the
  Apple `apple-app-site-association` and Android
  `assetlinks.json` manifests over `reinhardt-http`, so deep-link
  associations are configured in the same project as the routes.
- **Custom URL scheme dispatch** — Async dispatch primitives accept
  inbound deep-link URLs, resolve them against typed `reinhardt-urls`
  patterns, and forward to handlers without parsing raw URL strings
  at the call site.
- **Composable with `grpc` and `dispatch` re-exports** — The crate
  exposes feature-gated re-exports for the gRPC and dispatch
  integrations so downstream code can opt in without depending on the
  full surface.

### Notable Breaking Changes

`reinhardt-deeplink` did not introduce its own framework-wide breaking
changes in 0.1.0. Workspace-level breaking changes are tracked at the
[Breaking Changes Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/breaking-changes)
and summarized in the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).

### Migration Notes

See the workspace-level [Migration Guide](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22)
for the full upgrade flow. This crate has no crate-specific migration
steps for the 0.1.0 transition.
