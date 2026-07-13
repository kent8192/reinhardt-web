# Typed `page!` Event Payloads

## Status

- Issue: [#5563](https://github.com/kent8192/reinhardt-web/issues/5563)
- Target branch: `develop/0.4.0`
- Change class: breaking frontend API change
- Follow-up: [#5636](https://github.com/kent8192/reinhardt-web/issues/5636) for typed custom-event details

## Summary

`page!` handlers on intrinsic HTML and SVG elements will receive a distinct
payload type for each standard event name. For example, `@click` receives
`ClickEvent`, `@keydown` receives `KeyDownEvent`, and `@input` receives
`InputEvent`. The macro selects the payload from a central event catalog and
adapts it to the raw event transport used by rendering and hydration.

The same public payload surface will exist on browser-WASM and native targets.
Native payloads will carry real fixture data so component tests can exercise
event-dependent handler logic without a browser.

## Goals

1. Make handlers for different event names distinct in the Rust type system.
2. Remove application-level `web_sys` casts for common event properties and
   control values.
3. Preserve the runtime's heterogeneous raw-handler storage model.
4. Provide one authoritative catalog for parsing, code generation, hydration,
   public payload names, and native test dispatch.
5. Support all standardized events whose target can be an HTML or SVG element.
6. Reject misspelled standard event names at compile time with an actionable
   diagnostic.
7. Keep arbitrary DOM events expressible through an explicit raw escape hatch.

## Non-goals

- Component event props are not assigned DOM payload types. Their types remain
  defined by the component's props.
- Events specific to non-element targets such as `WebSocket`, `Worker`,
  `IDBRequest`, gamepads, and device sensors are not added to `page!`.
- Generic deserialization of `CustomEvent.detail` is deferred to #5636.
- Event delegation, capture modifiers, and listener option syntax are unchanged.
- The low-level DOM and HTML builder APIs are not converted to an end-to-end
  generic handler storage model.

## Design Decisions

### One payload type per event name

Every known event name has a distinct wrapper even when multiple events share
the same browser interface:

| DSL event | Public payload | Browser interface family |
| --- | --- | --- |
| `@click` | `ClickEvent` | `PointerEvent` with mouse fallback |
| `@mousedown` | `MouseDownEvent` | `MouseEvent` |
| `@keydown` | `KeyDownEvent` | `KeyboardEvent` |
| `@input` | `InputEvent` | `InputEvent` or base `Event` depending on control |
| `@submit` | `SubmitEvent` | `SubmitEvent` |
| `@animationend` | `AnimationEndEvent` | `AnimationEvent` |

The catalog stores the payload name explicitly. Names are not synthesized by a
case-conversion heuristic because DOM names such as `dblclick`,
`gotpointercapture`, and `beforexrselect` have non-obvious word boundaries.

### Intrinsic elements and components have separate semantics

The parser may use the same token form in both contexts, but the typed AST does
not conflate them:

```rust
enum IntrinsicEvent {
	Standard(KnownEvent),
	Custom(LitStr),
}

struct ComponentEventProp {
	name: Ident,
	handler: Expr,
}
```

An intrinsic `button { @click: handler }` is validated against the DOM event
catalog and receives `ClickEvent`. A component `Card { @click: handler }`
continues to lower to its `on_click` builder property, whose declared props type
controls the callback signature. A component can opt into `ClickEvent` by
declaring that prop type, but the DSL does not impose it.

### `current_target` is the convenience-accessor target

Convenience methods read the element on which the handler was registered, not
the descendant that originally dispatched the event. This makes a handler on a
button stable when the user clicks a nested icon or text span.

The originating browser target remains available through `target()` or
`raw()`. Target-dependent convenience methods return
`Result<T, EventTargetError>` rather than panicking or inventing a default.

### Custom events use an explicit raw form

Unknown `@name` forms are compile errors. Arbitrary DOM event names use:

```rust
@custom("item-selected"): |event: Event| {
	consume_raw_event(event);
}
```

The generic `@custom::<T>(...)` form is intentionally absent and tracked by
#5636.

## Architecture

### Central catalog crate

Add a dependency-free leaf crate, `reinhardt-event-catalog`, shared by
`reinhardt-core`, `reinhardt-manouche`, `reinhardt-pages-macros`, and
`reinhardt-pages`. Keeping the catalog separate avoids making the host-side DSL
parser depend on the much larger runtime core crate.

The catalog owns:

```rust
pub struct EventSpec {
	pub kind: KnownEvent,
	pub dom_name: &'static str,
	pub payload_name: &'static str,
	pub primary_interface: EventInterface,
	pub fallback_interfaces: &'static [EventInterface],
	pub capabilities: &'static [EventCapability],
	pub behavior: EventBehavior,
}
```

- `KnownEvent` is the closed set of standard element events.
- `EventInterface` describes the browser/native data family, such as mouse,
  pointer, keyboard, input, focus, drag, animation, or generic.
- `primary_interface` is the preferred browser interface and
  `fallback_interfaces` records standardized compatibility paths such as
  `click` accepting `MouseEvent` and `input` accepting a base `Event`.
- `EventCapability` controls which inherent convenience methods are generated
  for the unique payload type.
- `EventBehavior` records the catalog defaults for `bubbles`, `cancelable`, and
  `composed`. Browser wrappers expose the actual browser flags; native fixture
  dispatch starts from these defaults and permits explicit overrides.
- A hidden catalog-expansion macro emits enum arms, payload declarations, and
  parity tables from the same declaration.
- `EventName` is also defined in this leaf crate so core, hydration, parser, and
  fixtures share one known-or-custom runtime name without introducing a
  dependency cycle.

`reinhardt-core::types::page::EventType` remains as a compatibility re-export
of `KnownEvent`. Runtime storage uses the catalog's `EventName`, which
distinguishes known and custom names:

```rust
pub enum EventName {
	Known(KnownEvent),
	Custom(Cow<'static, str>),
}
```

### Macro lowering

For an intrinsic standard event, code generation selects the payload wrapper
from `EventSpec` and creates a raw adapter:

```rust
PageElement::new("button").on(
	KnownEvent::Click,
	typed_event_handler::<ClickEvent, _>(handler),
)
```

`typed_event_handler` stores an `Arc<dyn Fn(RawEvent)>` and constructs the
event-specific wrapper at invocation. Inline closures, external functions,
`Callback<Payload, ()>`, and async closures use the same payload contract.
Separate synchronous and asynchronous adapter helpers preserve the current
automatic task-spawning behavior.

Zero-argument handlers and closures that ignore an inferred argument remain
source-compatible. An explicit raw parameter on a standard event must migrate
to the event-specific type and call `raw()` when needed.

### Raw transport

WASM continues to transport `web_sys::Event`. Event-specific wrappers retain
that raw event and expose the corresponding browser interface and target
capabilities.

Native replaces the empty `DummyEvent` transport with a data-carrying
`NativeEvent`. It lives in `reinhardt-core`, next to `PageEventHandler`, because
the raw handler transport is owned by core and core cannot depend on
`reinhardt-pages`. `reinhardt-pages::platform::Event` re-exports it. The
compatibility name `DummyEvent` is removed rather than kept as an alias because
it would misdescribe the new semantics.

```rust
pub struct NativeEvent {
	name: EventName,
	current_target: Option<NativeEventTarget>,
	target: Option<NativeEventTarget>,
	base: BaseEventData,
	payload: NativeEventPayload,
	dispatch_state: Arc<NativeDispatchState>,
}
```

`NativeEventPayload` is grouped by interface family, not duplicated for every
event name. The catalog verifies that each `KnownEvent` uses the expected
family. Event-specific public wrappers enforce the exact event name while
sharing the family data internally. `NativeDispatchState` shares
`default_prevented`, propagation-stopped, and immediate-propagation-stopped
flags across the event values constructed for each listener. It uses
thread-safe owned state so event values remain usable by the existing native
async handler path.

The native component tree returns `(listener_node_id, handler)` pairs instead
of discarding the listener owner. Dispatch preserves one originating `target`
snapshot and constructs a `NativeEvent` for each listener with that listener's
`current_target` snapshot. Catalog metadata decides whether the path may bubble.
`stop_propagation()` stops traversal before the next ancestor;
`stop_immediate_propagation()` also skips later handlers on the same node; and
`prevent_default()` updates the shared state only when the event is cancelable.

On WASM, the adapter snapshots `current_target` before calling or spawning the
user handler. Browsers clear `Event.currentTarget` after listener invocation,
so async payload methods read the snapshot while `raw()` still exposes the
original browser event.

## Public API

### Module layout

Public payload types live under `reinhardt_pages::event`:

```rust
use reinhardt_pages::event::{ClickEvent, InputEvent, KeyDownEvent};
```

The macro refers to these types with an absolute crate path, so inferred
handlers need no imports. The prelude does not glob-re-export the full catalog;
doing so would add dozens of collision-prone names. It re-exports only common
support types such as `EventTargetError`, `Modifiers`, and the raw cross-target
`Event` escape hatch.

### Common event surface

Every standard payload provides:

```rust
impl ClickEvent {
	pub fn raw(&self) -> &platform::Event;
	pub fn event_type(&self) -> &'static str;
	pub fn prevent_default(&self);
	pub fn stop_propagation(&self);
	pub fn stop_immediate_propagation(&self);
	pub fn default_prevented(&self) -> bool;
	pub fn target(&self) -> Option<EventTarget>;
	pub fn current_target(&self) -> Option<EventTarget>;
}
```

Cross-target `EventTarget` exposes only stable target information. Direct
`web_sys::EventTarget` access remains available through `raw()` on WASM.

### Capability methods

The catalog adds inherent methods according to the event interface and
capabilities. Representative surfaces are:

```rust
impl InputEvent {
	pub fn value(&self) -> Result<String, EventTargetError>;
	pub fn checked(&self) -> Result<bool, EventTargetError>;
	pub fn selected_values(&self) -> Result<Vec<String>, EventTargetError>;
	pub fn files(&self) -> Result<Vec<EventFile>, EventTargetError>;
	pub fn input_type(&self) -> Option<String>;
	pub fn data(&self) -> Option<String>;
}

impl KeyDownEvent {
	pub fn key(&self) -> String;
	pub fn code(&self) -> String;
	pub fn repeat(&self) -> bool;
	pub fn modifiers(&self) -> Modifiers;
}

impl PointerMoveEvent {
	pub fn client_position(&self) -> Point;
	pub fn button(&self) -> MouseButton;
	pub fn buttons(&self) -> MouseButtons;
	pub fn pointer_id(&self) -> i32;
	pub fn pointer_type(&self) -> PointerKind;
	pub fn pressure(&self) -> f32;
	pub fn modifiers(&self) -> Modifiers;
}
```

Properties intrinsic to a standard browser interface are infallible. The
browser dispatch contract supplies the matching interface, and native fixture
construction validates the required family data. Methods that inspect the
listener element remain fallible because event name alone cannot prove target
capabilities.

### Target errors

`EventTargetError` is non-exhaustive and carries structured context:

```rust
#[non_exhaustive]
pub enum EventTargetError {
	MissingCurrentTarget { event: &'static str },
	UnsupportedElement {
		event: &'static str,
		actual_tag: String,
		expected: &'static [&'static str],
	},
	UnsupportedProperty {
		event: &'static str,
		property: &'static str,
		actual_tag: String,
	},
}
```

Errors implement `Display` and `std::error::Error` and use identical variants
on WASM and native targets.

`EventFile` is a cross-target owned snapshot containing `name`, `media_type`,
`size`, and `last_modified`. Its WASM implementation additionally exposes the
source `web_sys::File` through a target-gated `raw()` method. This lets form
bindings use the same capability API without pretending a browser file object
exists on native targets.

## Event Catalog Boundary

The initial catalog is the union of:

1. standard event-handler names defined for HTML elements;
2. standardized event families whose target includes `Element`,
   `HTMLElement`, `SVGElement`, or a concrete HTML element;
3. the existing Reinhardt event names retained for compatibility.

The catalog excludes browser-specific names and deprecated mutation events.
`keypress` remains because it is already public, but its payload and docs are
marked deprecated.

### HTML element event handlers

`abort`, `auxclick`, `beforeinput`, `beforematch`, `beforetoggle`, `blur`,
`cancel`, `canplay`, `canplaythrough`, `change`, `click`, `close`, `command`,
`contextlost`, `contextmenu`, `contextrestored`, `copy`, `cuechange`, `cut`,
`dblclick`, `drag`, `dragend`, `dragenter`, `dragleave`, `dragover`,
`dragstart`, `drop`, `durationchange`, `emptied`, `ended`, `error`, `focus`,
`formdata`, `input`, `invalid`, `keydown`, `keypress`, `keyup`, `load`,
`loadeddata`, `loadedmetadata`, `loadstart`, `mousedown`, `mouseenter`,
`mouseleave`, `mousemove`, `mouseout`, `mouseover`, `mouseup`, `paste`,
`pause`, `play`, `playing`, `progress`, `ratechange`, `reset`, `resize`,
`scroll`, `scrollend`, `securitypolicyviolation`, `seeked`, `seeking`,
`select`, `slotchange`, `stalled`, `submit`, `suspend`, `timeupdate`, `toggle`,
`volumechange`, `waiting`, and `wheel`.

### Additional standardized element families

- Composition: `compositionstart`, `compositionupdate`, `compositionend`
- Focus: `focusin`, `focusout`
- Pointer: `pointerdown`, `pointerup`, `pointermove`, `pointerover`,
  `pointerenter`, `pointerout`, `pointerleave`, `pointercancel`,
  `gotpointercapture`, `lostpointercapture`, `pointerrawupdate`
- Touch: `touchstart`, `touchend`, `touchmove`, `touchcancel`
- CSS animation: `animationstart`, `animationend`, `animationiteration`,
  `animationcancel`
- CSS transition: `transitionrun`, `transitionstart`, `transitionend`,
  `transitioncancel`
- Fullscreen: `fullscreenchange`, `fullscreenerror`
- Selection: `selectionchange`, `selectstart`
- Scrolling and rendering: `scrollsnapchange`, `scrollsnapchanging`,
  `contentvisibilityautostatechange`
- Encrypted media: `encrypted`, `waitingforkey`
- Picture-in-picture: `enterpictureinpicture`, `leavepictureinpicture`
- WebXR element interception: `beforexrselect`
- SVG animation timing: `beginEvent`, `endEvent`, `repeatEvent`

Catalog additions after this baseline require a new `EventSpec` entry, public
payload documentation, WASM interface coverage, native fixture coverage, and a
catalog snapshot update.

## Data Flow

### Browser-WASM

1. `page!` parses an intrinsic event into `KnownEvent`.
2. Code generation selects the exact public payload type.
3. `PageElement` stores the generated raw adapter under `EventName`.
4. Mount or hydration attaches one browser listener using the DOM event name.
5. The browser passes `web_sys::Event` to the adapter.
6. The adapter constructs the event-specific wrapper and invokes the handler.
7. Convenience accessors use `current_target`; `raw()` exposes advanced
   browser interop.

### Native component testing

1. A test locates an `ElementHandle`.
2. A shortcut or explicit fixture updates target state before dispatch.
3. The fixture constructs a validated `NativeEvent` with the expected family.
4. Handler lookup retains each listener node and constructs a per-listener
   `current_target` while preserving the originating `target`.
5. The same raw adapter constructs the event-specific wrapper.
6. The handler executes normally, including state updates, propagation control,
   and spawned tasks.
7. `Screen::settle()` processes resulting rerenders and task work.

Intrinsic event code generation emits `.on(...)` on native targets as well as
WASM. Server rendering still serializes no event behavior, but the retained
handler storage is available to the native component harness instead of being
compiled away.

## Native Testing API

Existing high-frequency helpers remain concise:

```rust
screen.get_by_role(Role::Button, "Save").click();
screen.get_by_label("Name").input("Ada");
screen.get_by_label("Enabled").change_checked(true);
screen.get_by_label("Search").key_down("Enter");
```

The helpers update the node before dispatching the corresponding event. A
generic fixture API covers the full catalog without requiring one `Screen`
method per event:

```rust
element.dispatch(
	EventFixture::pointer_move()
		.client_position(120.0, 80.0)
		.pointer_kind(PointerKind::Pen)
		.pressure(0.5),
)?;
```

Fixture builders reject a field that is incompatible with the catalog's
interface family. Setters are infallible so fixtures remain convenient to
compose; `build()` and `dispatch()` validate the accumulated fields and return
`EventFixtureError`. Dispatch also rejects a fixture whose event name and
payload family do not match.

The fixture uses deterministic catalog-family defaults. Keyboard strings are
empty, `repeat` and modifiers are false, pointer ID and coordinates are zero,
pointer kind is mouse, pressure is zero, and button state is derived from the
event kind (`Primary` for click/down/up and no pressed button for movement).
Every field can be overridden explicitly.

The existing `click()`, `submit()`, `input(value)`, and `change(value)` helpers
remain source-compatible. `change_checked(value)` and `key_down(key)` are
additive shortcuts. `EventFixture::custom(name)` supports raw custom-event
dispatch with base flags and target data only; typed `detail` remains deferred
to #5636.

Native target snapshots model value, checked state, selected values, file
metadata, and text content. `value()` reads input, textarea, and select values,
or text content for a contenteditable element. `selected_values()` applies only
to select elements, `files()` only to file inputs, and `checked()` only to
checkbox and radio controls; other combinations return
`UnsupportedProperty`.

## Error Handling

- Unknown intrinsic `@event` names produce a compile error naming the invalid
  event and suggesting the nearest known names.
- Invalid `@custom` syntax produces a span-local parser diagnostic.
- Target capability mismatches return `EventTargetError` without panic.
- Native fixture/catalog mismatches return `EventFixtureError` before invoking
  the handler.
- Standard browser interface mismatches caused by manually dispatching an
  event object with a misleading type string are treated as an invalid
  synthetic event. The adapter logs the expected interface, actual event type,
  and listener target through the `reinhardt-pages` error logging macro; the
  handler is not invoked.
- User handler panics continue to propagate according to the existing runtime
  and test-harness behavior.

## Form Integration

Framework-owned form bindings use the same adapters and target accessors as
`page!` handlers. Direct `dyn_into::<HtmlInputElement>()` and associated
`expect(...)` calls are removed from form value, checked-state, select, and
submit paths. Multi-select extraction uses `selected_values()` and file input
extraction uses `files()`. Framework code handles `EventTargetError` explicitly
and reports the field name and expected target kind.

`use_callback` and `Action::dispatching*` accept generic callback argument
types so `Callback<Payload, ()>` participates in the same typed adapter path as
an inline closure. Native intrinsic code generation registers handlers instead
of discarding them, which allows the component harness to exercise both sync
and async typed handlers.

This alignment prevents the public DSL and internal form runtime from evolving
two different event extraction rules.

## Migration

Inferred handlers that ignore the event remain unchanged:

```rust
@click: |_| save()
```

Explicit and external handlers migrate to the event-specific payload:

```rust
// Before
fn update(event: reinhardt_pages::platform::Event) {
	// web_sys target cast
}

// After
fn update(event: reinhardt_pages::event::InputEvent) {
	match event.value() {
		Ok(value) => set_value(value),
		Err(error) => report(error),
	}
}
```

Code that needs the browser object calls `event.raw()`. A raw handler for an
arbitrary name uses `@custom("name")`; a known standard event never silently
falls back to the raw type.

Native code that explicitly named `DummyEvent` migrates according to its
boundary. Intrinsic handlers use the generated event-specific payload;
component event props retain their declared domain type and may use `()`, a
domain struct, or an explicitly chosen event payload. `Callback<DummyEvent,
()>` and the public `DummyEvent` re-export are removed.

## Testing Strategy

### Catalog tests

- every DOM name is unique;
- every payload type name is unique;
- every event has an interface family and valid capabilities;
- `KnownEvent::as_str` and `FromStr` round-trip the full snapshot;
- generated payload declarations and macro mappings cover the same snapshot;
- no non-standard or non-element target slips into the baseline list.

### Macro UI tests

- every catalog event compiles with an inferred handler;
- representative explicit payload annotations compile;
- a payload from a different event name fails to compile;
- unknown event names fail with suggestions;
- raw `@custom("...")` compiles;
- component event props retain their declared callback type;
- sync, async, external function, and `Callback` handlers use the same payload;
- zero- and one-argument arity rules remain intact.

### Platform tests

- WASM wrappers expose the expected base and capability methods;
- value and checked accessors cover input, textarea, select, checkbox, radio,
  contenteditable, missing target, and unsupported target cases;
- bubbling tests prove convenience methods use `current_target`;
- raw event access remains available;
- native wrappers match WASM return types and error variants.

### Native component tests

- click, input, change, submit, keyboard, pointer, clipboard, composition,
  animation, transition, media, and generic fixtures reach the correct handler;
- target state is updated before handler invocation;
- invalid fixture families fail before invocation;
- handler-triggered rerenders and tasks settle deterministically.

### Integration and documentation checks

- `form!` bindings use typed extraction without cast panics;
- relevant examples and crate documentation show typed handlers;
- migration documentation covers explicit raw annotations and callbacks;
- `cargo doc --no-deps`, targeted native tests, UI tests, and WASM tests pass;
- formatting, clippy, placeholder, and TODO checks remain clean.

## Documentation Impact

Update the following in the implementation workflow:

- `crates/reinhardt-pages/README.md`
- `crates/reinhardt-pages/src/lib.rs`
- `crates/reinhardt-pages/src/platform.rs` and event module API docs
- `crates/reinhardt-pages/macros/src/lib.rs`
- React-to-Reinhardt event-handler guidance
- native component testing documentation
- breaking migration guidance for the `develop/0.4.0` line

Documentation will describe the event contract and technical migration only;
it will not record conversational or tool-specific context.

## Implementation Boundaries

The implementation should be split into independently verifiable units:

1. central catalog and runtime event names;
2. cross-target raw/native event transport;
3. generated event-specific public wrappers;
4. parser, validation, and macro adapters;
5. native component fixture dispatch;
6. form runtime migration;
7. documentation, migration examples, and full verification.

The central catalog must land before consumers so each later unit can compile
against one authoritative mapping. No unit may introduce a second handwritten
event-name-to-payload table.

The catalog remains a dependency-free metadata leaf: it may export declarative
expansion macros, but it does not depend on parser/token crates or contain
platform wrapper implementations. `reinhardt-pages-ast` continues to re-export
the canonical Manouche AST and does not gain a duplicate catalog integration.
Until the duplicated proc-macro validator is removed, both validation entry
points must consume the same catalog APIs and parity tests must prevent their
diagnostics from drifting.
