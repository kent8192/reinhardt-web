# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0-alpha.16](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.4.0-alpha.15...reinhardt-utils@v0.4.0-alpha.16) - 2026-09-15

### Added

- *(utils/core)* Add core utilities
- *(utils/cache)* Add caching system
- *(utils/logging)* Add logging system
- *(utils/storage)* Add file storage system
- *(utils)* Add utilities umbrella crate
- *(utils,tasks,micro)* Enhance utilities, task scheduling, and framework integration
- *(cache)* Add hit/miss statistics tracking to InMemoryCache
- *(cache)* Add cache inspection functionality with key listing and entry details
- *(cache)* Add automatic background cleanup for expired entries
- *(cache)* Add Redis connection pooling with deadpool-redis
- *(cache)* Add file-based cache, cache warming, and tag-based invalidation
- *(cache)* Add hybrid cache strategy and Redis Cluster support
- *(cache)* Add layered TTL cleanup strategy inspired by Redis 6.0
- *(cache)* Add memcached, redis sentinel, and pub/sub backends
- *(cache)* Add hybrid caching strategy and DI support
- *(utils)* Improve utilities, tasks, testing, and mail modules
- *(utils)* Enhance Redis pub/sub and cluster support for cache
- *(utils/static)* Add file watcher for static development server
- *(utils/static)* Improve manifest structure and add load_manifest method
- *(logging)* Add synchronous logging methods and LogRecord extra field API
- *(workspace)* Add -full features to remaining parent crates
- *(crates)* Add full features to all sub-crates and standalone crates
- *(cache)* Add timestamp tracking to cache entries
- *(middleware,logging)* Enhance middleware and logging functionality
- Enhance cache, i18n, routers, and documentation
- *(utils)* [**breaking**] rename r#static module to staticfiles
- *(utils)* add lock poisoning recovery utilities
- *(utils)* add path sanitization and input validation helpers

### Changed

- *(forms,middleware,utils)* Update form handling, middleware, and utilities
- Various crate improvements and dependency updates
- *(cache)* Split lib.rs into focused modules to improve maintainability
- *(cache)* Relax trait bounds and skip promotion in HybridCache
- *(cache)* Rename add to with_warmer in cache warming API
- *(multi)* Improve routing, serializers, and misc components
- *(cache)* Migrate cache backend tests to TestContainers integration
- *(logging)* Rename Handler trait to LogHandler for clarity
- *(cache)* Update Redis container import to use testcontainers-modules
- *(sessions,cache)* Update rand crate API usage to latest version
- *(cache)* Improve RedisCluster configuration handling
- *(tests)* Remove unnecessary mut qualifiers from immutable variables
- *(cache)* Remove unused methods and verbose docs from RedisClusterCache
- *(placeholder)* Replace no-op implementations with proper markers
- *(workspace)* Update Cargo.toml dependencies across all crates
- *(workspace)* Replace direct crate imports with reinhardt_core facade imports
- *(workspace)* Update all crates to use reinhardt-core imports
- *(utils)* Move static module from contrib to reinhardt-utils
- *(utils)* Move humanize module from contrib to reinhardt-utils
- *(di)* Remove obsolete di_support modules after DI crate promotion
- *(utils/cache)* Improve Redis cluster and pool implementations
- *(static)* Remove obsolete sleep from dev server watcher
- Migrate remaining components to Request builder pattern
- *(tests)* Remove redundant assert!(result.is_ok()) checks
- *(logging)* Convert Logger API to impl Into<String>
- *(misc)* Remove unused imports and improve code quality across crates
- *(core)* Remove unused imports and minor optimizations
- *(misc)* Update various crates with minor fixes
- *(utils)* Update pagination, signals, and logging
- *(utils)* Optimize utility modules and form rendering
- *(core)* Update core crates with new patterns
- *(static)* Update static file handling for pages framework
- Update various modules and utilities
- *(static)* update static files middleware
- Update miscellaneous crates for API changes
- use let-chain pattern in PathResolver
- *(utils)* Update Page type imports in reinhardt-utils
- *(utils)* integrate subcrates into main crate
- *(workspace)* update remaining crates to use extracted throttling crate
- *(rest,test,utils)* organize imports and formatting
- convert relative paths to absolute paths
- restore single-level super:: paths preserved by convention
- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Documentation

- *(crates)* Standardize README formatting across all crates
- *(readme)* Sync Japanese README files with English versions across 70 crates
- Move Planned Features from README to lib.rs per DM-6 policy
- Remove all Japanese README files to reduce maintenance burden
- *(utils)* Translate utils README to English
- *(changelog)* Standardize CHANGELOG format for all publishable crates
- *(routers-macros,utils-core,views-core)* Add validation rules and HTML API details
- *(static)* Improve JsMinifier documentation
- *(utils)* Convert utility crates doctests to executable examples
- *(top-level)* Convert top-level crate and README doctests
- *(server,db,utils,websockets)* Update service layer doctests
- Update project documentation and README for framework modernization
- update READMEs for DI, utils, and other crates
- *(changelog)* add missing 0.1.0-alpha.1 release entries
- *(changelog)* correct version to 0.1.0-alpha.2 in all changelogs
- add release-plz migration markers to CHANGELOGs
- fix empty Rust code blocks in doc comments across workspace
- add missing doc comments for public API modules and types
- *(utils)* add missing public API documentation
- add missing documentation for public items across crates
- update version references to v0.2.0-rc.2
- update version references to v0.2.0-rc.3
- update version references to v0.2.0-rc.4
- update version references to v0.2.0-rc.5
- *(release)* finalize 0.2.0 changelog
- *(release)* refine 0.2.0 changelog narrative
- update version references to v0.2.0-rc.6
- *(release)* fold crate rc6 changelogs into stable notes
- update version references to v0.2.0-rc.7
- finalize 0.2.0 release metadata
- update version references to v0.3.0-rc.6
- update version references to v0.3.0-rc.7
- consolidate 0.3.0 changelog entries
- remove prerelease changelog sections
- update version references to v0.3.1
- update version references to v0.3.2
- update version references to v0.3.3
- update version references to v0.3.4
- update version references to v0.3.5
- update version references to v0.3.6
- update version references to v0.3.7
- update version references to v0.3.8
- update version references to v0.3.13
- update version references to v0.3.14
- update version references to v0.3.15
- update version references to v0.3.16
- update version references to v0.4.0-alpha.15

### Fixed

- *(crypto)* Adapt to sha2 and aes-gcm API changes
- Apply clippy automatic fixes across workspace
- *(logging)* Change Handler storage to Arc and avoid holding lock during async operations
- *(cache)* Add wait time before cleanup in expiration test
- *(utils)* remove unused dev-dependencies to break circular publish chain
- *(utils)* break circular publish dependency with reinhardt-test
- *(utils)* use fully qualified Result type in poll_until helpers
- *(utils)* add path validation to all LocalStorage methods
- *(utils)* escape HTML in linebreaks/linebreaksbr and fix strip_tags
- *(utils)* add security feature dependency for strip_tags_safe
- *(tests)* resolve miscellaneous unit test failures across crates
- *(meta)* fix workspace inheritance and authors metadata
- *(staticfiles)* unify manifest.json format to use "paths" key
- *(utils)* capitalize only first character in capfirst function
- *(release)* bump all crates to v0.1.0-rc.6 to skip yanked reinhardt-query-macros rc.5
- *(ci)* recover develop release-plz prerelease
- *(staticfiles)* inject wasm loader for directory index
- *(staticfiles)* preserve raw index in non-spa mode
- *(staticfiles)* inject wasm loader for directory index without spa mode
- *(build)* address CodeRabbit review feedback
- *(build)* port strict hot patch regression assertion
- *(ci)* pin broken upstream transitive releases
- *(ci)* pin brotli allocator dependency
- *(utils)* verify vendor asset integrity before install
- *(staticfiles)* normalize passthrough prefix matching
- *(staticfiles)* harden passthrough prefix normalization
- *(utils)* confine generated Azure blob URLs
- *(utils)* confine local storage paths
- *(utils)* allow benign double-dot filenames
- *(storage)* close local path races
- *(storage)* retain local root capability
- *(utils)* gate async filesystem import
- *(utils)* avoid unused filesystem import in native tests
- *(deps)* constrain incompatible AWS Smithy releases

### Maintenance

- *(workspace)* Add repository.workspace to 37 Cargo.toml files
- *(changelog)* Initialize CHANGELOG.md for all crates
- *(metadata)* Add description field to all crate manifests
- Apply code cleanup across workspace - clippy warnings and unused code removal
- *(release)* Prepare for alpha release - update all versions to 0.1.0-alpha.1
- *(cache)* Add missing dev-dependencies for cache integration tests
- *(deps)* Update Cargo.toml dependencies for promoted crates
- *(workspace)* Mark internal sub-crates as publish = false
- *(workspace)* Remove CHANGELOG files from unpublished sub-crates
- *(tests)* Remove unused Uri imports from test code across 12 files
- *(deps)* Add test dependencies to functional crates
- *(infra)* Update infrastructure crates dependencies
- *(utils)* remove integrated subcrate directories
- *(test,utils)* remove Redis Cluster support
- *(package)* replace version.workspace with explicit versions
- *(changelog)* remove obsolete [0.1.0] sections
- *(release)* bump reinhardt-utils to v0.2.0-alpha.1
- *(release)* correct version to 0.1.0-alpha.2 for changed crates
- *(release)* bump affected crates to v0.1.0-alpha.3
- merge main into chore/release-plz-migration
- *(release)* migrate all crates from alpha to 0.1.0-rc.1
- merge main into develop/0.3.0
- finalize 0.3.0 release metadata
- bind storages to 0.3.0 release group
- merge main into PR branch
- chore!(sync): merge main into develop/0.4.0

### Other

- Upgrade outdated dependencies to the latest versions
- resolve conflict with main (criterion version)

### Performance

- trim standard facade feature dependencies
- *(commands)* notify browsers after hot reload rebuilds
- *(build)* measure cold workspace build

### Reverted

- undo release PR [[#215](https://github.com/kent8192/reinhardt-web/issues/215)](https://github.com/kent8192/reinhardt-web/issues/215) version bumps

### Security

- *(utils)* fix XSS in error pages and media rendering, harden cache
- *(utils)* replace recursive cleanup with bounded iterative loop
- *(utils)* replace blocking KEYS with non-blocking SCAN+UNLINK
- *(utils)* recover from poisoned mutex/rwlock instead of panicking
- *(utils)* add cancellation mechanism for auto-cleanup tasks
- *(utils)* replace legacy Azure SDK staticfiles backend
- *(utils)* reject Azure blob dot segments

### Styling

- *(workspace)* Format codebase with cargo fmt
- Apply trunk fmt code formatting across entire workspace
- Apply cargo fmt across entire workspace
- Apply cargo fmt to clippy automatic fixes across workspace
- Format multi-line closures and method chains for readability
- *(auth,orm,parsers,tests)* Apply cargo fmt automatic formatting fixes
- *(crates)* Add todo!() markers for unimplemented features
- *(workspace)* Apply rustfmt and clippy auto-fixes across test files and build scripts
- Apply automatic code formatting
- Apply automatic code formatting
- sort imports alphabetically in static files modules
- fix let-chain formatting in PathResolver
- *(utils)* standardize doc comment formatting
- apply rustfmt and clippy auto-fixes across workspace
- apply rustfmt formatting to workspace files
- apply rustfmt to pre-existing formatting violations in 16 files
- apply workspace-wide formatting fixes

### Testing

- *(utils/logging)* Add logging tests
- *(utils)* Add proptest regression files
- *(urls,utils)* Add strict validation in URL proxy and logging tests
- *(deps)* Add missing test dependencies across workspace
- *(cache)* Move cache pubsub tests to integration test suite
- *(cache/in_memory)* Replace sleep-based TTL tests with poll_until
- *(cache/layered)* Replace sleep-based TTL tests with poll_until
- *(cache)* Replace sleep-based tests with poll_until in backend modules
- *(logging)* Improve assertion strictness and remove obsolete sleeps
- *(static/caching)* Replace loose assertions with exact match validation
- *(cache)* Add explicit wait for expiration in cleanup test
- *(cache)* Add wait time before TTL cleanup to stabilize test
- *(migration)* Remove old test files for reorganization into focused integration tests
- *(integration)* Add comprehensive integration tests organized by feature domain
- *(remaining)* Update remaining test files for code quality standards
- Update integration tests across all modules
- *(logging)* Adjust logging database integration test configuration
- *(static)* Add template integration tests for static utilities
- *(utils)* add UTF-8 multibyte truncation boundary regression tests for [[#762](https://github.com/kent8192/reinhardt-web/issues/762)](https://github.com/kent8192/reinhardt-web/issues/762)

## [0.4.0-alpha.15](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.4.0-alpha.14...reinhardt-utils@v0.4.0-alpha.15) - 2026-09-10

### Documentation

- update version references to v0.3.14
- update version references to v0.3.15
- update version references to v0.3.16

### Fixed

- *(utils)* confine local storage paths
- *(utils)* allow benign double-dot filenames
- *(storage)* close local path races
- *(storage)* retain local root capability
- *(utils)* gate async filesystem import
- *(utils)* avoid unused filesystem import in native tests

### Maintenance

- merge main into PR branch
- chore!(sync): merge main into develop/0.4.0

## [0.4.0-alpha.13](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.4.0-alpha.12...reinhardt-utils@v0.4.0-alpha.13) - 2026-09-02

### Fixed

- *(utils)* confine local storage paths
- *(utils)* allow benign double-dot filenames
- *(storage)* close local path races
- *(storage)* retain local root capability
- *(utils)* gate async filesystem import
- *(utils)* qualify native filesystem test helper

## [0.4.0-alpha.11](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.4.0-alpha.10...reinhardt-utils@v0.4.0-alpha.11) - 2026-08-27

### Documentation

- update version references to v0.3.13

### Fixed

- *(utils)* confine generated Azure blob URLs

### Maintenance

- merge main into develop/0.4.0

### Security

- *(utils)* reject Azure blob dot segments

## [0.4.0-alpha.9](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.4.0-alpha.8...reinhardt-utils@v0.4.0-alpha.9) - 2026-08-23

### Documentation

- update version references to v0.3.9
- update version references to v0.3.10

### Fixed

- *(staticfiles)* keep Azure deletion idempotent

### Maintenance

- merge main into develop/0.4.0

### Security

- *(utils)* replace legacy Azure SDK staticfiles backend

## [0.3.16](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.3.15...reinhardt-utils@v0.3.16) - 2026-09-08

### Maintenance

- update Cargo.toml dependencies

## [0.3.15](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.3.14...reinhardt-utils@v0.3.15) - 2026-09-01

### Fixed

- *(utils)* confine local storage paths
- *(utils)* allow benign double-dot filenames
- *(storage)* close local path races
- *(storage)* retain local root capability
- *(utils)* gate async filesystem import
- *(utils)* avoid unused filesystem import in native tests

### Maintenance

- merge main into PR branch

## [0.3.14](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.3.13...reinhardt-utils@v0.3.14) - 2026-08-29

### Maintenance

- update Cargo.toml dependencies

## [0.3.13](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.3.12...reinhardt-utils@v0.3.13) - 2026-08-27

### Fixed

- *(utils)* confine generated Azure blob URLs

### Security

- *(utils)* reject Azure blob dot segments

## [0.3.12](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.3.11...reinhardt-utils@v0.3.12) - 2026-08-25

### Maintenance

- update Cargo.toml dependencies

## [0.3.11](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.3.10...reinhardt-utils@v0.3.11) - 2026-08-24

### Maintenance

- update Cargo.toml dependencies

## [0.4.0-alpha.7](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.4.0-alpha.6...reinhardt-utils@v0.4.0-alpha.7) - 2026-08-19

### Documentation

- update version references to v0.3.3
- update version references to v0.3.4
- update version references to v0.3.5
- update version references to v0.3.6
- update version references to v0.3.7
- update version references to v0.3.8

### Maintenance

- merge main into develop/0.4.0

## [0.4.0-alpha.6](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.4.0-alpha.5...reinhardt-utils@v0.4.0-alpha.6) - 2026-08-06

### Documentation

- *(release)* restore coherent alpha.3 references

## [0.4.0-alpha.3](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.4.0-alpha.2...reinhardt-utils@v0.4.0-alpha.3) - 2026-07-27

### Fixed

- *(commands)* resolve static manifest aliases
- *(staticfiles)* harden manifest alias handling

## [0.4.0-alpha.1](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.3.2...reinhardt-utils@v0.4.0-alpha.1) - 2026-07-21

### Fixed

- *(release)* restore develop prerelease lifecycle

### Maintenance

- merge main into develop/0.4.0
- merge develop/0.4.0 into remove-anyhow branch
- merge develop/0.4.0 into anyhow removal branch
## [0.3.10](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.3.9...reinhardt-utils@v0.3.10) - 2026-08-22

### Maintenance

- update Cargo.toml dependencies

## [0.3.9](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.3.8...reinhardt-utils@v0.3.9) - 2026-08-21

### Security

- *(utils)* replace legacy Azure SDK staticfiles backend

## [0.3.8](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.3.7...reinhardt-utils@v0.3.8) - 2026-08-16

### Maintenance

- update Cargo.toml dependencies

## [0.3.7](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.3.6...reinhardt-utils@v0.3.7) - 2026-08-12

### Maintenance

- update Cargo.toml dependencies

## [0.3.6](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.3.5...reinhardt-utils@v0.3.6) - 2026-08-04

### Maintenance

- update Cargo.toml dependencies

## [0.3.5](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.3.4...reinhardt-utils@v0.3.5) - 2026-08-02

### Maintenance

- update Cargo.toml dependencies

## [0.3.4](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.3.3...reinhardt-utils@v0.3.4) - 2026-07-30

### Maintenance

- update Cargo.toml dependencies

## [0.3.3](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.3.2...reinhardt-utils@v0.3.3) - 2026-07-28

### Maintenance

- update Cargo.toml dependencies

## [0.3.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.2.0...reinhardt-utils@v0.3.0) - 2026-06-28

Stable release of `reinhardt-utils` for the Reinhardt 0.3.0 line. This
entry consolidates the 0.3.0 release-candidate series into one
stable release section.

### Migration Notes

- Review the root CHANGELOG and `instructions/MIGRATION_0.3.md` before upgrading from 0.2.x.

### Fixed

- *(staticfiles)* normalize passthrough prefix matching
- *(staticfiles)* harden passthrough prefix normalization
- *(utils)* verify vendor asset integrity before install
- *(ci)* pin brotli allocator dependency

### Maintenance

- merge main into develop/0.3.0

## [0.2.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.1.3...reinhardt-utils@v0.2.0) - 2026-06-11

Stable release of `reinhardt-utils` for the Reinhardt 0.2.0 line. This
entry consolidates the 0.2.0 release-candidate series into one
stable release section.

### Migration Notes

- Review the breaking changes listed below before upgrading from 0.1.x.
- See the root CHANGELOG and `instructions/MIGRATION_0.2.md` for cross-crate migration guidance.

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates

### Fixed

- *(build)* port strict hot patch regression assertion
- *(staticfiles)* inject wasm loader for directory index
- *(staticfiles)* preserve raw index in non-spa mode
- *(staticfiles)* inject wasm loader for directory index without spa mode

- *(ci)* pin broken upstream transitive releases
- *(build)* address CodeRabbit review feedback
- *(ci)* recover develop release-plz prerelease

### Performance

- *(commands)* notify browsers after hot reload rebuilds
- *(build)* measure cold workspace build
- trim standard facade feature dependencies

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-utils@v0.1.0-rc.30...reinhardt-utils@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-utils` as part of the
reinhardt-web 0.1.0 release. This crate is the workspace's shared
utility surface: the `staticfiles` subsystem, the `AppVendorAsset`
type that drives CDN-style vendor delivery, path / HTML sanitization
helpers consumed by every other Reinhardt crate, and lock-poisoning
recovery wrappers that replace `unwrap()` on `Mutex` / `RwLock` guards.

For the workspace-wide release narrative (Highlights, Breaking
Changes, Migration Guide), see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is preserved in the
[Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **`staticfiles` subsystem with WASM auto-injection** — `StaticFilesConfig`
  collects per-app static directories under `STATIC_ROOT`, detects
  WASM entry points (`WasmEntry`), and auto-injects the SPA bootstrap
  script into the served HTML so applications using `reinhardt-pages`
  do not need to hand-write `<script>` tags. Bundle assets disable
  immutable cache headers in debug builds so HMR can replace them.
- **`passthrough_prefixes` and `index_file`** — `StaticFilesConfig`
  exposes `passthrough_prefixes` (validated as non-empty at builder
  time) and `index_file` so the runserver / collectstatic CLIs can
  serve SPA-style fallback files from external paths via
  `serve_direct_file`.
- **`AppVendorAsset` vendor delivery** — apps register CDN assets
  with SHA-256 integrity, path validation, and an async downloader
  that uses a lazy first-request guard. `inventory`-based query
  helpers let the admin crate enumerate every registered asset for
  `collectstatic`.
- **Path / URL / HTML sanitization helpers** — `is_safe_url`
  (with anchor-link support), path-traversal prevention on
  `LocalStorage`, redirect-URL validators, XSS-safe `format_html`,
  `linebreaks` / `linebreaksbr` / `strip_tags_safe` HTML escapers,
  and UTF-8-safe truncation that never panics on a multibyte
  boundary.
- **Numeric & lock-safety utilities** — checked arithmetic helpers
  for cursor encoding (preventing arithmetic underflow), DST-gap
  handling in `make_aware_local` without panic, bounded iterative
  cleanup replacing recursive variants, and `poll_until` helpers
  used in lieu of `reinhardt-test` from inside `reinhardt-utils` to
  avoid a circular publish chain.
- **Lock-poisoning recovery wrappers** — `RwLock` / `Mutex`
  helpers in `reinhardt-utils` centralize poison recovery (replace
  blocking `KEYS` with non-blocking `SCAN+UNLINK`, recover from
  poisoned guards instead of panicking) and back the same patterns
  that `reinhardt-core` and `reinhardt-conf` expose.
- **UUID v7 throughout** — every UUID generated inside Reinhardt
  flows through helpers in this crate and is emitted as UUID v7 for
  monotonic ordering.

### Notable Breaking Changes

- **`r#static` module renamed to `staticfiles`** (#114) — the
  module rename removes the raw-identifier prefix from
  `reinhardt_utils::r#static`; the feature flag is also renamed
  from `static` to `staticfiles`.

### Migration Notes

See the [root Migration Guide](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#migration-guide)
for project-wide guidance. The only utility-specific move is to
update imports:

- Replace `reinhardt_utils::r#static::*` with
  `reinhardt_utils::staticfiles::*`, and switch the feature flag
  from `static` to `staticfiles`.
