# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

### Changed

- **BREAKING**: `use_effect`, `use_layout_effect`, `use_memo`, `use_callback`,
  and `use_callback_with` now require an explicit deps tuple as the final
  positional argument — exact React parity with `useEffect(fn, [deps])` etc.
  Mount-only is `()`. Missing deps becomes a hard compile error (E0061-style).
  Identity-based equality on `signal_id()` means no `T: Clone` requirement on
  the underlying signal value. The runtime gates re-runs so a closure that
  reads an unlisted signal does not re-execute when that signal changes.
  Spec §4.2. Migration via `cargo make migrate-manouche-v2` (PR3 codemod).
  Refs #4195.
- **BREAKING**: `page!` macro now unconditionally wraps every `{expr}` and
  every `if` / `for` / `match` control-flow block in `Page::reactive(move || ...)`.
  Helper-routed Signal reads (`{helper(&signal)}`) re-render correctly without
  any opt-in. Spec §4.1. Resolves #4515.
- **BREAKING**: `page!` body identifiers must be declared in the closure
  parameter list. Implicit captures of outer Signal bindings are a hard
  compile error. Spec §3.7. Migration: pass the binding as a closure param
  or qualify free function calls with `self::` so the path is multi-segment.

### Removed

- **BREAKING**: `watch { ... }` block is removed. The body of `watch` can be
  inlined as-is; the new auto-wrap subsumes it. The validator emits a
  pointer at the `cargo make migrate-manouche-v2` codemod when it sees a
  surviving `watch` block.

#### BREAKING CHANGES — Router Relocation Cleanup

**First of two PRs** removing reinhardt-pages's 16 RC-deprecated items per
STABILITY_POLICY § SP-4 (umbrella Issue
[#4520](https://github.com/kent8192/reinhardt-web/issues/4520)).
This PR removes the 8 router-relocation items (relocated to
`reinhardt_urls::routers` since `0.1.0-rc.27`); the remaining 8
items (App struct, launcher legacy, use_reducer migration, MSW
migration, CSRF auto-inject) require selective Edit and ship in
the follow-up `feat(pages)!:` PR.

Removed in this PR (8 items):

- **`src/router/core.rs`** (6 items, all deprecated `0.1.0-rc.27`,
  refs #4234 / cloud#578) — `PathError`, `RouterError`,
  `ClientRouteMatch` (RouteMatch), `ClientRoute` (Route),
  `ClientRouter` (Router), `NavigationSubscription`. All relocated
  to `reinhardt_urls::routers`.
- **`src/router/pattern.rs`** (1 item, `0.1.0-rc.27`) — `ClientPathPattern`
  (PathPattern). Use `reinhardt_urls::routers::ClientPathPattern`.
- **`src/router/params.rs`** (1 item, `0.1.0-rc.27`) — `Path` extractor.
  Use `reinhardt_urls::routers::Path`.

### Added

- `pub trait Trackable` in `reinhardt-pages::reactive` (re-exported as
  `reinhardt_pages::reactive::Trackable`). Implemented for `Signal<T>` and
  `Memo<T>`; consumed by the new auto-wrap visitor and the upcoming hook
  deps-tuple machinery (#4195).
- `NodeId::as_u64()` accessor in `reinhardt-core` so external callers (such
  as `Trackable::signal_id`) can obtain the underlying counter value.
- New `Component { prop: val, @event: handler, child_element { ... } }`
  invocation syntax inside `page!` bodies. Components are functions
  matching `fn <name>(props: <NameProps>) -> Page` where `<NameProps>`
  derives `bon::Builder`. The legacy positional form
  `{component_fn(args)}` continues to work unchanged. Spec §3.5.
- `bon` added as a `reinhardt-pages` runtime dependency. Staged for
  removal under spec §10 once `#[derive(PageProps)]` /
  `#[component]` proc-macros take over the prop-struct generation.
- `reinhardt_pages::router::request` submodule re-exports the
  Manouche DSL v2 spec §4.3 `FromRequest` building blocks
  (`FromRequest`, `RouteContext`, `ExtractError`, `PathParam<T>`,
  `QueryParam<T>`) from `reinhardt_urls::routers::client_router::from_request`
  so application code can write
  `use reinhardt_pages::router::request::FromRequest;` matching the
  spec's namespace. The legacy non-generic `PathParam` re-exported at
  `reinhardt_pages::router::PathParam` (deprecated since `0.1.0-rc.27`)
  is unrelated and remains in place during its deprecation cycle.
  (Refs #4668)

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
