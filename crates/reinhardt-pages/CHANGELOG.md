# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `callback_with_deps` internal helper backing `use_callback` /
  `use_callback_with`. Maintains stable `Arc<dyn Fn>` identity across
  re-entries at the same call site while listed deps are unchanged,
  matching React `useCallback`.
- "Reactivity semantics" rustdoc section on 8 closure-taking hooks
  (`use_reducer`, `use_action`, `use_action_state`,
  `use_sync_external_store{,_with_server}`, `use_transition`,
  `use_debug_value_with`, `use_websocket`) documenting that their
  closures run outside any active Observer by construction.
- Cross-hook integration test suite at
  `tests/hooks_deps_integration.rs` covering Memo→Effect propagation,
  unlisted-Signal isolation, cleanup ordering, and empty-deps
  mount-only behavior.
- Scaffolded `page!` macro pre-codegen pass `hook_deps_validator` (in
  the `macros` crate). Full Signal-read detection against Manouche's
  `PageBody` AST is deferred to a follow-up issue; the runtime contract
  is already enforced by the `*::new_with_deps` constructors.
- `ServerFnMetadata` cross-target supertrait carrying `PATH`, `NAME`,
  `CODEC`, and `INJECTED_PARAMS` for every `#[server_fn]`. Available
  on both native and wasm targets without any feature flag, so a
  `#[url_patterns(mode = unified)]` aggregator can name
  `my_fn::marker` from a closure body that compiles on either side
  of the cfg boundary ([#4711](https://github.com/kent8192/reinhardt-web/issues/4711)).
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
- `form!` macro fields now accept optional generic type parameters
  (`HiddenField<i64>`, `ChoiceField<bool>`, `MultipleChoiceField<String>`,
  `JsonField<MyStruct>`) to forward typed values to `#[server_fn]` handlers
  instead of always stringifying them (#4397)
- `IpAddressField` is now specialized to `Option<IpAddr>` in generated code
- Fields without a generic parameter default to `String` for backward
  compatibility
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
- `use_resource(fetcher, deps)` — unified async data-fetching hook, the
  resource counterpart of `use_effect`. `()` deps fetch once on mount; listed
  `Trackable` deps (`Signal` / `Memo` / `Resource`) drive automatic refetch.
  Available on all targets: the native/SSR path renders the `Loading` state and
  the client performs the real fetch after hydration, mirroring `use_action`.
  Supersedes the deprecated `create_resource` / `create_resource_with_deps`.

### Changed (BREAKING)

- `use_effect`, `use_layout_effect`, `use_memo`, `use_callback`, and
  `use_callback_with` now take an explicit dependency tuple as the second
  argument, aligning with React.js semantics (#4195, Manouche v2 Layer ②).
  The closure type for `use_effect` / `use_layout_effect` is now
  `FnMut() -> Option<C>` so cleanup functions can be returned, matching
  React `useEffect(() => () => cleanup())`. Pass `()` for mount-only.
- Hook closures now run with no active reactive Observer ("Option A"),
  so `Signal::get` inside the closure does NOT auto-subscribe.
  Subscriptions derive exclusively from the deps tuple.
- `impl Trackable for Resource<T, E>` lets `Resource` participate in
  hook deps tuples alongside `Signal` and `Memo`.
- `ServerFnRegistration` (native) and `MockableServerFn` (msw) now
  extend `ServerFnMetadata` instead of declaring `PATH`, `NAME`, and
  `CODEC` themselves. Existing consumers reach the constants through
  supertrait inheritance with no source change required
  ([#4711](https://github.com/kent8192/reinhardt-web/issues/4711)).
- **BREAKING**: `page!` macro now unconditionally wraps every `{expr}` and
  every `if` / `for` / `match` control-flow block in `Page::reactive(move || ...)`.
  Helper-routed Signal reads (`{helper(&signal)}`) re-render correctly without
  any opt-in. Spec §4.1. Resolves #4515.
- **BREAKING**: `page!` body identifiers must be declared in the closure
  parameter list. Implicit captures of outer Signal bindings are a hard
  compile error. Spec §3.7. Migration: pass the binding as a closure param
  or qualify free function calls with `self::` so the path is multi-segment.
- **BREAKING**: `page!` no longer accepts bare-identifier shorthand in
  element bodies (`div { name }`). Always use the explicit braced form:
  `div { {name} }`. Spec §3.6. Migration: codemod `cargo make
  migrate-manouche-v2` (PR3) handles this mechanically.

### Deprecated

- `use_effect_event` and `use_effect_event_with` are deprecated.
  Option A semantics make them structurally redundant — the wrapped
  closure of `use_effect`/`use_layout_effect` already runs without
  auto-tracking. Use `use_callback(f, deps)` for stable identity, or
  read latest values via `.get_untracked()` inside the closure.
  Scheduled for removal in v0.3.0.
- `create_resource` and `create_resource_with_deps` are deprecated in favor of
  the unified, cross-target `use_resource(fetcher, deps)` (`()` deps = fetch
  once; `(signal,)` deps = refetch on change). Thin forwarding shims are kept;
  scheduled for removal in v0.3.0.

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

### Fixed

- Resource dependency-change refetch now actually fires. The old
  `create_resource_with_deps` dropped its tracking `Effect` handle immediately
  after creation, so the `Effect` was disposed and never re-ran — automatic
  refetch on dependency change silently never happened (and the only covering
  test was excluded on every target by a contradictory `cfg`). The unified
  `use_resource` stores the `Effect` inside the returned `Resource` so it stays
  alive for the Resource's lifetime, and applies the `defer_yield` microtask
  deferral (#3316) on the dependency-driven path as well, not only the
  fetch-once path.

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
