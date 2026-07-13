# Typed `page!` Event Payloads Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` to execute this plan task by task.

**Goal:** Replace intrinsic `page!` raw events with one public payload type per
standard HTML/SVG event, preserve raw custom events, and make the same handlers
executable with meaningful event data in native component tests.

**Architecture:** A dependency-free `reinhardt-event-catalog` crate owns every
standard event name and its metadata. Core retains heterogeneous raw handlers
with `EventName` and owns the native raw transport. Pages generates public typed
wrappers and adapters from the catalog, while Manouche and the proc macro keep
intrinsic DOM events separate from component callback props.

**Tech Stack:** Rust 2024, proc macros with `syn`/`quote`, `web-sys`, native
component testing, `trybuild`, `rstest`, Cargo Make.

## Global Constraints

- Work only in branch `fix/issue-5563-typed-event-payloads` and its dedicated
  worktree.
- Target `develop/0.4.0`; do not rebase or force-push.
- Follow `docs/rfc/2026-07-10-typed-page-event-payloads-design.md` as the
  approved contract.
- Use test-driven development: add a failing focused test, observe the expected
  failure, implement the smallest coherent unit, and rerun the focused tests.
- Do not create a second event-name-to-payload table. Catalog metadata or its
  expansion macro must drive every consumer.
- Keep component `@event` props typed solely by component props.
- Do not add typed custom details; that work belongs to #5636.
- Keep code comments and public documentation in English. Add no `TODO`,
  `FIXME`, `todo!()`, `unimplemented!()`, or undocumented `#[allow]`.
- Use Rust 2024 `module.rs` plus `module/` layouts; never add `mod.rs`.
- Every task ends with spec review, code-quality review, focused verification,
  and a small conventional commit. Commit messages must end with:

  ```text
  Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
  ```

## Task 1: Add the authoritative event catalog

**Files:**

- Create: `crates/reinhardt-event-catalog/Cargo.toml`
- Create: `crates/reinhardt-event-catalog/README.md`
- Create: `crates/reinhardt-event-catalog/src/lib.rs`
- Modify: `Cargo.toml`

**Public contract:**

```rust
pub enum KnownEvent { /* generated variants */ }

pub enum EventName {
	Known(KnownEvent),
	Custom(Cow<'static, str>),
}

pub struct EventSpec {
	pub kind: KnownEvent,
	pub dom_name: &'static str,
	pub payload_name: &'static str,
	pub primary_interface: EventInterface,
	pub fallback_interfaces: &'static [EventInterface],
	pub capabilities: &'static [EventCapability],
	pub behavior: EventBehavior,
}

pub const EVENT_SPECS: &[EventSpec];
pub fn event_spec(name: &str) -> Option<&'static EventSpec>;
```

The crate must also provide a hidden exported callback macro that lets pages
declare all unique payload wrappers from the same catalog declaration. The
macro must pass consumer-neutral tokens and must not embed paths to core,
Manouche, pages, `syn`, or `quote`.

**Steps:**

1. Register an otherwise empty publishable leaf crate in workspace members and
   dependencies, using the same package metadata and explicit workspace
   dependency version as adjacent public Reinhardt crates.
2. Add tests for unique DOM names, unique payload names, lookup, exact
   case-sensitive SVG timing names, `KnownEvent` round trips, custom names, and
   interface/capability validity. Run `cargo test -p reinhardt-event-catalog`
   and confirm the expected failure because the catalog API is not implemented.
3. Implement the approved catalog: HTML handler names, composition, focus,
   pointer, touch, CSS animation/transition, fullscreen, selection,
   scrolling/rendering, encrypted media (`MediaEncryptedEvent` for
   `encrypted`, generic media event data for `waitingforkey`),
   picture-in-picture (`PictureInPictureEvent`), WebXR interception, and SVG
   animation timing.
4. Represent `click` and `input` interface fallbacks explicitly. Add
   `EventBehavior` defaults for `bubbles`, `cancelable`, and `composed`.
5. Run:

   ```bash
   cargo test -p reinhardt-event-catalog
   cargo tree -p reinhardt-event-catalog
   cargo fmt --all --check
   ```

6. Commit as `feat(pages): add authoritative event catalog` with `Fixes #5563`.

## Task 2: Replace the core event name and native raw transport

**Files:**

- Modify: `crates/reinhardt-core/Cargo.toml`
- Modify: `crates/reinhardt-core/src/types/page.rs`
- Modify: `crates/reinhardt-core/src/types/page/event.rs`
- Create: `crates/reinhardt-core/src/types/page/native_event.rs`
- Modify: `crates/reinhardt-pages/src/component/into_page.rs`
- Modify: `crates/reinhardt-pages/src/component/reactive_if.rs`
- Modify: `crates/reinhardt-pages/src/component.rs`
- Modify: `crates/reinhardt-pages/src/lib.rs`
- Modify: `crates/reinhardt-pages/src/platform/native.rs`
- Modify: `crates/reinhardt-pages/src/callback.rs`
- Modify: `crates/reinhardt-pages/src/reactive/hooks/memo.rs`
- Modify: `crates/reinhardt-pages/src/reactive/hooks/async_action.rs`
- Modify: `crates/reinhardt-pages/src/hydration/events.rs`
- Modify: `crates/reinhardt-pages/src/hydration/runtime.rs`
- Modify: `crates/reinhardt-pages/src/testing/component/events.rs`
- Modify: `crates/reinhardt-pages/macros/src/form/codegen.rs`
- Modify: `crates/reinhardt-pages/macros/src/page/codegen.rs`
- Modify: `crates/reinhardt-pages/tests/component_system_integration.rs`
- Modify: component-prop UI fixtures returned by `rg -n '\bDummyEvent\b'`

**Public contract:**

```rust
pub use reinhardt_event_catalog::KnownEvent as EventType;

#[cfg(native)]
pub struct NativeEvent { /* owned snapshots and shared dispatch state */ }

#[cfg(native)]
pub enum NativeEventPayload { /* one variant per interface family */ }

#[cfg(native)]
pub struct NativeEventTarget { /* tag, attributes, control state */ }
```

`PageElement` stores `(EventName, PageEventHandler)`. WASM raw handlers continue
to accept `web_sys::Event`; native raw handlers accept `NativeEvent`. Remove
`DummyEvent` rather than retaining a misleading alias.

**Steps:**

1. Add core tests for `EventType` compatibility, known/custom `EventName`
   storage, native target snapshots, family payload data, shared
   `prevent_default`, and shared propagation flags. Observe failures against
   the old `DummyEvent` implementation.
2. Add the optional catalog dependency to the `page` feature and replace the
   hand-written `EventType` enum/string tables with catalog re-exports.
3. Implement owned base data, target snapshots, family payloads, and an
   `Arc`-backed dispatch state. `prevent_default` changes state only for
   cancelable events.
4. Change `.on()`/`.listener()` and all handler vectors to `EventName`. Public
   builder methods that currently receive `KnownEvent` take
   `impl Into<EventName>` so existing calls convert explicitly inside the
   method; a `From<KnownEvent>` implementation alone is not sufficient. The
   low-level string `.listener()` classifies catalog names as known and other
   names as custom instead of attempting to parse every string as `KnownEvent`.
5. Replace hydration's duplicate string mapping with catalog lookup while
   preserving custom names as `EventName::Custom`.
6. Remove every `DummyEvent` production, generated-token, public re-export, and
   fixture reference in the same task. Raw native consumers use `NativeEvent`
   directly until later tasks add exact typed wrappers. Migrate component test
   props to their own domain type or `()` so the branch compiles at this task
   boundary; do not leave a temporary `DummyEvent` alias.
7. Run `rg -n '\bDummyEvent\b' crates/reinhardt-core crates/reinhardt-pages`
   and require zero hits before the focused commands.
8. Run:

   ```bash
   cargo test -p reinhardt-core --no-default-features --features page types::page
   cargo check -p reinhardt-core --no-default-features --features page
   cargo check -p reinhardt-pages
   cargo fmt --all --check
   ```

9. Commit as `feat(pages): add cross-target raw event transport` with
   `Fixes #5563`.

## Task 3: Generate public event payloads and typed adapters

**Files:**

- Modify: `crates/reinhardt-pages/Cargo.toml`
- Modify: `crates/reinhardt-pages/src/lib.rs`
- Modify: `crates/reinhardt-pages/src/prelude.rs`
- Modify: `crates/reinhardt-pages/src/platform.rs`
- Modify: `crates/reinhardt-pages/src/platform/native.rs`
- Modify: `crates/reinhardt-pages/src/platform/wasm.rs`
- Create: `crates/reinhardt-pages/src/event.rs`
- Create: `crates/reinhardt-pages/src/event/payload.rs`
- Create: `crates/reinhardt-pages/src/event/target.rs`
- Create: `crates/reinhardt-pages/src/event/value.rs`
- Modify: `crates/reinhardt-pages/src/callback.rs`

**Public contract:**

```rust
pub trait EventPayload: Sized {
	const EVENT: KnownEvent;
	fn try_from_raw(event: platform::Event) -> Result<Self, EventConversionError>;
}

pub trait IntoTypedEventHandler<P> { /* sync Fn(P) and Callback<P, ()> only */ }
pub fn typed_event_handler<P, H>(handler: H) -> PageEventHandler
where
	H: IntoTypedEventHandler<P>;
pub fn typed_async_event_handler<P, H, Fut>(handler: H) -> PageEventHandler;
pub fn raw_event_handler<H>(handler: H) -> PageEventHandler;
pub fn raw_async_event_handler<H, Fut>(handler: H) -> PageEventHandler;

pub enum EventTargetError { /* approved structured variants */ }
pub struct EventTarget { /* cross-target owned snapshot */ }
pub struct EventFile { /* cross-target file metadata */ }
```

Generate a unique public wrapper for every catalog entry. Each wrapper exposes
the common API and only the capability methods assigned by the catalog. WASM
wrappers retain the raw `web_sys::Event` but snapshot `current_target` before an
async handler is spawned.

**Steps:**

1. Add tests for exact-name conversion, cross-name rejection, base event
   methods, target/current-target separation, error display, keyboard and
   pointer defaults, value/checked/selected-values/files access, and
   `Callback<Payload, ()>` conversion. Observe failures before adding the
   module.
2. Add catalog and required `web-sys` features to both the direct and
   `web-sys-full` feature lists. Include the standardized event interfaces and
   supporting types used by capability methods, including
   `MediaEncryptedEvent` and `PictureInPictureEvent`.
3. Implement common support values (`Modifiers`, `Point`, `MouseButton`,
   `MouseButtons`, `PointerKind`, `EventFile`) and target accessors with the
   exact same result types on native and WASM.
4. Generate all payload structs and exact event-name checks from the catalog
   expansion macro. Implement family conversions with primary and fallback
   interfaces; a misleading synthetic browser event must report conversion
   failure without invoking the handler.
5. Keep sync, async, and raw adapters as distinct paths. The sealed synchronous
   `IntoTypedEventHandler<P>` follows the existing proven adapter pattern and
   has implementations only for `Fn(P)` and `Callback<P, ()>`; it has no
   `Fn()`, async, or raw blanket implementation. `typed_async_event_handler`
   accepts `Fn(P) -> Fut`. Codegen wraps a syntactically zero-argument closure
   as `move |_event: P| handler()` rather than adding a competing `Fn()` blanket
   implementation. Do not add generic typed custom detail.
6. Keep the Task 2 `DummyEvent` removal intact and re-export only common typed
   event support types through the prelude.
7. Run:

   ```bash
   cargo test -p reinhardt-pages --lib event
   cargo test -p reinhardt-pages --lib callback
   cargo check -p reinhardt-pages
   cargo check -p reinhardt-pages --target wasm32-unknown-unknown
   cargo fmt --all --check
   ```

8. Commit as `feat(pages): add typed event payload API` with `Fixes #5563`.

## Task 4: Split event AST semantics and lower intrinsic typed handlers

**Files:**

- Modify: `crates/reinhardt-manouche/Cargo.toml`
- Modify: `crates/reinhardt-manouche/src/core/node.rs`
- Modify: `crates/reinhardt-manouche/src/core/typed_node.rs`
- Modify: `crates/reinhardt-manouche/src/parser/page.rs`
- Modify: `crates/reinhardt-manouche/src/parser/page/component_brace.rs`
- Modify: `crates/reinhardt-manouche/src/validator/page.rs`
- Modify: `crates/reinhardt-pages/macros/Cargo.toml`
- Modify: `crates/reinhardt-pages/macros/src/page/validator.rs`
- Modify: `crates/reinhardt-pages/macros/src/page/codegen.rs`
- Modify: `crates/reinhardt-pages/macros/src/lib.rs`
- Modify: `crates/reinhardt-pages/tests/ui/page/pass/component_brace_with_event.rs`
- Modify: `crates/reinhardt-pages/tests/ui/page/pass/all_event_types.rs`
- Modify: `crates/reinhardt-pages/tests/ui/page/pass/external_handlers.rs`
- Create: `crates/reinhardt-pages/tests/ui/page/pass/custom_raw_event.rs`
- Create: `crates/reinhardt-pages/tests/ui/page/pass/typed_event_handlers.rs`
- Create: `crates/reinhardt-pages/tests/ui/page/fail/custom_event_invalid_syntax.rs`
- Create: `crates/reinhardt-pages/tests/ui/page/fail/mismatched_event_payload.rs`
- Create corresponding `.stderr` UI fixture files after validating diagnostics.

**AST contract:**

```rust
pub enum IntrinsicEvent {
	Standard { event: KnownEvent, handler: Expr },
	Custom { name: LitStr, handler: Expr },
}

pub struct ComponentEventProp {
	pub name: Ident,
	pub handler: Expr,
}
```

**Steps:**

1. Add parser/validator tests proving known intrinsic events resolve through
   the catalog, unknown names suggest nearby standard events,
   `@custom("name")` parses only on intrinsic elements, mixed-case SVG timing
   names remain exact, and component events retain an identifier and declared
   prop type. Observe focused test failures.
2. Add the leaf dependency and split untyped and typed AST nodes. Do not edit
   the inactive duplicate source files in `reinhardt-pages-ast`.
3. Parse intrinsic standard/custom syntax separately from component event prop
   syntax. Make both the canonical Manouche validator and the proc macro's
   current duplicated validator consume catalog APIs.
4. Expand UI fixtures to compile every catalog event with inferred payloads and
   representative explicit sync, async, external, zero-argument, and
   `Callback<Payload, ()>` handlers. Add a compile-fail case that supplies a
   payload from a different event name.
5. Remove the hand-written event-name match in codegen. Obtain the payload name
   from `EventSpec`, emit an absolute `reinhardt_pages::event` path, lower
   standard events through typed adapters, and lower custom events through raw
   adapters. Generate `.on(...)` on native targets instead of discarding
   intrinsic handlers. Leave component builder lowering unchanged.
6. Preserve span-local errors for invalid custom syntax and exact unknown-event
   suggestions. Update macro API documentation to show typed signatures and
   catalog-driven coverage without adding another event list.
7. Run:

   ```bash
   cargo test -p reinhardt-manouche parser::page
   cargo test -p reinhardt-manouche validator::page
   cargo test -p reinhardt-pages-ast
   cargo test -p reinhardt-pages-macros page::codegen
   cargo test -p reinhardt-pages --test ui test_page_macro_pass -- --exact
   cargo test -p reinhardt-pages --test ui test_page_macro_fail -- --exact
   cargo check -p reinhardt-pages --target wasm32-unknown-unknown
   cargo fmt --all --check
   ```

8. Commit as `feat(pages): lower intrinsic events to typed payloads` with
   `Fixes #5563`.

## Task 5: Dispatch real typed events in the native component harness

**Files:**

- Create: `crates/reinhardt-pages/src/testing/component/fixture.rs`
- Modify: `crates/reinhardt-pages/src/testing/component.rs`
- Modify: `crates/reinhardt-pages/src/testing/component/error.rs`
- Modify: `crates/reinhardt-pages/src/testing/component/events.rs`
- Modify: `crates/reinhardt-pages/src/testing/component/tree.rs`
- Modify: `crates/reinhardt-pages/src/testing/component/tests.rs`
- Modify: `crates/reinhardt-pages/tests/component_testing.rs`
- Modify: `crates/reinhardt-pages/tests/component_system_integration.rs`

**Public contract:**

```rust
pub struct EventFixture { /* validated builder */ }
pub enum EventFixtureError { /* family/field/name mismatch */ }

impl ElementHandle {
	pub fn dispatch(&self, fixture: EventFixture) -> Result<(), EventError>;
	pub fn change_checked(&self, value: bool);
	pub fn try_change_checked(&self, value: bool) -> Result<(), EventError>;
	pub fn key_down(&self, key: impl Into<String>);
	pub fn try_key_down(&self, key: impl Into<String>) -> Result<(), EventError>;
}
```

Keep `click`, `submit`, `input`, and `change` source-compatible.
`EventError::InvalidFixture` wraps `EventFixtureError` as its source; the
fallible `dispatch` method uses that variant while convenience methods retain
the existing panic-wrapper plus `try_*` convention.

**Steps:**

1. Add failing tests for click type/name, input value ordering, checked state,
   key/modifier fields, pointer data, custom raw dispatch, catalog-family
   mismatch, missing handler, and unsupported target state.
2. Add integration tests for a descendant target with an ancestor listener,
   non-bubbling events, `stop_propagation`, `stop_immediate_propagation`,
   `prevent_default`, typed async handlers, rerendering, and deterministic
   `Screen::settle()`.
3. Extend `ElementNode` with owned value, checked, selected-values, file, and
   contenteditable snapshots. Return `(listener_node_id, handler)` from event
   lookup and construct a distinct current-target event for each listener.
4. Implement deterministic family defaults, infallible builder setters, and
   validation in `build`/`dispatch`. Apply catalog bubbling rules and shared
   propagation state.
5. Replace all harness `DummyEvent` uses and migrate component test props to
   their declared domain type, `()`, or an explicit payload as appropriate.
6. Run:

   ```bash
   cargo test -p reinhardt-pages --lib --features testing testing::component::tests
   cargo test -p reinhardt-pages --test component_testing --features testing
   cargo test -p reinhardt-pages --test component_system_integration --features testing
   cargo clippy -p reinhardt-pages --tests --features testing -- -D warnings
   cargo fmt --all --check
   ```

7. Commit as `feat(pages): add native typed event fixtures` with `Fixes #5563`.

## Task 6: Migrate callbacks, actions, and forms to typed extraction

**Files:**

- Modify: `crates/reinhardt-pages/src/reactive/hooks/memo.rs`
- Modify: `crates/reinhardt-pages/src/reactive/hooks/async_action.rs`
- Modify: `crates/reinhardt-pages/macros/src/form/codegen.rs`
- Modify: `crates/reinhardt-pages/src/form/component.rs`
- Modify: relevant form and hook tests adjacent to these modules
- Modify: `crates/reinhardt-pages/tests/ui/form/pass/` fixtures selected by the
  existing UI harness

**Steps:**

1. Add failing unit/UI tests for `use_callback` and `Action::dispatching*` with
   typed event arguments, plus form input, checked control, textarea, select,
   multi-select, file, and submit extraction.
2. Make hook callback arguments generic without weakening existing callback
   dependency tracking or native task scheduling.
3. Replace framework-owned event-target `dyn_into`/`expect` paths with the
   typed capability APIs. Use `value`, `checked`, `selected_values`, and `files`
   as appropriate and report `EventTargetError` with field context.
4. Remove unreachable stale event-extraction code in `form/component.rs` if its
   module gating proves it cannot be built. Do not expand existing
   `Closure::forget` workarounds or add another manual-lifetime path.
5. Run:

   ```bash
   cargo test -p reinhardt-pages --lib reactive::hooks
   cargo test -p reinhardt-pages-macros form
   cargo test -p reinhardt-pages --test ui test_form_macro_pass -- --exact
   cargo check -p reinhardt-pages --target wasm32-unknown-unknown --features web-sys-full
   cargo fmt --all --check
   ```

6. Commit as `refactor(pages): use typed event extraction in forms` with
   `Fixes #5563`.

## Task 7: Verify browser behavior and catalog parity

**Files:**

- Create: `crates/reinhardt-pages/tests/typed_event_payload_wasm_test.rs`
- Modify: `crates/reinhardt-pages/Cargo.toml`
- Modify: catalog, wrapper, macro, or test files only when a parity test exposes
  a concrete gap

**Steps:**

1. Add WASM tests that dispatch real browser events and verify nested
   target/current-target separation, async current-target snapshots, raw
   access, primary/fallback interfaces, value/checked/select/file extraction,
   propagation, and one representative from each interface family.
2. Add parity tests asserting every `EVENT_SPECS` item has one wrapper, one
   macro lowering, and valid native fixture construction. The test must consume
   catalog expansion rather than introduce an expected-name list.
3. Run the native parity tests, then run browser tests when Chrome and
   `wasm-pack` are available:

   ```bash
   cargo test -p reinhardt-event-catalog
   cargo test -p reinhardt-pages --lib event
   cargo check -p reinhardt-pages --target wasm32-unknown-unknown --features web-sys-full
   wasm-pack test --headless --chrome crates/reinhardt-pages -- --test typed_event_payload_wasm_test
   cargo fmt --all --check
   ```

4. If the browser runner is unavailable, record the exact environmental reason
   in the PR test section; do not claim the browser test passed.
5. Commit as `test(pages): cover typed event payload parity` with `Fixes #5563`.

## Task 8: Document the breaking event API and complete verification

**Files:**

- Modify: `crates/reinhardt-pages/README.md`
- Modify: `crates/reinhardt-pages/src/lib.rs`
- Modify: `crates/reinhardt-pages/src/platform.rs`
- Modify: `crates/reinhardt-pages/src/testing.rs`
- Modify: `crates/reinhardt-pages/docs/react_to_reinhardt.md`
- Create: `crates/reinhardt-pages/docs/native_component_testing.md`
- Create: `instructions/MIGRATION_0.4.md`
- Modify other directly affected crate API docs found by `rg` for
  `DummyEvent`, raw event annotations, or native handlers described as ignored

**Steps:**

1. Update examples to use inferred or exact typed payloads, document
   `current_target` convenience semantics, raw custom syntax, target errors,
   component prop separation, native fixtures, and the #5636 deferral.
2. Add migration guidance for explicit raw handlers, external functions,
   callbacks, `DummyEvent`, component props, and browser-only `raw()` use.
3. Search for stale documentation and code references:

   ```bash
   rg -n "DummyEvent|platform::Event|ignored on non-WASM|ignored on native|EventType::" \
     crates/reinhardt-pages crates/reinhardt-core instructions
   ```

   Classify every hit as intentional low-level compatibility or migrate it.
4. Run focused and broad verification, fixing only regressions introduced by
   this branch:

   ```bash
   cargo test -p reinhardt-event-catalog
   cargo test -p reinhardt-core --no-default-features --features page
   cargo test -p reinhardt-manouche
   cargo test -p reinhardt-pages-macros
   cargo test -p reinhardt-pages --features testing
   cargo check -p reinhardt-pages --target wasm32-unknown-unknown --features web-sys-full
   RUSTDOCFLAGS="-D warnings" cargo doc -p reinhardt-pages -p reinhardt-pages-macros --no-deps
   cargo make fmt-check
   cargo make clippy-check
   cargo make placeholder-check
   cargo make clippy-todo-check
   ```

5. Inspect `git diff --check`, the full branch diff against
   `origin/develop/0.4.0`, and all test output. Request a final multi-dimensional
   code review and resolve every actionable finding.
6. Commit as `docs(pages): document typed event payload migration` with
   `Fixes #5563`.

## Task 9: Publish the branch and open the Draft PR

**Files:**

- Read: `.github/PULL_REQUEST_TEMPLATE.md`
- No source edits unless final verification reveals a scoped regression

**Steps:**

1. Confirm the worktree is clean, commits are small and conventional, no
   protected branch is checked out, and the diff contains only #5563 plus its
   approved documentation.
2. Push `fix/issue-5563-typed-event-payloads` without force.
3. Create a Draft PR targeting `develop/0.4.0` with a conventional title, the
   repository PR template, a `Fixes #5563` closing line, a `Refs #5636` line,
   exact verification results, and the breaking-change label.
4. Read back the PR base/head/title/body/labels/check state and report the URL.
   Do not post a separate PR or issue comment.
