# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.1](https://github.com/kent8192/reinhardt-web/compare/reinhardt-openapi-macros@v0.1.0...reinhardt-openapi-macros@v0.2.0-rc.1) - 2026-05-22

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-openapi-macros@v0.1.0-rc.30...reinhardt-openapi-macros@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-openapi-macros` as part of the
reinhardt-web 0.1.0 release. Procedural-macro companion to
`reinhardt-rest` that emits operation- and container-level OpenAPI
metadata directly from ViewSet attributes.

For the workspace-wide release narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is in the [Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **Container-level OpenAPI attributes** — Attribute macros annotate
  ViewSet containers with OpenAPI schema metadata so the generated
  spec carries grouping, tagging, and documentation context.
- **Safe attribute parsing** — Parse errors propagate as proper
  `compile_error!` diagnostics instead of panicking; min/max
  constraints validate at expansion time; `get_ident()` replaces
  unchecked `expect()`.
- **Serde-aware validation** — Serde attribute handling and validation
  are normalized so OpenAPI output stays consistent with the runtime
  serialization shape.
- **Workspace-version pinning** — Proc-macro dependencies (including
  `native-tls`) align with workspace versions to keep the published
  graph reproducible.

### Notable Breaking Changes

This is a proc-macro crate consumed exclusively by `reinhardt-rest`;
breaking changes flow through that crate. See the [Breaking Changes
Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/breaking-changes).

### Migration Notes

This is the first stable release, so there is no prior stable version
to migrate from. See the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22)
for the cross-crate migration guide.
