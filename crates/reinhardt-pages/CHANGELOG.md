# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **BREAKING**: `page!` no longer accepts bare-identifier shorthand in
  element bodies (`div { name }`). Always use the explicit braced form:
  `div { {name} }`. Spec §3.6. Migration: codemod `cargo make
  migrate-manouche-v2` (PR3) handles this mechanically.

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-pages@v0.1.0-rc.30...reinhardt-pages@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-pages` as part of the
reinhardt-web 0.1.0 release. `reinhardt-pages` provides the
WASM-based frontend framework with a Django-like API: declarative
`page!` / `head!` / `form!` DSLs, a reactive `Signal<T>` runtime,
a SPA router and `ClientLauncher`, and `#[server_fn]` RPC stubs
shared with the server-side handler.

For the workspace-wide release narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is in the [Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **`page!` declarative DSL** — anonymous component bodies with
  70+ compile-time-validated HTML elements, closure-style props,
  `@event` handlers, `if`/`else`/`for` control flow, reactive
  `watch` blocks driven by `Signal<T>`, and accessibility checks
  (img alt, button labels).
- **`head!` macro for SSR metadata** — title / meta / link /
  script / style elements with SSR head injection.
- **WASM SPA launcher** — `ClientLauncher` builder with built-in
  document-level link interception (opt out with
  `.intercept_links(false)`), `before_launch` / `after_launch`
  lifecycle hooks, `on_path` / `on_path_pattern` declarative
  effects with `PathCtx` portals, and JWT token management
  primitives for authenticated WASM clients.
- **Explicit navigation subscription API** — `Router::on_navigate`
  delivers a synchronous callback after every successful `push` /
  `replace` and on `popstate`, decoupled from Signal / Effect
  auto-tracking; `use_router()` hook, `RouterHandle`, free
  `navigate()`, and `try_with_spa_router` round out the SPA
  navigation surface.
- **Server Functions with DI** — `#[server_fn]` generates both the
  WASM client stub and the server-side handler, with JSON / URL /
  msgpack codecs, per-request DI context forking, `FromRequest`
  extractor support, CSRF auto-pass, and HTTP status code
  preservation for DI auth errors.
- **HMR (opt-in `hmr` feature)** — file-watcher-driven CSS / DOM
  updates pushed over a WebSocket scheme that is auto-selected
  based on `window.location.protocol` (secure variant on HTTPS).

### Notable Breaking Changes

- **`ClientLauncher::on_navigate`** ([#4117](https://github.com/kent8192/reinhardt-web/discussions/4117)) — SPA navigation routes through `Router::on_navigate` instead of relaunching the client per navigation; remove any manual `ClientLauncher::launch` wiring tied to the old model.
- **`client_router::history` dedup** ([#4219](https://github.com/kent8192/reinhardt-web/discussions/4219)) — the duplicate `history` module under `client_router` is removed; consume `pages::router::history` (or the canonical `reinhardt-urls` `client_router::history` on WASM) instead.
- **`pages::Router` deprecated** in favour of `urls::ClientRouter`; migration is summarised in [#4234](https://github.com/kent8192/reinhardt-web/issues/4234).
- **`form! on_success:` closure lift** ([#4624](https://github.com/kent8192/reinhardt-web/issues/4624)) — type-annotated `on_success: |value: T| ...` closures now lift to the outer scope (allowing route-parameter capture) and require `Send + Sync`; unannotated closures (`|value|`, `|_value|`) keep the historical inline emit.
- **MSW mocks replace `MockFetch`** — `MockFetch` and `mock_server_fn` are deprecated; opt into MSW-style mocking via the `msw` feature and the generated `MockableServerFn` trait.

### Migration Notes

- Replace `pages::Router` consumers with `urls::ClientRouter`; the
  former is `#[deprecated]` and downstream `#[allow(deprecated)]`
  attribute may be needed during the transition.
- Move ad-hoc SPA link interception into the built-in interceptor
  by removing your handler and letting `ClientLauncher::launch`
  install one by default — or pass `.intercept_links(false)` to
  preserve the previous behaviour.
- For dynamic `form! { on_success: |value: T| ... }` callbacks that
  need to capture an enclosing `qid` or other route parameters,
  add an explicit type annotation on the closure parameter.
- For the workspace-wide migration narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
