# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-tasks@v0.1.3...reinhardt-tasks@v0.2.0) - 2026-06-11

Stable release of `reinhardt-tasks` for the Reinhardt 0.2.0 line. This
entry consolidates the 0.2.0 release-candidate series; the original
RC entries remain below as detailed history.

### Migration Notes

- Review the breaking changes listed below before upgrading from 0.1.x.
- See the root CHANGELOG and `instructions/MIGRATION_0.2.md` for cross-crate migration guidance.

### Breaking Changes

- *(tasks)* [**breaking**] remove misleading create_queue_from_settings API

### Added

- *(tasks)* add settings fragments and settings-first constructors

### Deprecated

- *(tasks)* deprecate config structs in favor of settings fragments

### Fixed

- *(settings)* require explicit nested settings nodes
- *(tasks)* [**breaking**] remove misleading create_queue_from_settings API

- *(ci)* pin broken upstream transitive releases

### Documentation

- *(tasks)* note that create_queue_from_settings does not retain settings
- *(tasks)* correct tracking issue reference to [[#5068](https://github.com/kent8192/reinhardt-web/issues/5068)](https://github.com/kent8192/reinhardt-web/issues/5068)
- *(tasks)* move deferred queue-settings note to lib.rs header

### Maintenance

- update Cargo.toml dependencies
- *(tasks)* add reinhardt-conf and reinhardt-core dependencies for settings


## [0.2.0-rc.5](https://github.com/kent8192/reinhardt-web/compare/reinhardt-tasks@v0.2.0-rc.4...reinhardt-tasks@v0.2.0-rc.5) - 2026-06-11

### Maintenance

- update Cargo.toml dependencies

## [0.2.0-rc.4](https://github.com/kent8192/reinhardt-web/compare/reinhardt-tasks@v0.2.0-rc.3...reinhardt-tasks@v0.2.0-rc.4) - 2026-06-06

### Fixed

- *(settings)* require explicit nested settings nodes

## [0.2.0-rc.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-tasks@v0.1.3...reinhardt-tasks@v0.2.0-rc.2) - 2026-06-03

### Added

- *(tasks)* add settings fragments and settings-first constructors

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Deprecated

- *(tasks)* deprecate config structs in favor of settings fragments

### Documentation

- *(tasks)* note that create_queue_from_settings does not retain settings
- *(tasks)* correct tracking issue reference to [[#5068](https://github.com/kent8192/reinhardt-web/issues/5068)](https://github.com/kent8192/reinhardt-web/issues/5068)
- *(tasks)* move deferred queue-settings note to lib.rs header

### Fixed

- *(ci)* recover develop release-plz prerelease
- *(tasks)* [**breaking**] remove misleading create_queue_from_settings API

### Maintenance

- *(tasks)* add reinhardt-conf and reinhardt-core dependencies for settings

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-tasks@v0.1.0-rc.30...reinhardt-tasks@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-tasks` as part of the
reinhardt-web 0.1.0 release. Provides the async task framework
underpinning background jobs, scheduled work, and webhook fan-out
across Redis, RabbitMQ, SQS, and Kafka backends.

For the workspace-wide release narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is in the [Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **Pluggable backends** — Redis, RabbitMQ, SQS, and (via
  `kafka-backend` through `reinhardt-streaming`) Kafka, all behind a
  uniform `TaskQueue::enqueue` surface that delegates to the
  selected backend.
- **Atomic enqueue and lock acquisition** — Redis uses a
  `MULTI`/`EXEC` transaction for atomic enqueue; lock acquisition
  verifies ownership and rolls back DAG edges if the post-lock
  validation fails.
- **Scheduler with bounded concurrency** — the scheduler runs tasks
  asynchronously with a tokio `Semaphore`-bounded concurrency limit,
  supports graceful shutdown, and uses an instance-field counter to
  avoid contention in the priority queue.
- **Weight-based priorities** — `Priority` orders work by explicit
  weight rather than enum-variant accident, and rejects zero-weight
  division at enqueue time.
- **Hardened scheduling primitives** — TTL truncation is fixed,
  `RetryStrategy` multipliers are validated, integer underflow and
  duration overflow paths panic-free, and the SQS adapter only
  deletes messages after successful processing.
- **Webhook fan-out with SSRF protection** — webhook URLs are
  validated to prevent SSRF, retries sleep on a tested cadence, and
  failures use structured `tracing` logging rather than ad-hoc
  `println!` / `eprintln!`.
- **UUID v7 for IDs (v4 for tokens)** — task IDs migrated to UUID v7
  for time-sortable ordering; security-sensitive tokens stay on v4.

### Notable Breaking Changes

The task framework had no public-API breaking changes specific to
this crate at 0.1.0. Workspace-wide DI changes
([#3628](https://github.com/kent8192/reinhardt-web/discussions/3628),
[#3631](https://github.com/kent8192/reinhardt-web/discussions/3631))
affect task handlers that inject dependencies; follow the [root
CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#breaking-changes)
for the full list.

### Migration Notes

See the [root CHANGELOG migration guide](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#migration-guide).
Existing task definitions continue to compile; the user-visible
runtime changes are stricter validation (zero-weight priority, TTL
truncation, SSRF on webhook URLs) and the new `kafka-backend`
feature flag if you want Kafka delivery.
