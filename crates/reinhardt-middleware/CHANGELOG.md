# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0-rc.2](https://github.com/kent8192/reinhardt-web/compare/reinhardt-middleware@v0.1.3...reinhardt-middleware@v0.2.0-rc.2) - 2026-06-03

### Added

- *(middleware)* [**breaking**] migrate session DI from Arc<SessionStore> to Depends<SessionStore>
- *(middleware)* bridge CorsConfig to CorsSettings fragment

### Changed

- [**breaking**] align develop/0.2.0 with main, preserving 8 feature crates
- *(conf)* delete deprecated Settings, AdvancedSettings, JsonFileSource, and related APIs
- *(middleware)* [**breaking**] replace deprecated SecurityConfig with builder pattern on SecurityMiddleware

### Fixed

- *(ci)* recover develop release-plz prerelease
- *(auth)* [**breaking**] migrate internal consumers from removed User/SimpleUser types
- *(auth)* address CodeRabbit review feedback
- *(docs)* resolve rustdoc intra-doc link errors on develop/0.2.0
- *(docs)* resolve additional rustdoc link errors for feature-gated types
- complete downstream shielding for deprecated config re-exports

### Maintenance

- *(examples)* remove examples-twitter

### Styling

- apply formatter fixes across workspace

### Changed

- [**breaking**] Migrate session DI resolution from `Arc<SessionStore>` to the
  canonical `Depends<SessionStore>` shape, aligning session middleware with the
  rest of reinhardt-di's `#[inject] Depends<T>` direction. See
  [#4437](https://github.com/kent8192/reinhardt-web/issues/4437).
  - `SessionMiddleware::di_registrations` now keys on `TypeId::of::<SessionStore>()`
    (previously `TypeId::of::<Arc<SessionStore>>()`), and stores the
    `Arc<SessionStore>` verbatim (no double-Arc envelope).
  - The `SessionStoreRef` newtype wrapper has been **removed**. Handlers that
    previously took `#[inject] store: SessionStoreRef` must now take
    `#[inject] store: Depends<SessionStore>`. Call sites for `SessionAuthExt`
    (`session.login(&store, …)`, `session.logout(&store)`) continue to work
    unchanged thanks to `Depends<T>: Deref<Target = T>` and Rust deref coercion.
  - `SessionAuthExt::login` / `SessionAuthExt::logout` now take `&SessionStore`
    directly instead of `&SessionStoreRef`.
  - **Migration**: replace `#[inject] store: SessionStoreRef` with
    `#[inject] store: Depends<SessionStore>` and update imports from
    `reinhardt::middleware::session::SessionStoreRef` to
    `reinhardt::middleware::session::SessionStore` + `reinhardt::di::Depends`.
    Manual DI setup that called `singleton.set::<Arc<SessionStore>>(store)`
    should call `singleton.set_arc(store)` instead.

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-middleware@v0.1.0-rc.30...reinhardt-middleware@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-middleware` as part of the
reinhardt-web 0.1.0 release. Ships the canonical, batteries-included
HTTP middleware set: sessions, authentication, CORS, CSRF, security
headers, gzip, rate limiting, broken-link tracking, and more — every
component plugs into the request-scoped DI container via
`Middleware::di_registrations`.

For the workspace-wide release narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is in the [Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **Pluggable session middleware** — `SessionMiddleware` auto-registers
  its `Arc<SessionStore>` through `di_registrations`, supports
  pluggable storage via the `AsyncSessionBackend` trait (with a
  Redis-backed `RedisSessionBackend` behind the `session-redis`
  feature), exposes typed `SessionValue<T>` / `OptionalSessionValue<T>`
  / `SessionValueNamed` / `OptionalSessionValueNamed<K, T>` extractors
  with Path-style auto-extraction, and propagates handler-side session
  ID rotation into `Set-Cookie`.
- **Authentication middleware family** — `JwtAuthMiddleware` for
  stateless tokens, `CookieSessionAuthMiddleware` for session cookies,
  `RemoteUserMiddleware` / `PersistentRemoteUserMiddleware` for proxy
  trust, and `LoginRequiredMiddleware` for unauthenticated redirect.
  JWT `Claims` carries `is_staff` / `is_superuser` so admin layers can
  authorize without an extra round-trip.
- **CSRF, CORS, CSP, and clickjacking protection** —
  `OriginGuardMiddleware` for CSRF, `CspMiddleware` with per-path
  `exempt_paths`, hardened `X-Frame-Options`, hardened CORS origin
  validation against the request per the Fetch Standard, and a `Vary`
  header injection when wildcard origins are combined with credentials.
  Handler-set CSP headers are respected. Cryptographic-random nonces
  and CSRF fallback secrets replace previous predictable values.
- **Operational middleware** — `gzip` encoding (asserted via typed
  handler tests), `HttpsRedirectMiddleware` with allowed-host
  validation, rate limiting with sliding-window statistics and stale
  bucket eviction, and `BrokenLinkConfig` driven from settings via
  `BrokenLinkConfig::from_settings` (no more env-var hot path).
- **Settings + DI integration** — Middleware reads configuration from
  the layered settings system rather than `env::var`, and the
  request-scoped session ID is injected into request extensions before
  the handler runs so downstream extractors see a consistent view.
- **UUIDv7 across security tokens** — Generation migrated from UUIDv4
  to UUIDv7 for ordered IDs; v4 is retained for security-sensitive
  tokens where unpredictability matters.

### Notable Breaking Changes

`SessionData` is `#[non_exhaustive]` to prevent source-breaking field
additions, and several middleware accessors gained DI-aware variants.
For the cross-crate view (including DI unification on `Depends<T>`)
see the [Breaking Changes Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/breaking-changes).

### Migration Notes

This is the first stable release, so there is no prior stable version
to migrate from. See the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22)
for the cross-crate migration guide.
