# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `ClientRouter::page<F, P>(pattern, handler)` and
  `ClientRouter::named_page<F, P>(name, pattern, handler)` accepting any
  handler `Fn(P) -> Page` where `P: FromRequest`. The same Props struct
  can be used both as a Component prop bag (Manouche DSL v2 spec §4.3)
  and as a page function — "every page is a component." Path / query
  extraction errors surface as a `Page::Text` at the router boundary
  rather than panicking. (Refs #4668)
- `reinhardt_urls::routers::client_router::from_request` module
  exposing `FromRequest`, `RouteContext`, `ExtractError`,
  `PathParam<T>`, and `QueryParam<T>` — the manual building blocks for
  `ClientRouter::page` handlers. `#[derive(FromRequest)]` and
  `#[derive(PageProps)]` proc-macros are deferred to spec §10.
- `ClientRouteMatch::query: Option<String>` — populated by `match_path`
  after stripping an optional `?query` suffix from the path before
  pattern matching. Required for `QueryParam<T>` to see the query under
  real routing.
- `ParamContext::with_query(...)` / `ParamContext::query()` /
  `ParamContext::params()` — `render_current` threads the captured
  query through to the `RouteHandler` trait. Backward-compatible:
  existing `ParamContext::new(...)` keeps the previous signature and
  defaults the new field to `None`.

## [0.1.0](https://github.com/kent8192/reinhardt-web/compare/reinhardt-urls@v0.1.0-rc.30...reinhardt-urls@v0.1.0) - 2026-05-22

Initial stable release of `reinhardt-urls` as part of the reinhardt-web
0.1.0 release. Provides the URL routing core: the `#[url_patterns]`
consumer, `ServerRouter` / `ClientRouter` / `UnifiedRouter` builders,
typed reverse resolution, and the radix-based path matcher shared with
client-side WASM.

For the workspace-wide release narrative, see the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22).
Per-prerelease history is in the [Release Discussions](https://github.com/kent8192/reinhardt-web/discussions/categories/release).

### Capabilities at 0.1.0

- **Server, client, and unified routers** — `ServerRouter`,
  `ClientRouter`, and `UnifiedRouter` cover native, WASM, and hybrid
  setups; `Debug` is implemented for both server-side variants, and
  `UnifiedRouter` carries a WASM-friendly `ServerRouterStub` so
  `#[url_patterns(mode = unified | server)]` closures compile on WASM.
  Native `mount_unified` correctly merges child client routes.
- **Typed URL resolution** — Compile-time type-safe URL resolution via
  extension traits, name-alias support on `UrlReverser` and
  `ServerRouter`, and a client-side `ClientUrlReverser`. The
  `url-resolver` capability is part of the standard, `api-only`, and
  `urls-full` feature sets.
- **Async-capable `#[routes]`** — `#[routes]` accepts async handlers,
  and the radix-based pattern matcher surfaces insertion errors and
  exposes fallible reverse helpers. Route registration accumulates
  prefixes correctly, strips leading slashes on action `url_path`, and
  normalizes ViewSet prefixes to prevent triple slashes.
- **Reactive client navigation** — `ClientRouter` carries reactive
  navigation observation, `render_current()` returns a `Page`, and
  `Clone` is derived so route resolution composes inside reactive
  scopes. `pages::router::history` is the single source of truth (the
  duplicate `client_router::history` module was removed).
- **Middleware-aware route assembly** — `with_middleware` harvests
  middleware-contributed DI registrations, `group()` drains pending
  DI on grouped routers, and child routers receive `with_di_context`
  propagation. `exclude()` provides declarative route exclusion on the
  builder side.
- **Streaming and viewset integration** — `UnifiedRouter::mount_streaming()`
  registers streaming handlers, `StreamingTopicResolver` resolves
  topics from URL patterns, and `viewset_with_actions` bridges
  `#[viewset]` actions through `ServerRouter` / `RouteGroup` builders.
- **Defensive runtime** — Lock poisoning is recovered (no `unwrap()` on
  `RwLock` guards), `RwLock` guards are never held across `.await`
  points, the LRU route cache enforces memory-bounded eviction with
  periodic compaction, and path validation rejects traversal, ambiguous
  parameters, ReDoS-prone patterns, and parameter injection at compile
  time.

### Notable Breaking Changes

- **Typed `#[url_patterns]`** ([Discussion #3770](https://github.com/kent8192/reinhardt-web/discussions/3770))
  — Accepts `InstalledApp::*` identifiers with `mode = server | client
  | unified`; pattern functions are renamed accordingly.
- **`urls/` directory layout** ([Discussion #3918](https://github.com/kent8192/reinhardt-web/discussions/3918))
  — `ws_urls.rs` and friends move under `src/apps/<app>/urls/`.
- **`client_router::history` dedup** ([Discussion #4219](https://github.com/kent8192/reinhardt-web/discussions/4219))
  — The duplicate `history` module under `client_router` is removed;
  consume `pages::router::history` instead.
- **Apps relocation** ([Discussion #4476](https://github.com/kent8192/reinhardt-web/discussions/4476))
  — Per-app handlers move into `apps/<app>/`, which affects how URL
  patterns are declared and mounted.
- **Async `#[routes]`** — Route handler signatures may be `async fn`;
  sync ABIs remain supported but are no longer the canonical shape.
- **Radix insertion errors surface** — `#[routes]` no longer panics on
  pattern conflicts; fallible reverse helpers return `Result` values
  callers must handle.

### Migration Notes

See the [root CHANGELOG](https://github.com/kent8192/reinhardt-web/blob/main/CHANGELOG.md#010---2026-05-22)
for the consolidated migration guide. The most disruptive moves are
covered in Discussions [#3770](https://github.com/kent8192/reinhardt-web/discussions/3770),
[#3918](https://github.com/kent8192/reinhardt-web/discussions/3918),
[#4219](https://github.com/kent8192/reinhardt-web/discussions/4219),
and [#4476](https://github.com/kent8192/reinhardt-web/discussions/4476).
