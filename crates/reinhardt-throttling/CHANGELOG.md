# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-throttling@v0.1.0-rc.30...reinhardt-throttling@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-throttling` as part of the
reinhardt-web 0.1.0 release. Provides async rate limiting with pluggable
in-memory and Redis backends, plus optional GeoIP-aware policies.

For the workspace-wide release narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is in the [Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **Per-key token and leaky bucket** — `TokenBucket` and leaky-bucket
  throttles maintain per-key state with lazy initialization and a
  capacity-overflow check, so traffic from one identifier cannot
  starve others.
- **Atomic distributed limiting** — The Redis backend uses a Lua
  script for atomic `INCR` / `EXPIRE`, eliminating the read-then-write
  race that earlier prereleases exposed.
- **Bounded memory backend** — `MemoryBackend` enforces TTL-based
  eviction and a bounded `HashMap` cap (`max_entries`, kept private to
  preserve SemVer compatibility) so long-running processes do not leak
  per-key state.
- **Hardened public surface** — `TimeRange::new` returns `Result`
  rather than panicking, refill intervals and time windows are
  validated, and cache keys are sanitized to prevent injection.
- **Optional GeoIP gating** — Behind the `geo-limiting` feature flag,
  throttles can route decisions through `maxminddb` country-code
  lookups for region-aware policies.

### Notable Breaking Changes

- **Bucket struct fields removed** — `refactor!(throttling): remove
  unused key and backend fields from bucket structs`. Code that
  matched on these fields by name must drop them.

Workspace-level breaking changes are tracked at the
[Breaking Changes Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/breaking-changes)
and summarized in the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).

### Migration Notes

See the workspace-level [Migration Guide](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22)
for the full upgrade flow. Crate-specific notes:

- Replace any direct field access on bucket structs that referenced
  the removed `key` / `backend` fields.
- Replace `TimeRange::new(...)` call sites that previously assumed
  infallibility with `Result`-handling.
