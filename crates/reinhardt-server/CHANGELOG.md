# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.4](https://github.com/kent8192/reinhardt-web/compare/reinhardt-server@v0.1.3...reinhardt-server@v0.1.4) - 2026-06-09

### Maintenance

- update Cargo.toml dependencies

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-server@v0.1.0-rc.30...reinhardt-server@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-server` as part of the
reinhardt-web 0.1.0 release. The hyper-based HTTP server launcher that
wires routers, middleware, and the request-scoped DI container into a
production-ready listener.

For the workspace-wide release narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is in the [Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **Hyper-based listener** — Drives the assembled router and middleware
  chain over hyper, with sensible defaults and a reduced Tokio feature
  surface; the build profile uses `debug=1` to keep compile times
  manageable without losing essential symbols.
- **Hardened error responses** — All server-level errors flow through
  `SafeErrorResponse` so stack traces and internal details never leak
  to clients.
- **Sliding-window rate limiting** — Built-in rate limiter applies a
  sliding window and evicts stale entries periodically; trusted-proxy
  validation gates `X-Forwarded-For` so request IP attribution stays
  honest behind load balancers.
- **Body-size and decompression guardrails** — Request body size
  limits and decompression-bomb prevention sit in front of the
  middleware chain, so malicious payloads are rejected before they
  reach handlers.
- **Reduced log surface** — WebSocket logging verbosity is dialled
  back so the access log does not exfiltrate payload data.

### Notable Breaking Changes

This crate's API surface stabilized incrementally during the rc
cycle; cross-crate breaking changes are catalogued in the
[Breaking Changes Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/breaking-changes).

### Migration Notes

This is the first stable release, so there is no prior stable version
to migrate from. See the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22)
for the cross-crate migration guide.
