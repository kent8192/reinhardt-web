# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-dispatch@v0.1.0...reinhardt-dispatch@v0.2.0-rc.2) - 2026-05-24

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-dispatch@v0.1.0-rc.30...reinhardt-dispatch@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-dispatch` as part of the
reinhardt-web 0.1.0 release. This crate is the request dispatch
runtime: it joins the URL resolver (`reinhardt-urls`), the middleware
chain (`reinhardt-middleware`), and the view handlers (`reinhardt-views`)
into a single executable pipeline that turns an incoming HTTP request
into a response, and it owns the dispatcher-level exception handler
and signal-handling glue.

For the workspace-wide release narrative (Highlights, Breaking
Changes, Migration Guide), see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is preserved in the
[Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **Routing × middleware dispatch pipeline** — resolves the URL via
  `reinhardt-urls`, runs the middleware chain with per-request DI
  forked off the application context, and invokes the matched view.
  Default-handler resolution preserves the request context across
  every layer (no lost extensions on the fallback path).
- **Configurable middleware-chain depth limit** — the chain depth is
  bounded by configuration to prevent infinite-recursion DoS via
  pathological middleware composition.
- **Signal dispatch with error logging** — signal sends log their
  errors instead of silently discarding them, and the signal lock
  is released before user callbacks run so handler panics cannot
  deadlock the dispatcher.
- **Dispatcher-level exception handler** — produces structured error
  responses with `Content-Type` and `X-Content-Type-Options: nosniff`
  headers, and is hardened against information disclosure (no
  internal type names or stack frames leak into responses).
- **Lock-poisoning recovery** — `Mutex` / `RwLock` accesses use the
  workspace-wide poison-recovery helpers from `reinhardt-utils`
  instead of `unwrap()`, so a single poisoned guard cannot take
  down the dispatcher.

### Notable Breaking Changes

Tracked at the workspace level — see the [root CHANGELOG Breaking Changes section](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#breaking-changes).

### Migration Notes

See the [root Migration Guide](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#migration-guide)
for project-wide guidance.
