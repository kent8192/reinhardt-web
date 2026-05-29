# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3](https://github.com/kent8192/reinhardt-web/compare/reinhardt-auth@v0.1.2...reinhardt-auth@v0.1.3) - 2026-05-29

### Documentation

- align documentation with current APIs
- fix version marker counts

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-auth@v0.1.0-rc.30...reinhardt-auth@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-auth` as part of the
reinhardt-web 0.1.0 release. This crate ships the
authentication / authorization surface — JWT, cookie sessions,
OAuth2 / OIDC, pluggable user managers, and Django-style
permission guards — that powers both `reinhardt-admin` and
end-user apps.

For the workspace-wide release narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is in the [Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **JWT, sessions, and storage backends** — `JwtSessionBackend`
  with a `JwtError` enum that rejects expired tokens by default,
  cookie session backend with HMAC signed via the server secret,
  `CacheSessionBackend`, a database token storage with an O(1)
  SHA-256 digest index, and pluggable session storage
  (file / cookie / `redis-sessions`).
- **OAuth2 and OIDC** — `GenericOidcProvider` for arbitrary OIDC
  IdPs (in addition to GitHub / Google / etc.), HTTPS enforced
  on every OAuth2 / OIDC endpoint URL, `client_id` validated
  against the authorization code on exchange, and a GitHub
  `/user` transform that normalises responses into
  `StandardClaims`.
- **User and superuser management** — `AuthIdentity` trait
  (replaces the deprecated `User` trait), `Group` and
  `AuthPermission` ORM models, `GroupManager` integrated with
  `PermissionsMixin`, `SuperuserInit` trait + `SuperuserCreator`
  registry with `inventory` auto-registration for
  `#[user(full = true)]` + `#[model]` types.
- **Extractors and DI** — `AuthInfo` lightweight extractor,
  `AuthUser<U>` tuple-struct extractor, `CurrentUser` DI binding,
  and a `validate_auth_extractors` startup validation pass that
  fails closed when a route declares `AuthUser<U>` without a
  matching DI binding.
- **Permission guards** — `Guard<P>` runtime type with `Public`,
  `All`, `Any`, `Not` combinators, plus a `guard!()` proc macro
  whose winnow-based parser compiles permission expressions at
  attribute-expansion time.
- **Hardened defaults** — argon2 password hashing, constant-time
  comparison everywhere a token meets a secret, session
  rotation on login to defeat session fixation, and TOTP
  algorithm / proxy trust hardening for SSO flows.

### Notable Breaking Changes

- **OAuth2 `exchange_code` redirect URI** ([#3609](https://github.com/kent8192/reinhardt-web/discussions/3609)) — `exchange_code()` now requires the callback URL as its fourth argument, so the IdP-side `redirect_uri` is verified server-side.
- **`User` trait deprecated** in favour of `AuthIdentity`; `DefaultUser` carries the same deprecation. Update extractors and ORM bindings to the new trait.
- **JWT default rejects expired tokens** — the `JwtError` enum surfaces `Expired` distinctly from `Invalid`; callers that previously swallowed expiry must update their error matching.
- **`#[user(...)]` macro contract change** — the macro now emits a `BaseUserManager` impl and requires an explicit `LABEL` on the `AppLabel` implementor; fixtures and integration tests that need to supply their own manager must opt out via the macro attribute.
- **`Mutex` migration** — internal `std::Mutex` replaced with `tokio::Mutex` to prevent async deadlocks; downstream code that held the lock across an `.await` should re-verify boundaries.

### Migration Notes

- Pass the `redirect_uri` as the fourth argument to every
  `OAuth2Provider::exchange_code` call (covered in
  [#3609](https://github.com/kent8192/reinhardt-web/discussions/3609)).
- Migrate `#[user]`-annotated types to declare an explicit
  `LABEL` constant on their `AppLabel` impl; integration test
  fixtures that previously relied on a manual manager should
  opt out of the auto-emitted `BaseUserManager` impl.
- Replace `Injected<T>` / `OptionalInjected<T>` in auth wiring
  with `Depends<T>` / `Option<Depends<T>>` per [#3631](https://github.com/kent8192/reinhardt-web/discussions/3631).
- For the workspace-wide migration narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
